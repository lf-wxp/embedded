//! Battery Service (BAS, UUID 0x180F)
//!
//! Standard BLE Battery Service, provided as a pluggable module.
//! Since Web Bluetooth's default blocklist includes Battery Service (privacy fingerprinting issue),
//! this service is kept in the project for use with native BLE tools like nRF Connect.
//!
//! This module only defines the GATT structure and event handling function, it does not participate in NUS data path.

use defmt::info;
use nrf_softdevice::Softdevice;
use nrf_softdevice::ble::Connection;

/// Battery Service (Battery Service UUID: 0x180F)
#[nrf_softdevice::gatt_service(uuid = "180f")]
pub struct BatteryService {
  /// Battery Level characteristic (Battery Level UUID: 0x2A19)
  /// Value range: 0-100, representing battery percentage
  #[characteristic(uuid = "2a19", read, notify)]
  pub level: u8,
}

impl BatteryService {
  /// Register with SoftDevice and return service instance
  pub fn register(sd: &mut Softdevice) -> Result<Self, nrf_softdevice::ble::gatt_server::RegisterError> {
    Self::new(sd)
  }

  /// Set battery level and send notification to subscribed connections
  pub fn update(&self, level: u8, conn: Option<&Connection>) {
    let level = level.min(100);
    let _ = self.level_set(&level);
    if let Some(c) = conn {
      // Client may not have subscribed to notifications, failure is acceptable
      let _ = self.level_notify(c, &level);
    }
  }

  /// Handle subscription event (CCCD write)
  pub fn handle_event(event: BatteryServiceEvent) {
    match event {
      BatteryServiceEvent::LevelCccdWrite { notifications } => {
        info!(
          "Battery level notification {}",
          if notifications {
            "enabled"
          } else {
            "disabled"
          }
        );
      }
    }
  }
}
