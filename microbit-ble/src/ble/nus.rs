//! Nordic UART Service (NUS)
//!
//! 行业标准的 BLE 串口透传协议，被 Web Bluetooth 完美支持。
//!
//! UUIDs (128-bit):
//! - Service:       6e400001-b5a3-f393-e0a9-e50e24dcca9e
//! - RX (Write):    6e400002-b5a3-f393-e0a9-e50e24dcca9e  (Central -> Peripheral)
//! - TX (Notify):   6e400003-b5a3-f393-e0a9-e50e24dcca9e  (Peripheral -> Central)
//!
//! 通过 [`crate::ble::protocol`] 帧承载浏览器与 micro:bit 之间的双向通信。

use defmt::info;
use nrf_softdevice::ble::Connection;

/// NUS 单次写入/通知最大字节数。
///
/// 选择 64 与 ATT MTU 对齐（MTU 64 - 3 字节 ATT 头 = 61 可用）。
/// 选择固定数组而非 `heapless::Vec`，避免不同 crate 间 heapless 版本冲突。
pub const NUS_MAX_LEN: usize = 64;

/// Nordic UART Service GATT 定义
///
/// 注意：这里的 RX/TX 字段类型是 `[u8; NUS_MAX_LEN]`。
/// nrf-softdevice 的 `GattValue for [u8; N]` 实现允许接收任意 0..=N 字节的写入：
/// 不足 N 字节时会用 0 填充（这正是变长协议帧通常的处理方式）。
/// 真正的帧长度由协议头自身（[`crate::ble::protocol`] 中的 LEN 字段）确定，
/// 因此尾部填充 0 不会影响解析。
#[nrf_softdevice::gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
pub struct NusService {
  /// RX：浏览器写入的数据 (Write / Write Without Response)
  #[characteristic(
    uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e",
    write,
    write_without_response
  )]
  pub rx: [u8; NUS_MAX_LEN],

  /// TX：板子向浏览器推送的数据 (Notify)
  #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", notify)]
  pub tx: [u8; NUS_MAX_LEN],
}

impl NusService {
  /// 通过 TX 特征值向 central 发送一帧数据。
  ///
  /// 数据若不足 [`NUS_MAX_LEN`]，会用 0 字节填充到固定长度。
  /// 接收端依据协议帧自身的 LEN 字段决定有效长度。
  pub fn send(&self, conn: &Connection, data: &[u8]) -> Result<(), ()> {
    if data.len() > NUS_MAX_LEN {
      return Err(());
    }
    let mut buf = [0u8; NUS_MAX_LEN];
    buf[..data.len()].copy_from_slice(data);
    self.tx_notify(conn, &buf).map_err(|_| ())
  }

  /// 处理 GATT 事件
  pub fn handle_event(event: NusServiceEvent) -> Option<NusRx> {
    match event {
      NusServiceEvent::TxCccdWrite { notifications } => {
        info!(
          "NUS TX 通知 {}",
          if notifications {
            "已启用"
          } else {
            "已禁用"
          }
        );
        None
      }
      NusServiceEvent::RxWrite(data) => Some(NusRx { buf: data }),
    }
  }
}

/// 一次 RX 写入的固定长度快照。
#[derive(Clone, Copy)]
pub struct NusRx {
  buf: [u8; NUS_MAX_LEN],
}

impl NusRx {
  /// 返回完整的固定长度缓冲区切片（含尾部填充 0）。
  /// 协议解析器应该自行根据帧头 LEN 字段截取有效数据。
  pub fn as_slice(&self) -> &[u8] {
    &self.buf
  }
}
