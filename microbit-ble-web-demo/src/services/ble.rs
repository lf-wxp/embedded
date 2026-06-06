//! Web Bluetooth service module
//! Wraps Web Bluetooth API calls, providing a Rust-style async interface
//!
//! Reference: https://developer.mozilla.org/en-US/docs/Web/API/Web_Bluetooth_API

use crate::utils::{NUS_RX_CHAR, NUS_SERVICE, NUS_TX_CHAR};
use js_sys::{JsString, Uint8Array};

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  Bluetooth, BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
  BluetoothRemoteGattServer, BluetoothRemoteGattService, EventTarget, RequestDeviceOptions,
};

/// BLE connection state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleConnectionState {
  Disconnected,
  Connecting,
  Connected,
}

/// Data receive callback type (WASM single-threaded, using Rc<RefCell<>>)
type OnDataCallback = Rc<RefCell<Option<Box<dyn Fn(Vec<u8>)>>>>;
/// State change callback type
type OnStateChangeCallback = Rc<RefCell<Option<Box<dyn Fn(BleConnectionState)>>>>;

/// BLE service handle (interior mutability)
/// WASM single-threaded, safe to share using Rc<RefCell<>>
#[derive(Clone)]
pub struct BleService {
  pub device: Option<BluetoothDevice>,
  pub server: Option<BluetoothRemoteGattServer>,
  pub rx_char: Option<BluetoothRemoteGattCharacteristic>,
  pub tx_char: Option<BluetoothRemoteGattCharacteristic>,
  on_data: OnDataCallback,
  on_state_change: OnStateChangeCallback,
}

impl Default for BleService {
  fn default() -> Self {
    Self::new()
  }
}

impl BleService {
  pub fn new() -> Self {
    Self {
      device: None,
      server: None,
      rx_char: None,
      tx_char: None,
      on_data: Rc::new(RefCell::new(None)),
      on_state_change: Rc::new(RefCell::new(None)),
    }
  }

  /// Request user to select a device and connect
  pub async fn connect(&mut self) -> Result<(), String> {
    let bluetooth = get_bluetooth()?;

    // Build requestDevice options
    // https://developer.mozilla.org/en-US/docs/Web/API/Bluetooth/requestDevice
    let opts = RequestDeviceOptions::new();

    // filters: filter by name prefix
    let filter = BluetoothLeScanFilterInit::new();
    filter.set_name_prefix("MicroBit");
    opts.set_filters(&[filter]);

    // optionalServices: service UUIDs to access
    opts.set_optional_services(&[JsString::from(NUS_SERVICE)]);

    // Request device - request_device returns Promise<BluetoothDevice>
    // JsFuture::<BluetoothDevice>::await directly yields BluetoothDevice
    let device: BluetoothDevice = JsFuture::from(bluetooth.request_device(&opts))
      .await
      .map_err(|e| format!("Device selection failed or user cancelled: {e:?}"))?;

    // Set up disconnect listener
    self.setup_disconnect_listener(&device)?;

    // Connect to GATT server
    self.set_state(BleConnectionState::Connecting);
    let gatt_server = device.gatt().ok_or("Device does not support GATT")?;
    // connect() returns Promise<BluetoothRemoteGattServer>
    let server: BluetoothRemoteGattServer = JsFuture::from(gatt_server.connect())
      .await
      .map_err(|e| format!("GATT connection failed: {e:?}"))?;

    // Get Nordic UART Service
    // get_primary_services_with_str returns Promise<Array<BluetoothRemoteGattService>>
    let services = JsFuture::from(server.get_primary_services_with_str(NUS_SERVICE))
      .await
      .map_err(|e| format!("Failed to get NUS service: {e:?}"))?;
    // Array<T>.get(0) directly returns T type
    let service: BluetoothRemoteGattService = services.get(0);

    // Get RX characteristic (write)
    // get_characteristic_with_str returns Promise<BluetoothRemoteGattCharacteristic>
    let rx_char: BluetoothRemoteGattCharacteristic =
      JsFuture::from(service.get_characteristic_with_str(NUS_RX_CHAR))
        .await
        .map_err(|e| format!("Failed to get RX characteristic: {e:?}"))?;

    // Get TX characteristic (notify)
    let tx_char: BluetoothRemoteGattCharacteristic =
      JsFuture::from(service.get_characteristic_with_str(NUS_TX_CHAR))
        .await
        .map_err(|e| format!("Failed to get TX characteristic: {e:?}"))?;

    // Set up notification callback (set before starting notifications to avoid missing data)
    self.setup_notification_handler(&tx_char)?;

    // Start notifications - start_notifications returns Promise<BluetoothRemoteGattCharacteristic>
    JsFuture::from(tx_char.start_notifications())
      .await
      .map_err(|e| format!("Failed to start notifications: {e:?}"))?;

    self.device = Some(device);
    self.server = Some(server);
    self.rx_char = Some(rx_char);
    self.tx_char = Some(tx_char);

    self.set_state(BleConnectionState::Connected);

    Ok(())
  }

  /// Set up disconnect listener
  fn setup_disconnect_listener(&self, device: &BluetoothDevice) -> Result<(), String> {
    let on_state_change = self.on_state_change.clone();

    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      log::info!("Device disconnected");
      let callback = on_state_change.borrow();
      if let Some(ref cb) = *callback {
        cb(BleConnectionState::Disconnected);
      }
    }) as Box<dyn FnMut(_)>);

    let callback: &js_sys::Function = closure.as_ref().unchecked_ref();
    device.set_ongattserverdisconnected(Some(callback));
    closure.forget();

    Ok(())
  }

  /// Set up notification handler for TX characteristic
  fn setup_notification_handler(
    &self,
    tx_char: &BluetoothRemoteGattCharacteristic,
  ) -> Result<(), String> {
    let on_data = self.on_data.clone();

    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
      let target = event.target().unwrap();
      let char: &BluetoothRemoteGattCharacteristic = target.unchecked_ref();

      // Get characteristic value (DataView)
      let data_view = match char.value() {
        Some(v) => v,
        None => {
          log::warn!("No value in notification event");
          return;
        }
      };

      // Correctly convert from DataView to Vec<u8>
      // DataView cannot be directly passed to Uint8Array::new(), need buffer + offset + length
      let buffer = data_view.buffer();
      let byte_offset = data_view.byte_offset() as u32;
      let byte_length = data_view.byte_length() as u32;
      let bytes =
        js_sys::Uint8Array::new_with_byte_offset_and_length(&buffer, byte_offset, byte_length)
          .to_vec();

      // Invoke callback function
      let callback = on_data.borrow();
      if let Some(ref cb) = *callback {
        cb(bytes);
      }
    }) as Box<dyn FnMut(_)>);

    // Set notification handler
    // add_event_listener_with_callback is defined on EventTarget
    // BluetoothRemoteGattCharacteristic implements AsRef<EventTarget>
    let callback: &js_sys::Function = closure.as_ref().unchecked_ref();
    <BluetoothRemoteGattCharacteristic as AsRef<EventTarget>>::as_ref(tx_char)
      .add_event_listener_with_callback("characteristicvaluechanged", callback)
      .map_err(|e| format!("Failed to add notification handler: {e:?}"))?;

    // Forget the closure (avoid memory leak)
    closure.forget();

    Ok(())
  }

  /// Send data frame
  pub async fn send(&self, data: &[u8]) -> Result<(), String> {
    let rx_char = self.rx_char.as_ref().ok_or("Not connected")?;

    // Convert &[u8] to Uint8Array (write_value_with_u8_array requires &Uint8Array parameter)
    // SAFETY: Uint8Array::view requires data to not be modified or deallocated during the call,
    // since we create the view before the await and data is a borrowed reference,
    // data must remain valid until JsFuture::from(promise).await completes
    let js_data = unsafe { Uint8Array::view(data) };
    let promise = rx_char
      .write_value_with_u8_array(&js_data)
      .map_err(|e| format!("Write request failed: {e:?}"))?;
    JsFuture::from(promise)
      .await
      .map_err(|e| format!("Write failed: {e:?}"))?;

    log::debug!("TX: {}", crate::utils::to_hex(data));
    Ok(())
  }

  /// Disconnect
  pub fn disconnect(&mut self) {
    if let Some(device) = &self.device {
      if let Some(gatt) = device.gatt() {
        gatt.disconnect();
      }
    }
    self.device = None;
    self.server = None;
    self.rx_char = None;
    self.tx_char = None;
    self.set_state(BleConnectionState::Disconnected);
  }

  /// Check if connected
  pub fn is_connected(&self) -> bool {
    self.server.as_ref().is_some_and(|s| s.connected())
  }

  /// Get device name
  pub fn device_name(&self) -> Option<String> {
    self.device.as_ref().and_then(|d| d.name())
  }

  /// Set data receive callback
  pub fn set_on_data<F>(&mut self, f: F)
  where
    F: Fn(Vec<u8>) + 'static,
  {
    *self.on_data.borrow_mut() = Some(Box::new(f));
  }

  /// Set state change callback
  pub fn set_on_state_change<F>(&mut self, f: F)
  where
    F: Fn(BleConnectionState) + 'static,
  {
    *self.on_state_change.borrow_mut() = Some(Box::new(f));
  }

  /// Internal: set state and trigger callback
  fn set_state(&self, state: BleConnectionState) {
    log::info!("BLE state changed: {state:?}");
    let callback = self.on_state_change.borrow();
    if let Some(ref cb) = *callback {
      cb(state);
    }
  }
}

/// Get Bluetooth API from Window object
fn get_bluetooth() -> Result<Bluetooth, String> {
  let window = web_sys::window().ok_or("Failed to get window object")?;
  window
    .navigator()
    .bluetooth()
    .ok_or_else(|| "Browser does not support Web Bluetooth API".to_string())
}

/// Check if browser supports Web Bluetooth
pub fn is_bluetooth_supported() -> bool {
  web_sys::window()
    .and_then(|w| w.navigator().bluetooth())
    .is_some()
}
