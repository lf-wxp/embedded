//! Battery Service (BAS, UUID 0x180F)
//!
//! 标准 BLE 电池服务，作为可插拔的 mod 提供。
//! 由于 Web Bluetooth 默认 blocklist 中包含 Battery Service（隐私指纹问题），
//! 该服务保留在工程里供 nRF Connect 等 native BLE 工具使用。
//!
//! 本模块只定义 GATT 结构和事件处理函数，不参与 NUS 的主数据通路。

use defmt::info;
use nrf_softdevice::Softdevice;
use nrf_softdevice::ble::Connection;

/// 电池服务 (Battery Service UUID: 0x180F)
#[nrf_softdevice::gatt_service(uuid = "180f")]
pub struct BatteryService {
  /// 电池电量特征值 (Battery Level UUID: 0x2A19)
  /// 值范围: 0-100，表示电池百分比
  #[characteristic(uuid = "2a19", read, notify)]
  pub level: u8,
}

impl BatteryService {
  /// 注册到 SoftDevice 并返回服务实例
  pub fn register(sd: &mut Softdevice) -> Result<Self, nrf_softdevice::ble::gatt_server::RegisterError> {
    Self::new(sd)
  }

  /// 设置电量并向已订阅的连接发送 notify
  pub fn update(&self, level: u8, conn: Option<&Connection>) {
    let level = level.min(100);
    let _ = self.level_set(&level);
    if let Some(c) = conn {
      // 客户端可能未订阅 notify，失败可忽略
      let _ = self.level_notify(c, &level);
    }
  }

  /// 处理订阅事件（CCCD 写入）
  pub fn handle_event(event: BatteryServiceEvent) {
    match event {
      BatteryServiceEvent::LevelCccdWrite { notifications } => {
        info!(
          "电池电量通知 {}",
          if notifications {
            "已启用"
          } else {
            "已禁用"
          }
        );
      }
    }
  }
}
