//! Echo loopback test component
//! Sends Echo command via BLE and listens for response

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// EchoPanel component
#[component]
pub fn EchoPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;
  let last_frame = app_state.last_frame;

  let (input_text, set_input_text) = signal(String::new());
  let (echo_result, set_echo_result) = signal("--".to_string());
  let (testing, set_testing) = signal(false);

  // Listen for Echo response
  Effect::new(move |_| {
    if let Some(frame) = last_frame.get() {
      if frame.cmd == Command::EchoResp as u8 {
        // Echo response payload is the original sent data
        let text = String::from_utf8(frame.payload.clone())
          .unwrap_or_else(|_| format!("(hex) {}", hex::encode(&frame.payload)));
        set_echo_result.set(format!("Echo: {text}"));
        set_testing.set(false);
      }
    }
  });

  // Echo test
  let on_echo = move |_| {
    if !connected.get() || testing.get() {
      return;
    }
    let text = input_text.get();
    if text.is_empty() {
      return;
    }

    set_testing.set(true);
    set_echo_result.set("Sending...".to_string());

    let payload = text.as_bytes().to_vec();
    match build_frame(Command::Echo as u8, &payload) {
      Ok(frame) => {
        log_tx(format!("Echo: {text}"), Some(frame.clone()));
        spawn_local(async move {
          if let Some(shared_ble) = get_global_ble() {
            let ble = shared_ble.0.borrow().clone();
            if let Err(e) = ble.send(&frame).await {
              log_error(format!("Send failed: {e}"));
              set_echo_result.set(format!("Error: {e}"));
              set_testing.set(false);
            }
            // Response will trigger Effect update via last_frame signal
          }
        });
      }
      Err(e) => {
        log_error(format!("Build frame failed: {e}"));
        set_echo_result.set(format!("Error: {e}"));
        set_testing.set(false);
      }
    }
  };

  view! {
      <section class="card">
          <h2>"Echo Loopback Test"</h2>
          <div class="row">
              <input
                  type="text"
                  placeholder="Enter text (≤56 chars)"
                  prop:value=move || input_text.get()
                  on:input=move |ev| {
                      set_input_text.set(event_target_value(&ev));
                  }
                  disabled=move || !connected.get() || testing.get()
              />
              <button
                  disabled=move || !connected.get() || testing.get() || input_text.get().is_empty()
                  on:click=on_echo
              >
                  {move || if testing.get() { "Sending..." } else { "Send" }}
              </button>
          </div>
          <div class="row">
              <span class="stat">{move || echo_result.get()}</span>
          </div>
      </section>
  }
}
