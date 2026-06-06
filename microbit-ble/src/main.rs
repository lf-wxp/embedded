//! micro:bit V2 BLE peripheral advertising example (NUS + LED matrix + buttons + temperature)
//!
//! Bidirectional communication with browser (Web Bluetooth) / nRF Connect via Nordic UART Service.
//!
//! Features demonstrated:
//! - Control onboard 5x5 LED matrix (bitmap / character)
//! - Report onboard button A/B events
//! - Read on-chip temperature
//! - Protocol frame echo (connectivity test)
//!
//! See [`microbit_ble_protocol`] module for protocol definition.

#![no_std]
#![no_main]
#![allow(dead_code)]

mod ble;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::interrupt::Priority;
use nrf_softdevice::ble::advertisement_builder::{
  Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList,
};
use panic_probe as _;

use ble::buttons::{ButtonPins, button_task};
use ble::led_matrix::{LedPins, led_matrix_task};
use ble::{BleConfig, BleController};

/// BLE device name (shown in Bluetooth scan list)
const DEVICE_NAME: &[u8] = b"MicroBit-BLE";

/// Nordic UART Service 128-bit UUID (little-endian, for advertising packet)
const NUS_UUID_BYTES: [u8; 16] = [
  0x9E, 0xCA, 0xDC, 0x24, 0x0E, 0xE5, 0xA9, 0xE0, 0x93, 0xF3, 0xA3, 0xB5, 0x01, 0x00, 0x40, 0x6E,
];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
  info!("=== micro:bit V2 BLE (NUS) example started ===");

  // ========================================
  // 1. Initialize embassy-nrf peripherals
  //    GPIOTE priority must be lower than P0/P1/P4 used by SoftDevice
  // ========================================
  let mut nrf_config = embassy_nrf::config::Config::default();
  nrf_config.gpiote_interrupt_priority = Priority::P2;
  nrf_config.time_interrupt_priority = Priority::P2;
  let p = embassy_nrf::init(nrf_config);

  // ========================================
  // 2. Start LED matrix refresh task
  // ========================================
  spawner.spawn(
    led_matrix_task(LedPins {
      row1: p.P0_21,
      row2: p.P0_22,
      row3: p.P0_15,
      row4: p.P0_24,
      row5: p.P0_19,
      col1: p.P0_28,
      col2: p.P0_11,
      col3: p.P0_31,
      col4: p.P1_05,
      col5: p.P0_30,
    })
    .expect("led_matrix_task spawn failed"),
  );

  // Show an icon on startup to indicate firmware is running
  ble::led_matrix::show_char(b'B');

  // ========================================
  // 3. Start button monitoring task
  // ========================================
  spawner.spawn(
    button_task(ButtonPins {
      btn_a: p.P0_14,
      btn_b: p.P0_23,
    })
    .expect("button_task spawn failed"),
  );

  // ========================================
  // 4. Initialize BLE controller (enable SoftDevice)
  // ========================================
  let config = BleConfig {
    device_name: DEVICE_NAME,
    ..Default::default()
  };
  let ble = BleController::enable(&spawner, &config);
  info!(
    "BLE initialized, device name: {}",
    core::str::from_utf8(DEVICE_NAME).unwrap_or("?")
  );

  // ========================================
  // 5. Build advertising data
  // ========================================
  // Web Bluetooth requires the service UUID to appear in the advertising packet (adv_data) for filter matching.
  // 128-bit UUID takes 18 bytes, Flags take 3 bytes, leaving 10 bytes for the short device name.
  // Short name "MicroBit" is a prefix of the full device name "MicroBit-BLE", compliant with BLE spec.
  // Total advertising packet size: 3 (Flags) + 18 (UUID) + 10 (short name) = 31 bytes, exactly full.
  static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
    .services_128(ServiceList::Complete, &[NUS_UUID_BYTES])
    .short_name("MicroBit") // Prefix of full device name, fits in advertising packet
    .build();

  // Scan response packet: full device name (received after active scan by central device)
  static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .full_name("MicroBit-BLE")
    .build();

  // ========================================
  // 6. Start advertising and run main loop
  // ========================================
  ble.start_advertising();
  info!("BLE advertising started, waiting for browser/nRF Connect to connect...");
  ble.run(&ADV_DATA, &SCAN_DATA).await;
}
