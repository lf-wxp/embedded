//! BLE module
//!
//! Wraps nrf-softdevice BLE operations, providing a clean API for:
//! - Enabling/disabling SoftDevice
//! - Configuring and registering GATT Server (NUS + optional Battery Service)
//! - Controlling BLE advertising (start/stop)
//! - Managing BLE connection + binary protocol message dispatch

pub mod battery;
pub mod buttons;
pub mod commands;
pub mod led_matrix;
pub mod nus;

use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{Either3, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use nrf_softdevice::Softdevice;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::raw;

use battery::{BatteryService, BatteryServiceEvent};
use nus::{NusService, NusServiceEvent};

/// BLE advertising control signal
static ADVERTISING_ENABLED: AtomicBool = AtomicBool::new(true);

/// BLE disconnect signal (for actively disconnecting the current connection)
static DISCONNECT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Combined GATT Server: NUS (primary) + Battery (optional)
#[nrf_softdevice::gatt_server]
pub struct GattServer {
  pub nus: NusService,
  pub battery: BatteryService,
}

/// BLE configuration parameters
pub struct BleConfig {
  /// Device name (shown in Bluetooth scan list)
  pub device_name: &'static [u8],
  /// Maximum number of simultaneous connections
  pub max_connections: u8,
  /// ATT MTU size
  pub att_mtu: u16,
}

impl Default for BleConfig {
  fn default() -> Self {
    Self {
      device_name: b"MicroBit-BLE",
      max_connections: 1,
      att_mtu: 64,
    }
  }
}

/// BLE controller
pub struct BleController {
  sd: &'static Softdevice,
  server: GattServer,
}

impl BleController {
  /// Enable SoftDevice and initialize BLE controller
  pub fn enable(spawner: &Spawner, config: &BleConfig) -> Self {
    let sd_config = nrf_softdevice::Config {
      clock: Some(raw::nrf_clock_lf_cfg_t {
        // micro:bit V2 uses internal RC oscillator as low-frequency clock source
        source: raw::NRF_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: 16,
        rc_temp_ctiv: 2,
        accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
      }),
      conn_gap: Some(raw::ble_gap_conn_cfg_t {
        conn_count: config.max_connections,
        event_length: 24,
      }),
      conn_gatt: Some(raw::ble_gatt_conn_cfg_t {
        att_mtu: config.att_mtu,
      }),
      gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
        attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
      }),
      gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
        adv_set_count: 1,
        periph_role_count: 1,
      }),
      gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
        p_value: config.device_name.as_ptr() as *mut u8,
        current_len: config.device_name.len() as u16,
        max_len: config.device_name.len() as u16,
        write_perm: unsafe { mem::zeroed() },
        _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
          raw::BLE_GATTS_VLOC_STACK as u8,
        ),
      }),
      ..Default::default()
    };

    info!("Enabling SoftDevice...");
    let sd = Softdevice::enable(&sd_config);
    info!("SoftDevice enabled");

    let server = GattServer::new(sd).expect("GATT Server registration failed");
    info!("GATT Server registered (NUS + Battery)");

    spawner.spawn(softdevice_task(sd).expect("softdevice_task spawn failed"));
    info!("SoftDevice background task started");

    Self { sd, server }
  }

  pub fn softdevice(&self) -> &'static Softdevice {
    self.sd
  }

  pub fn server(&self) -> &GattServer {
    &self.server
  }

  /// Start BLE advertising
  pub fn start_advertising(&self) {
    ADVERTISING_ENABLED.store(true, Ordering::SeqCst);
    info!("BLE advertising started");
  }

  /// Stop BLE advertising
  pub fn stop_advertising(&self) {
    ADVERTISING_ENABLED.store(false, Ordering::SeqCst);
    info!("BLE advertising stopped");
  }

  pub fn is_advertising(&self) -> bool {
    ADVERTISING_ENABLED.load(Ordering::SeqCst)
  }

  pub fn disconnect(&self) {
    DISCONNECT_SIGNAL.signal(());
    info!("Disconnect signal sent");
  }

  /// Main loop: advertise -> handle GATT events -> disconnect -> re-advertise
  pub async fn run(&self, adv_data: &[u8], scan_data: &[u8]) -> ! {
    loop {
      if !ADVERTISING_ENABLED.load(Ordering::SeqCst) {
        embassy_time::Timer::after_millis(100).await;
        continue;
      }

      let adv_config = peripheral::Config {
        interval: 160, // 100ms
        ..peripheral::Config::default()
      };
      let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data,
        scan_data,
      };

      info!("Advertising, waiting for connection...");

      let conn = match peripheral::advertise_connectable(self.sd, adv, &adv_config).await {
        Ok(c) => c,
        Err(_) => {
          warn!("Advertising error, retrying in 1 second");
          embassy_time::Timer::after_millis(1000).await;
          continue;
        }
      };

      info!("BLE device connected!");
      // Clear subscription state (reset on new connection)
      buttons::set_subscribed(false);

      // GATT event loop
      let nus_ref = &self.server.nus;
      let sd_ref: &'static Softdevice = self.sd;
      let conn_ref = &conn;

      let gatt_future = gatt_server::run(conn_ref, &self.server, |event| match event {
        GattServerEvent::Nus(e) => {
          if let Some(rx) = NusService::handle_event(e) {
            // Received a frame via NUS, parse and execute
            if let Some(resp) = commands::handle_rx(sd_ref, rx.as_slice()) {
              let _ = nus_ref.send(conn_ref, resp.as_slice());
            }
          }
        }
        GattServerEvent::Battery(e) => match e {
          BatteryServiceEvent::LevelCccdWrite { notifications: _ } => {
            BatteryService::handle_event(e);
          }
        },
      });

      // Button event forwarding: take from BUTTON_EVENTS channel, encode and send via NUS
      let button_forward = async {
        loop {
          let evt = buttons::BUTTON_EVENTS.receive().await;
          if !buttons::is_subscribed() {
            continue;
          }
          if let Some(resp) = commands::encode_button_event(evt) {
            let _ = nus_ref.send(conn_ref, resp.as_slice());
          }
        }
      };

      let disconnect_future = DISCONNECT_SIGNAL.wait();

      match select3(gatt_future, button_forward, disconnect_future).await {
        Either3::First(_) => info!("Connection disconnected, re-advertising..."),
        Either3::Second(_) => {
          info!("Button forwarding task ended (should not happen), re-advertising...")
        }
        Either3::Third(_) => info!("Active disconnect, re-advertising..."),
      }
    }
  }
}

/// SoftDevice run task
#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
  sd.run().await
}
