//! Connect/Disconnect button component
//! Uses the globally shared BleService for BLE connect/disconnect operations
//! After successful connection, sets up data receive callback, parses frames and dispatches to AppState

use crate::components::comm_log::{log_error, log_info, log_rx};
use crate::context::{get_global_ble, AppState, ReceivedFrame};
use crate::services::ble::BleConnectionState;
use crate::utils::{parse_frame, to_hex};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// ConnectButtons component
/// Contains "Connect" and "Disconnect" buttons
#[component]
pub fn ConnectButtons() -> impl IntoView {
  // Get application state from context
  let app_state = expect_context::<AppState>();

  let AppState {
    connected,
    connecting,
    device_name,
    last_frame,
  } = app_state;

  // Connect operation
  let on_connect = move |_| {
    if connected.get() || connecting.get() {
      return;
    }

    connecting.set(true);

    spawn_local(async move {
      let shared_ble = match get_global_ble() {
        Some(ble) => ble,
        None => {
          log_error("BLE service not initialized".to_string());
          connecting.set(false);
          return;
        }
      };

      // Clone for async connection
      let mut ble_clone = shared_ble.0.borrow().clone();

      // Set data receive callback: parse frame and dispatch to last_frame signal
      ble_clone.set_on_data(move |data| {
        log::debug!("RX raw: {}", to_hex(&data));
        // Record raw receive log
        log_rx("RX".to_string(), Some(data.clone()));

        // Parse frame
        if let Some((cmd, payload)) = parse_frame(&data) {
          log::info!("RX frame: cmd=0x{:02x}, payload={}", cmd, to_hex(&payload));
          // Dispatch to global signal
          last_frame.set(Some(ReceivedFrame { cmd, payload }));
        } else {
          log::warn!("Failed to parse frame: {}", to_hex(&data));
        }
      });

      // Set state change callback
      ble_clone.set_on_state_change(move |state| match state {
        BleConnectionState::Disconnected => {
          connected.set(false);
          connecting.set(false);
          device_name.set(None);
          log_info("Device disconnected".to_string());
        }
        BleConnectionState::Connecting => {
          connecting.set(true);
          connected.set(false);
        }
        BleConnectionState::Connected => {
          connected.set(true);
          connecting.set(false);
        }
      });

      // Execute connection
      let result = ble_clone.connect().await;

      match result {
        Ok(()) => {
          // Connection successful, write modified ble_clone back to shared state
          let name = ble_clone.device_name();
          device_name.set(name.clone());

          // Write back shared state (including rx_char/tx_char, etc.)
          *shared_ble.0.borrow_mut() = ble_clone;

          log_info(format!(
            "Connected: {}",
            name.unwrap_or_else(|| "Unknown device".to_string())
          ));
        }
        Err(e) => {
          log::error!("Connection failed: {e}");
          connecting.set(false);
          log_error(format!("Connection failed: {e}"));
        }
      }
    });
  };

  // Disconnect operation
  let on_disconnect = move |_| {
    if !connected.get() {
      return;
    }

    if let Some(shared_ble) = get_global_ble() {
      shared_ble.0.borrow_mut().disconnect();
    }
  };

  view! {
      <button
          class="primary"
          disabled=move || connecting.get() || connected.get()
          on:click=on_connect
      >
          {move || {
              if connecting.get() {
                  "Connecting..."
              } else {
                  "Connect micro:bit"
              }
          }}
      </button>
      <button
          class="danger"
          disabled=move || !connected.get()
          on:click=on_disconnect
      >
          "Disconnect"
      </button>
  }
}
