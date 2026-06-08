//! LED 5×5 matrix control component

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{AppState, get_global_ble};
use leptos::prelude::*;
use microbit_ble_protocol::{Command, MAX_PAYLOAD, build_frame_vec as build_frame};
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

  // LED state: on/off state of 25 LEDs (binary mode)
  let (led_state, set_led_state) = signal([false; 25]);

  // Brightness mode state
  let (brightness_mode, set_brightness_mode) = signal(false);
  let (brightness_level, set_brightness_level) = signal(128u8);
  let (led_brightness, set_led_brightness) = signal([0u8; 25]);

  // Display a character
  let (char_input, set_char_input) = signal(String::new());
  let send_char = move |_| {
    let ch = char_input.get();
    if let Some(c) = ch.chars().next()
      && connected.get()
    {
      match build_frame(Command::LedChar as u8, &[c as u8]) {
        Ok(frame) => {
          log_tx(format!("LED Char '{c}'"), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    }
  };

  // Scroll text input
  let (scroll_input, set_scroll_input) = signal(String::new());
  let send_scroll = move |_| {
    let text = scroll_input.get();
    let bytes: Vec<u8> = text.bytes().take(MAX_PAYLOAD).collect();
    if !bytes.is_empty() && connected.get() {
      match build_frame(Command::LedScroll as u8, &bytes) {
        Ok(frame) => {
          log_tx(format!("LED Scroll \"{text}\""), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    }
  };

  // Toggle brightness mode also syncs LED states
  let toggle_brightness_mode = move |_| {
    let new_mode = !brightness_mode.get();
    set_brightness_mode.set(new_mode);
    if new_mode {
      // Convert binary state to brightness
      let state = led_state.get();
      let level = brightness_level.get();
      let new_brightness = state.map(|on| if on { level } else { 0 });
      set_led_brightness.set(new_brightness);
    } else {
      // Convert brightness back to binary
      let brightness = led_brightness.get();
      let new_state = brightness.map(|v| v > 0);
      set_led_state.set(new_state);
    }
  };

  // Toggle LED (works in both modes)
  let toggle_led = move |index: usize| {
    if brightness_mode.get() {
      let mut new_br = led_brightness.get();
      new_br[index] = if new_br[index] > 0 {
        0
      } else {
        brightness_level.get()
      };
      set_led_brightness.set(new_br);
    } else {
      let mut new_state = led_state.get();
      new_state[index] = !new_state[index];
      set_led_state.set(new_state);
    }
  };

  // Clear all LEDs
  let clear_all = move |_| {
    if brightness_mode.get() {
      set_led_brightness.set([0u8; 25]);
    } else {
      set_led_state.set([false; 25]);
    }
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
  let all_on = move |_| {
    if brightness_mode.get() {
      let level = brightness_level.get();
      set_led_brightness.set([level; 25]);
      if connected.get() {
        let values = [level; 25];
        match build_frame(Command::LedBrightness as u8, &values) {
          Ok(frame) => {
            log_tx("LED Brightness All On".to_string(), Some(frame.clone()));
            ble_send_frame(frame);
          }
          Err(e) => log_error(format!("Build frame failed: {e}")),
        }
      }
    } else {
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
    }
  };

  // Apply current state to device
  let apply_to_device = move |_| {
    if !connected.get() {
      return;
    }
    if brightness_mode.get() {
      let values = led_brightness.get();
      match build_frame(Command::LedBrightness as u8, &values) {
        Ok(frame) => {
          log_tx("LED Brightness".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    } else {
      let state = led_state.get();
      let payload: Vec<u8> = state.iter().map(|&on| u8::from(on)).collect();
      match build_frame(Command::LedSet as u8, &payload) {
        Ok(frame) => {
          log_tx("LED Set".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("Build frame failed: {e}")),
      }
    }
  };

  view! {
      <section class="card">
          <h2>"LED 5×5 Matrix"</h2>

          // Mode toggle
          <div class="row" style="margin-bottom: 8px;">
              <label class="toggle-label">
                  <input
                      type="checkbox"
                      checked=move || brightness_mode.get()
                      on:change=toggle_brightness_mode
                  />
                  " Grayscale brightness mode"
              </label>
          </div>

          // Brightness level slider (only in brightness mode)
          {move || if brightness_mode.get() {
              Some(view! {
                  <div class="row" style="margin-bottom: 8px;">
                      <span class="axis-label">"Brightness"</span>
                      <input
                          type="range"
                          min="1"
                          max="255"
                          value=move || brightness_level.get().to_string()
                          on:input=move |ev| {
                              let val = event_target_value(&ev).parse::<u8>().unwrap_or(128);
                              set_brightness_level.set(val);
                          }
                          class="brightness-slider"
                      />
                      <span class="stat">{move || brightness_level.get().to_string()}</span>
                  </div>
              }.into_any())
          } else {
              None
          }}

          // 5x5 LED grid
          <div class="led-grid">
              <For
                  each=move || 0..25
                  key=|i| *i
                  children=move |index| {
                      let onclick = {
                          let toggle = toggle_led;
                          move |_| toggle(index)
                      };
                      let class = move || {
                          let mut c = "led".to_string();
                          if brightness_mode.get() {
                              let br = led_brightness.get()[index];
                              if br > 0 {
                                  c.push_str(" on brightness-mode");
                                  c
                              } else {
                                  c
                              }
                          } else {
                              if led_state.get()[index] {
                                  c.push_str(" on");
                              }
                              c
                          }
                      };
                      let style = move || {
                          if brightness_mode.get() {
                              let br = led_brightness.get()[index];
                              if br > 0 {
                                  let opacity = (br as f32 / 255.0).max(0.15);
                                  format!("opacity: {:.2}", opacity)
                              } else {
                                  String::new()
                              }
                          } else {
                              String::new()
                          }
                      };
                      view! {
                          <div
                              class=class
                              style=style
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
                  on:click=apply_to_device
              >
                  "📤 Apply to Device"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=clear_all
              >
                  "Clear"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=all_on
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

          // Scroll text
          <div class="row">
              <input
                  type="text"
                  placeholder="Scroll text on LED matrix..."
                  maxlength="60"
                  on:input=move |ev| {
                      set_scroll_input.set(event_target_value(&ev));
                  }
              />
              <button
                  disabled=move || !connected.get() || scroll_input.get().is_empty()
                  on:click=send_scroll
              >
                  "📜 Scroll Text"
              </button>
          </div>

          <p class="hint">"Click a cell to toggle on/off, then press 'Apply to Device'. Enable grayscale mode for brightness control. Use 'Scroll Text' to display scrolling messages."</p>
      </section>
  }
}
