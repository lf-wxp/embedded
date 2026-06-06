//! micro:bit V2 BLE 外设广播示例（NUS + LED 矩阵 + 按键 + 温度）
//!
//! 通过 Nordic UART Service 与浏览器（Web Bluetooth）/ nRF Connect 双向通信。
//! 演示功能：
//! - 控制板载 5x5 LED 矩阵（位图 / 字符）
//! - 上报板载按键 A/B 事件
//! - 读取芯片内部温度
//! - 协议帧 echo（连通性测试）
//!
//! 详见 [`crate::ble::protocol`] 模块的协议定义。

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

/// BLE 设备名称（显示在手机的蓝牙扫描列表中）
const DEVICE_NAME: &[u8] = b"MicroBit-BLE";

/// Nordic UART Service 128-bit UUID（小端序，用于广播包）
const NUS_UUID_BYTES: [u8; 16] = [
  0x9E, 0xCA, 0xDC, 0x24, 0x0E, 0xE5, 0xA9, 0xE0, 0x93, 0xF3, 0xA3, 0xB5, 0x01, 0x00, 0x40, 0x6E,
];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
  info!("=== micro:bit V2 BLE (NUS) 示例启动 ===");

  // ========================================
  // 1. 初始化 embassy-nrf 外设
  //    GPIOTE 优先级必须低于 SoftDevice 使用的 P0/P1/P4
  // ========================================
  let mut nrf_config = embassy_nrf::config::Config::default();
  nrf_config.gpiote_interrupt_priority = Priority::P2;
  nrf_config.time_interrupt_priority = Priority::P2;
  let p = embassy_nrf::init(nrf_config);

  // ========================================
  // 2. 启动 LED 矩阵刷新任务
  // ========================================
  spawner.spawn(led_matrix_task(LedPins {
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
  }).expect("led_matrix_task spawn 失败"));

  // 启动时显示一个图标，提示固件已运行
  ble::led_matrix::show_char(b'B');

  // ========================================
  // 3. 启动按键监听任务
  // ========================================
  spawner.spawn(
    button_task(ButtonPins {
      btn_a: p.P0_14,
      btn_b: p.P0_23,
    })
    .expect("button_task spawn 失败"),
  );

  // ========================================
  // 4. 初始化 BLE 控制器（启用 SoftDevice）
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
  // 5. 构建广播数据
  // ========================================
  // Web Bluetooth 要求服务 UUID 必须出现在广播包（adv_data）中才能被 filter 匹配。
  // 128-bit UUID 占 18 字节，Flags 占 3 字节，剩余 10 字节给短设备名。
  // 短名 "MicroBit" 是完整设备名 "MicroBit-BLE" 的前缀，符合 BLE 规范。
  // 广播包总大小：3 (Flags) + 18 (UUID) + 10 (短名) = 31 字节，刚好满。
  static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
    .services_128(ServiceList::Complete, &[NUS_UUID_BYTES])
    .short_name("MicroBit")     // 完整设备名的前缀，放入广播包
    .build();

  // 扫描响应包：完整设备名（中央设备主动扫描后收到）
  static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
    .full_name("MicroBit-BLE")
    .build();

  // ========================================
  // 6. 开启广播并运行主循环
  // ========================================
  ble.start_advertising();
  info!("开始 BLE 广播，等待浏览器/nRF Connect 连接...");
  ble.run(&ADV_DATA, &SCAN_DATA).await;
}
