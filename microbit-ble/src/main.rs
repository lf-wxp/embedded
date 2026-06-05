//! micro:bit V2 BLE 外设广播示例
//!
//! 本示例演示如何使用 nrf-softdevice 在 micro:bit V2 上启用蓝牙 BLE，
//! 创建一个 GATT Server 并进行 BLE 外设广播。
//! 手机可以通过 nRF Connect 等 BLE 调试工具扫描并连接到该设备。

#![no_std]
#![no_main]
#![allow(dead_code)]

mod ble;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf as _;
use nrf_softdevice::raw;
use panic_probe as _;

use ble::{BleConfig, BleController};

/// BLE 设备名称（显示在手机的蓝牙扫描列表中）
const DEVICE_NAME: &[u8] = b"MicroBit-BLE";

/// 主入口点
#[embassy_executor::main]
async fn main(spawner: Spawner) {
  info!("=== micro:bit V2 BLE 示例启动 ===");

  // ========================================
  // 1. 初始化 BLE 控制器
  // ========================================
  let config = BleConfig {
    device_name: DEVICE_NAME,
    ..Default::default()
  };

  let ble = BleController::enable(&spawner, &config);
  info!(
    "BLE 已初始化，设备名: {}",
    core::str::from_utf8(DEVICE_NAME).unwrap_or("?")
  );

  // ========================================
  // 2. 构建广播数据
  // ========================================
  #[rustfmt::skip]
    let adv_data = &[
        // Flags: LE General Discoverable + BR/EDR Not Supported
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        // 完整设备名称
        (1 + DEVICE_NAME.len()) as u8,
        0x09, // AD Type: Complete Local Name
        b'M', b'i', b'c', b'r', b'o', b'B', b'i', b't', b'-', b'B', b'L', b'E',
        // 16-bit UUID 列表: Battery Service (0x180F)
        0x03, 0x03, 0x0F, 0x18,
    ];

  #[rustfmt::skip]
    let scan_data = &[
        // 扫描响应中的制造商特定数据
        0x05, 0xFF, // AD Type: Manufacturer Specific Data
        0x59, 0x00, // Nordic Semiconductor 公司 ID
        0x01, 0x00, // 自定义数据
    ];

  // ========================================
  // 3. 开启广播并运行 BLE 事件循环
  // ========================================
  ble.start_advertising();
  info!("开始 BLE 广播...");

  // 运行 BLE 主循环（永不返回）
  ble.run(adv_data, scan_data).await;
}
