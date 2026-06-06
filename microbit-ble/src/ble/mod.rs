//! BLE 蓝牙模块
//!
//! 封装 nrf-softdevice 的蓝牙操作，提供简洁的 API 用于：
//! - 启用/禁用 SoftDevice
//! - 配置和注册 GATT Server（NUS + 可选 Battery Service）
//! - 控制 BLE 广播（开启/停止）
//! - 管理 BLE 连接 + 二进制协议消息分发

pub mod battery;
pub mod buttons;
pub mod commands;
pub mod led_matrix;
pub mod nus;
pub mod protocol;

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

/// BLE 广播控制信号
static ADVERTISING_ENABLED: AtomicBool = AtomicBool::new(true);

/// BLE 断开连接信号（用于主动断开当前连接）
static DISCONNECT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// 整体 GATT Server：NUS（主） + Battery（可选）
#[nrf_softdevice::gatt_server]
pub struct GattServer {
  pub nus: NusService,
  pub battery: BatteryService,
}

/// BLE 配置参数
pub struct BleConfig {
  /// 设备名称（显示在蓝牙扫描列表中）
  pub device_name: &'static [u8],
  /// 最大同时连接数
  pub max_connections: u8,
  /// ATT MTU 大小
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

/// BLE 控制器
pub struct BleController {
  sd: &'static Softdevice,
  server: GattServer,
}

impl BleController {
  /// 启用 SoftDevice 并初始化 BLE 控制器
  pub fn enable(spawner: &Spawner, config: &BleConfig) -> Self {
    let sd_config = nrf_softdevice::Config {
      clock: Some(raw::nrf_clock_lf_cfg_t {
        // micro:bit V2 使用内部 RC 振荡器作为低频时钟源
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

    info!("正在启用 SoftDevice...");
    let sd = Softdevice::enable(&sd_config);
    info!("SoftDevice 已启用");

    let server = GattServer::new(sd).expect("GATT Server 注册失败");
    info!("GATT Server 已注册（NUS + Battery）");

    spawner.spawn(softdevice_task(sd).expect("softdevice_task spawn 失败"));
    info!("SoftDevice 后台任务已启动");

    Self { sd, server }
  }

  pub fn softdevice(&self) -> &'static Softdevice {
    self.sd
  }

  pub fn server(&self) -> &GattServer {
    &self.server
  }

  /// 开启 BLE 广播
  pub fn start_advertising(&self) {
    ADVERTISING_ENABLED.store(true, Ordering::SeqCst);
    info!("BLE 广播已开启");
  }

  /// 停止 BLE 广播
  pub fn stop_advertising(&self) {
    ADVERTISING_ENABLED.store(false, Ordering::SeqCst);
    info!("BLE 广播已停止");
  }

  pub fn is_advertising(&self) -> bool {
    ADVERTISING_ENABLED.load(Ordering::SeqCst)
  }

  pub fn disconnect(&self) {
    DISCONNECT_SIGNAL.signal(());
    info!("已发送断开连接信号");
  }

  /// 主循环：广播 -> 处理 GATT 事件 -> 断开 -> 重新广播
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

      info!("正在广播中，等待连接...");

      let conn = match peripheral::advertise_connectable(self.sd, adv, &adv_config).await {
        Ok(c) => c,
        Err(_) => {
          warn!("广播错误，1 秒后重试");
          embassy_time::Timer::after_millis(1000).await;
          continue;
        }
      };

      info!("BLE 设备已连接!");
      // 取消订阅状态（新连接重置）
      buttons::set_subscribed(false);

      // GATT 事件循环
      let nus_ref = &self.server.nus;
      let sd_ref: &'static Softdevice = self.sd;
      let conn_ref = &conn;

      let gatt_future = gatt_server::run(conn_ref, &self.server, |event| match event {
        GattServerEvent::Nus(e) => {
          if let Some(rx) = NusService::handle_event(e) {
            // 通过 NUS 收到一帧数据，解析并执行
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

      // 按钮事件转发：从 BUTTON_EVENTS channel 取出，编码后通过 NUS 发送
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
        Either3::First(_) => info!("连接已断开，重新广播..."),
        Either3::Second(_) => info!("按钮转发任务结束（不应发生），重新广播..."),
        Either3::Third(_) => info!("主动断开连接，重新广播..."),
      }
    }
  }
}

/// SoftDevice 运行任务
#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
  sd.run().await
}
