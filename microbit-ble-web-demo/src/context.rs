//! Global context module
//! Provides application-level shared state, including BLE service and reactive signals

use crate::services::ble::BleService;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Received frame data (already parsed)
#[derive(Clone, Debug)]
pub struct ReceivedFrame {
  pub cmd: u8,
  pub payload: Vec<u8>,
}

/// Global application state
#[derive(Clone, Copy)]
pub struct AppState {
  /// Connection status
  pub connected: RwSignal<bool>,
  /// Currently connecting
  pub connecting: RwSignal<bool>,
  /// Device name
  pub device_name: RwSignal<Option<String>>,
  /// Most recently received frame (other components listen on this signal for data)
  pub last_frame: RwSignal<Option<ReceivedFrame>>,
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

impl AppState {
  pub fn new() -> Self {
    Self {
      connected: RwSignal::new(false),
      connecting: RwSignal::new(false),
      device_name: RwSignal::new(None),
      last_frame: RwSignal::new(None),
    }
  }
}

/// Globally shared BLE service handle
/// WASM is single-threaded, so Rc<RefCell<>> is sufficient
#[derive(Clone)]
pub struct SharedBleService(pub Rc<RefCell<BleService>>);

impl SharedBleService {
  pub fn new() -> Self {
    Self(Rc::new(RefCell::new(BleService::new())))
  }
}

impl Default for SharedBleService {
  fn default() -> Self {
    Self::new()
  }
}

// Global BLE service instance (WASM single-threaded, safe to use thread_local)
thread_local! {
  static GLOBAL_BLE: RefCell<Option<SharedBleService>> = const { RefCell::new(None) };
}

/// Initialize global BLE service
pub fn init_global_ble(ble: SharedBleService) {
  GLOBAL_BLE.with(|g| {
    *g.borrow_mut() = Some(ble);
  });
}

/// Get a clone of the global BLE service
pub fn get_global_ble() -> Option<SharedBleService> {
  GLOBAL_BLE.with(|g| g.borrow().clone())
}
