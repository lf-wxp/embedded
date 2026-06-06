//! micro:bit V2 BLE Web Demo - Leptos frontend
//!
//! Web Bluetooth console rewritten with Leptos v0.8
//! The original HTML/JS version has been refactored to Rust + WebAssembly + Leptos components

#![allow(non_snake_case)]

pub mod components;
pub mod context;
pub mod services;
pub mod utils;

use context::{AppState, SharedBleService, init_global_ble};
use leptos::context::provide_context;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Application main entry point
#[wasm_bindgen(start)]
pub fn main() {
  // Initialize panic hook and logger
  console_error_panic_hook::set_once();
  console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

  log::info!("micro:bit BLE Web Demo - Leptos v0.8");

  // Mount Leptos app to DOM
  mount_to_body(|| view! { <App /> });
}

/// Main application component
#[component]
fn App() -> impl IntoView {
  // Create global application state
  let app_state = AppState::new();
  provide_context(app_state);

  // Create globally shared BLE service (stored via thread_local, no Send+Sync needed)
  let shared_ble = SharedBleService::new();
  init_global_ble(shared_ble);

  view! {
      // Top navigation bar - consistent with original HTML structure
      <header>
          <h1>"🔵 micro:bit V2 BLE Console"</h1>
          <span class="hint">"Web Bluetooth · Nordic UART Service"</span>
          // Connection status indicator
          <components::status_indicator::StatusIndicator />
          // Connect/Disconnect buttons
          <components::connect_buttons::ConnectButtons />
      </header>

      // Main content area
      <main>
          // LED 5×5 matrix control
          <components::led_matrix::LedMatrixCard />
          // Onboard sensors / button status
          <components::sensor_panel::SensorPanel />
          // Echo loopback test
          <components::echo_panel::EchoPanel />
          // Communication log
          <components::comm_log::CommLog />
      </main>
  }
}
