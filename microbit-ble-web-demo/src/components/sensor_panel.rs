//! Sensor panel component
//! Displays onboard temperature sensor readings and button status
//! Sends commands via BLE and listens for responses

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Send data frame via global BLE service
fn ble_send_frame(frame: Vec<u8>) {
  spawn_local(async move {
    if let Some(shared_ble) = get_global_ble() {
      let ble = shared_ble.0.borrow().clone();
      if let Err(e) = ble.send(&frame).await {
        log_error(format!("Send failed: {e}"));
      }
    }
  });
}

/// SensorPanel component
#[component]
pub fn SensorPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;
  let last_frame = app_state.last_frame;

  // Sensor data state
  let (temperature, set_temperature) = signal("--".to_string());
  let (button_a, set_button_a) = signal(false);
  let (button_b, set_button_b) = signal(false);
  let (btn_subscribed, set_btn_subscribed) = signal(false);
  let (updating, set_updating) = signal(false);

  // Listen for received frames, handle temperature response and button events
  Effect::new(move |_| {
    if let Some(frame) = last_frame.get() {
      match frame.cmd {
        // Pong response (0x81)
        cmd if cmd == Command::Pong as u8 => {
          log::info!("Received Pong response");
        }
        // Temperature response (0x85)
        cmd if cmd == Command::TempResp as u8 => {
          if frame.payload.len() >= 2 {
            // Temperature value is i16, unit 0.01°C
            let raw = i16::from_le_bytes([frame.payload[0], frame.payload[1]]);
            let temp = f32::from(raw) / 100.0;
            set_temperature.set(format!("{temp:.1}°C"));
          } else if !frame.payload.is_empty() {
            // Single-byte temperature (integer)
            let temp = frame.payload[0] as i8;
            set_temperature.set(format!("{temp}°C"));
          }
          set_updating.set(false);
        }
        // Button event (0x90)
        cmd if cmd == Command::BtnEvent as u8 && frame.payload.len() >= 2 => {
          let btn_id = frame.payload[0];
          let pressed = frame.payload[1] != 0;
          match btn_id {
            1 => set_button_a.set(pressed),
            2 => set_button_b.set(pressed),
            _ => {}
          }
        }
        _ => {}
      }
    }
  });

  // Ping
  let on_ping = move |_| {
    if !connected.get() {
      return;
    }
    match build_frame(Command::Ping as u8, &[]) {
      Ok(frame) => {
        log_tx("Ping".to_string(), Some(frame.clone()));
        ble_send_frame(frame);
      }
      Err(e) => log_error(format!("Build frame failed: {e}")),
    }
  };

  // Request temperature reading
  let request_temperature = move |_| {
    if !connected.get() || updating.get() {
      return;
    }
    set_updating.set(true);
    match build_frame(Command::TempGet as u8, &[]) {
      Ok(frame) => {
        log_tx("TempGet".to_string(), Some(frame.clone()));
        ble_send_frame(frame);
      }
      Err(e) => {
        log_error(format!("Build frame failed: {e}"));
        set_updating.set(false);
      }
    }
  };

  // Subscribe/unsubscribe button status
  let toggle_btn_subscribe = move |_| {
    if !connected.get() {
      return;
    }
    let new_state = !btn_subscribed.get();
    let payload = [u8::from(new_state)];
    match build_frame(Command::BtnSubscribe as u8, &payload) {
      Ok(frame) => {
        log_tx(
          format!("BtnSubscribe({})", if new_state { "on" } else { "off" }),
          Some(frame.clone()),
        );
        set_btn_subscribed.set(new_state);
        ble_send_frame(frame);
      }
      Err(e) => log_error(format!("Build frame failed: {e}")),
    }
  };

  // Button state display
  let btn_a_class = move || {
    let mut cls = "btn-state".to_string();
    if button_a.get() {
      cls.push_str(" pressed");
    }
    cls
  };
  let btn_b_class = move || {
    let mut cls = "btn-state".to_string();
    if button_b.get() {
      cls.push_str(" pressed");
    }
    cls
  };

  view! {
      <section class="card">
          <h2>"Onboard Status"</h2>
          <div class="row">
              <button disabled=move || !connected.get() on:click=on_ping>"🏓 Ping"</button>
              <button disabled=move || !connected.get() || updating.get() on:click=request_temperature>
                  {move || if updating.get() { "🌡 Reading..." } else { "🌡 Read Temperature" }}
              </button>
              <span class="stat">{move || format!("Temp: {}", temperature.get())}</span>
          </div>
          <div class="row" style="margin-top: 14px;">
              <label>
                  <input
                      type="checkbox"
                      disabled=move || !connected.get()
                      checked=move || btn_subscribed.get()
                      on:change=toggle_btn_subscribe
                  />
                  " Subscribe to Button A/B events"
              </label>
          </div>
          <div class="row">
              <span class="stat">"A: "<span class=btn_a_class>{move || if button_a.get() { "Pressed" } else { "--" }}</span></span>
              <span class="stat">"B: "<span class=btn_b_class>{move || if button_b.get() { "Pressed" } else { "--" }}</span></span>
          </div>
          <p class="hint">"Press onboard button A or B after subscribing to see real-time status."</p>
      </section>
  }
}
