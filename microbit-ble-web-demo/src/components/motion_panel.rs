//! Motion sensor panel component
//! Displays accelerometer and magnetometer data from micro:bit V2

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{AppState, get_global_ble};
use leptos::prelude::*;
use microbit_ble_protocol::{Command, build_frame_vec as build_frame};
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

/// MotionPanel component - accelerometer & magnetometer
#[component]
pub fn MotionPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;
  let last_frame = app_state.last_frame;

  // Accelerometer data (unit: g)
  let (accel_x, set_accel_x) = signal(0.0_f32);
  let (accel_y, set_accel_y) = signal(0.0_f32);
  let (accel_z, set_accel_z) = signal(0.0_f32);
  let (accel_subscribed, set_accel_subscribed) = signal(false);

  // Magnetometer data (unit: μT)
  let (magnet_x, set_magnet_x) = signal(0.0_f32);
  let (magnet_y, set_magnet_y) = signal(0.0_f32);
  let (magnet_z, set_magnet_z) = signal(0.0_f32);
  let (magnet_subscribed, set_magnet_subscribed) = signal(false);

  // Compass heading (degrees)
  let (heading, set_heading) = signal(0.0_f32);

  // Listen for received frames
  Effect::new(move |_| {
    if let Some(frame) = last_frame.get() {
      match frame.cmd {
        // Accelerometer data (0x8A): 6 bytes, 3× i16 LE [x, y, z], unit 0.01g
        cmd if cmd == Command::AccelData as u8 && frame.payload.len() >= 6 => {
          let x = i16::from_le_bytes([frame.payload[0], frame.payload[1]]);
          let y = i16::from_le_bytes([frame.payload[2], frame.payload[3]]);
          let z = i16::from_le_bytes([frame.payload[4], frame.payload[5]]);
          set_accel_x.set(f32::from(x) / 100.0);
          set_accel_y.set(f32::from(y) / 100.0);
          set_accel_z.set(f32::from(z) / 100.0);
        }
        // Magnetometer data (0x8B): 6 bytes, 3× i16 LE [x, y, z], unit 0.1μT
        cmd if cmd == Command::MagnetData as u8 && frame.payload.len() >= 6 => {
          let x = i16::from_le_bytes([frame.payload[0], frame.payload[1]]);
          let y = i16::from_le_bytes([frame.payload[2], frame.payload[3]]);
          let z = i16::from_le_bytes([frame.payload[4], frame.payload[5]]);
          set_magnet_x.set(f32::from(x) / 10.0);
          set_magnet_y.set(f32::from(y) / 10.0);
          set_magnet_z.set(f32::from(z) / 10.0);
          // Compute compass heading from X/Y
          let rad = f32::atan2(f32::from(y), f32::from(x));
          let deg = rad.to_degrees();
          set_heading.set(if deg < 0.0 { deg + 360.0 } else { deg });
        }
        _ => {}
      }
    }
  });

  // Toggle accelerometer subscription
  let toggle_accel = move |_| {
    if !connected.get() {
      return;
    }
    let new_state = !accel_subscribed.get();
    let payload = [u8::from(new_state)];
    match build_frame(Command::AccelSubscribe as u8, &payload) {
      Ok(frame) => {
        log_tx(
          format!("AccelSubscribe({})", if new_state { "on" } else { "off" }),
          Some(frame.clone()),
        );
        set_accel_subscribed.set(new_state);
        if !new_state {
          set_accel_x.set(0.0);
          set_accel_y.set(0.0);
          set_accel_z.set(0.0);
        }
        ble_send_frame(frame);
      }
      Err(e) => log_error(format!("Build frame failed: {e}")),
    }
  };

  // Toggle magnetometer subscription
  let toggle_magnet = move |_| {
    if !connected.get() {
      return;
    }
    let new_state = !magnet_subscribed.get();
    let payload = [u8::from(new_state)];
    match build_frame(Command::MagnetSubscribe as u8, &payload) {
      Ok(frame) => {
        log_tx(
          format!("MagnetSubscribe({})", if new_state { "on" } else { "off" }),
          Some(frame.clone()),
        );
        set_magnet_subscribed.set(new_state);
        if !new_state {
          set_magnet_x.set(0.0);
          set_magnet_y.set(0.0);
          set_magnet_z.set(0.0);
          set_heading.set(0.0);
        }
        ble_send_frame(frame);
      }
      Err(e) => log_error(format!("Build frame failed: {e}")),
    }
  };

  // Accelerometer magnitude bar helpers
  let accel_mag = move || {
    let x = accel_x.get();
    let y = accel_y.get();
    let z = accel_z.get();
    (x * x + y * y + z * z).sqrt()
  };

  // Compass direction text
  let compass_dir = move || {
    let h = heading.get();
    match h {
      h if !(22.5..337.5).contains(&h) => "N",
      h if h < 67.5 => "NE",
      h if h < 112.5 => "E",
      h if h < 157.5 => "SE",
      h if h < 202.5 => "S",
      h if h < 247.5 => "SW",
      h if h < 292.5 => "W",
      _ => "NW",
    }
  };

  view! {
      <section class="card">
          <h2>"🧭 Motion Sensors"</h2>

          // Accelerometer section
          <div class="sensor-section">
              <div class="sensor-header">
                  <span class="sensor-title">"📊 Accelerometer"</span>
                  <label class="toggle-label">
                      <input
                          type="checkbox"
                          disabled=move || !connected.get()
                          checked=move || accel_subscribed.get()
                          on:change=toggle_accel
                      />
                      " Subscribe"
                  </label>
              </div>
              <div class="axis-grid">
                  <div class="axis-item">
                      <span class="axis-label">"X"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-x"
                              style=move || format!("width: {}%", (accel_x.get().abs() * 33.0).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.2}g", accel_x.get())}</span>
                  </div>
                  <div class="axis-item">
                      <span class="axis-label">"Y"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-y"
                              style=move || format!("width: {}%", (accel_y.get().abs() * 33.0).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.2}g", accel_y.get())}</span>
                  </div>
                  <div class="axis-item">
                      <span class="axis-label">"Z"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-z"
                              style=move || format!("width: {}%", (accel_z.get().abs() * 33.0).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.2}g", accel_z.get())}</span>
                  </div>
              </div>
              <div class="stat-row">
                  <span class="stat">"Mag: "{move || format!("{:.2}g", accel_mag())}</span>
              </div>
          </div>

          // Magnetometer section
          <div class="sensor-section">
              <div class="sensor-header">
                  <span class="sensor-title">"🧲 Magnetometer"</span>
                  <label class="toggle-label">
                      <input
                          type="checkbox"
                          disabled=move || !connected.get()
                          checked=move || magnet_subscribed.get()
                          on:change=toggle_magnet
                      />
                      " Subscribe"
                  </label>
              </div>
              <div class="axis-grid">
                  <div class="axis-item">
                      <span class="axis-label">"X"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-mx"
                              style=move || format!("width: {}%", (magnet_x.get().abs() * 0.2).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.1}μT", magnet_x.get())}</span>
                  </div>
                  <div class="axis-item">
                      <span class="axis-label">"Y"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-my"
                              style=move || format!("width: {}%", (magnet_y.get().abs() * 0.2).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.1}μT", magnet_y.get())}</span>
                  </div>
                  <div class="axis-item">
                      <span class="axis-label">"Z"</span>
                      <div class="axis-bar-container">
                          <div
                              class="axis-bar axis-mz"
                              style=move || format!("width: {}%", (magnet_z.get().abs() * 0.2).min(100.0))
                          ></div>
                      </div>
                      <span class="axis-value">{move || format!("{:.1}μT", magnet_z.get())}</span>
                  </div>
              </div>

              // Compass display
              <div class="compass-container">
                  <div class="compass" style=move || format!("transform: rotate({}deg)", -heading.get())>
                      <div class="compass-needle"></div>
                      <div class="compass-n">"N"</div>
                  </div>
                  <span class="compass-heading">
                      {move || format!("{}° {}", heading.get() as u16, compass_dir())}
                  </span>
              </div>
          </div>

          <p class="hint">"Subscribe to receive real-time sensor data. Accelerometer measures tilt/movement, magnetometer acts as a compass."</p>
      </section>
  }
}
