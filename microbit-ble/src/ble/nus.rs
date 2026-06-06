//! Nordic UART Service (NUS)
//!
//! Industry-standard BLE serial pass-through protocol, perfectly supported by Web Bluetooth.
//!
//! UUIDs (128-bit):
//! - Service:       6e400001-b5a3-f393-e0a9-e50e24dcca9e
//! - RX (Write):    6e400002-b5a3-f393-e0a9-e50e24dcca9e  (Central -> Peripheral)
//! - TX (Notify):   6e400003-b5a3-f393-e0a9-e50e24dcca9e  (Peripheral -> Central)
//!
//! Bidirectional communication between browser and micro:bit is carried via [`microbit_ble_protocol`] frames.

use defmt::info;
use nrf_softdevice::ble::Connection;

/// Maximum bytes per NUS write/notification.
///
/// Set to 64 to align with ATT MTU (MTU 64 - 3 bytes ATT header = 61 usable).
/// Uses fixed array instead of `heapless::Vec` to avoid heapless version conflicts across crates.
pub const NUS_MAX_LEN: usize = 64;

/// Nordic UART Service GATT definition
///
/// Note: The RX/TX field type here is `[u8; NUS_MAX_LEN]`.
/// The `GattValue for [u8; N]` implementation in nrf-softdevice allows receiving writes of any 0..=N bytes:
/// bytes shorter than N are zero-padded (this is the usual handling for variable-length protocol frames).
/// The actual frame length is determined by the protocol header itself (the LEN field in [`microbit_ble_protocol`]),
/// so zero-padding at the tail does not affect parsing.
#[nrf_softdevice::gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
pub struct NusService {
  /// RX: data written by browser (Write / Write Without Response)
  #[characteristic(
    uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e",
    write,
    write_without_response
  )]
  pub rx: [u8; NUS_MAX_LEN],

  /// TX: data pushed from board to browser (Notify)
  #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", notify)]
  pub tx: [u8; NUS_MAX_LEN],
}

impl NusService {
  /// Send a frame of data to central via TX characteristic value.
  ///
  /// If data is shorter than [`NUS_MAX_LEN`], it is zero-padded to the fixed length.
  /// The receiver determines the valid length from the LEN field in the protocol frame itself.
  pub fn send(&self, conn: &Connection, data: &[u8]) -> Result<(), ()> {
    if data.len() > NUS_MAX_LEN {
      return Err(());
    }
    let mut buf = [0u8; NUS_MAX_LEN];
    buf[..data.len()].copy_from_slice(data);
    self.tx_notify(conn, &buf).map_err(|_| ())
  }

  /// Handle GATT events
  pub fn handle_event(event: NusServiceEvent) -> Option<NusRx> {
    match event {
      NusServiceEvent::TxCccdWrite { notifications } => {
        info!(
          "NUS TX notification {}",
          if notifications { "enabled" } else { "disabled" }
        );
        None
      }
      NusServiceEvent::RxWrite(data) => Some(NusRx { buf: data }),
    }
  }
}

/// Fixed-length snapshot of one RX write.
#[derive(Clone, Copy)]
pub struct NusRx {
  buf: [u8; NUS_MAX_LEN],
}

impl NusRx {
  /// Returns the full fixed-length buffer slice (including trailing zero-padding).
  /// The protocol parser should extract valid data based on the LEN field in the frame header.
  pub fn as_slice(&self) -> &[u8] {
    &self.buf
  }
}
