//! LED 5×5 matrix control component

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Helper function to send data frame via global BLE service
fn ble_send_frame(frame: Vec<u8>) {
  spawn_local(async move {
    if let Some(shared_ble) = get_global_ble() {
      let ble = shared_ble.0.borrow().clone();
      if let Err(e) = ble.send(&frame).await {
        log_error(format!("Send failed: {e}"));
      }
    } else {
      log_error("BLE service not initialized".to_string());
    }
  });
}

/// LED matrix card component
#[component]
pub fn LedMatrixCard() -> impl IntoView {
  // Get connection state from context
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;

  // LED state: on/off state of 25 LEDs
  let (led_state, set_led_state) = signal([false; 25]);

  // Toggle a single LED
  let toggle_led = move |index: usize| {
    let mut new_state = led_state.get();
    new_state[index] = !new_state[index];
    set_led_state.set(new_state);
  };

  // Send LED state to device
  let send_led = move |_| {
    if !connected.get() {
      return;
    }
    let state = led_state.get();
    let payload: Vec<u8> = state.iter().map(|&on| u8::from(on)).collect();
    match build_frame(Command::LedSet as u8, &payload) {
      Ok(frame) => {
        log_tx("LED Set".to_string(), Some(frame.clone()));
        ble_send_frame(frame);
      }
      Err(e) => log_error(format!("Build frame failed: {e}")),
    }
  };

  // Clear all LEDs
  let clear_led = move |_| {
    set_led_state.set([false; 25]);
    if connected.get() {
      match build_frame(Command::LedClear as u8, &[]) {
        Ok(frame) => {
          log_tx("LED Clear".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    }
  };

  // Turn on all LEDs
  let all_led = move |_| {
    set_led_state.set([true; 25]);
    if connected.get() {
      let payload = vec![1u8; 25];
      match build_frame(Command::LedSet as u8, &payload) {
        Ok(frame) => {
          log_tx("LED All On".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    }
  };

  // Display a character
  let (char_input, set_char_input) = signal(String::new());
  let send_char = move |_| {
    let ch = char_input.get();
    if let Some(c) = ch.chars().next() {
      if connected.get() {
        match build_frame(Command::LedChar as u8, &[c as u8]) {
          Ok(frame) => {
            log_tx(format!("LED Char '{c}'"), Some(frame.clone()));
            ble_send_frame(frame);
          }
          Err(e) => log_error(format!("Build frame failed: {e}")),
        }
      }
    }
  };

  view! {
      <section class="card">
          <h2>"LED 5×5 Matrix"</h2>

          // 5x5 LED grid
          <div class="led-grid">
              <For
                  each=move || 0..25
                  key=|i| *i
                  children=move |index| {
                      let on = move || led_state.get()[index];
                      let onclick = {
                          let toggle = toggle_led;
                          move |_| toggle(index)
                      };
                      let class = move || {
                          let mut c = "led".to_string();
                          if on() {
                              c.push_str(" on");
                          }
                          c
                      };
                      view! {
                          <div
                              class=class
                              role="gridcell"
                              tabindex="0"
                              aria-label=format!("LED {}-{}", index / 5 + 1, index % 5 + 1)
                              on:click=onclick
                              on:keydown=move |ev| {
                                  if ev.key() == "Enter" || ev.key() == " " {
                                      ev.prevent_default();
                                      toggle_led(index);
                                  }
                              }
                          ></div>
                      }
                  }
              />
          </div>

          // Control buttons
          <div class="row">
              <button
                  disabled=move || !connected.get()
                  on:click=send_led
              >
                  "📤 Apply to Device"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=clear_led
              >
                  "Clear"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=all_led
              >
                  "All On"
              </button>
          </div>

          // Character input
          <div class="row">
              <input
                  type="text"
                  placeholder="Single character (A-Z 0-9 ! ?)"
                  maxlength="1"
                  on:input=move |ev| {
                      set_char_input.set(event_target_value(&ev));
                  }
              />
              <button
                  disabled=move || !connected.get() || char_input.get().is_empty()
                  on:click=send_char
              >
                  "Display Char"
              </button>
          </div>

          <p class="hint">"Click a cell to toggle on/off, then press 'Apply to Device' to write via BLE."</p>
      </section>
  }
}
