//! 传感器面板组件
//! 显示板载温度传感器读数和按键状态
//! 通过 BLE 发送命令并监听响应

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// 通过全局 BLE 服务发送数据帧
fn ble_send_frame(frame: Vec<u8>) {
  spawn_local(async move {
    if let Some(shared_ble) = get_global_ble() {
      let ble = shared_ble.0.borrow().clone();
      if let Err(e) = ble.send(&frame).await {
        log_error(format!("发送失败: {e}"));
      }
    }
  });
}

/// SensorPanel 组件
#[component]
pub fn SensorPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;
  let last_frame = app_state.last_frame;

  // 传感器数据状态
  let (temperature, set_temperature) = signal("--".to_string());
  let (button_a, set_button_a) = signal(false);
  let (button_b, set_button_b) = signal(false);
  let (btn_subscribed, set_btn_subscribed) = signal(false);
  let (updating, set_updating) = signal(false);

  // 监听接收到的帧，处理温度响应和按键事件
  Effect::new(move |_| {
    if let Some(frame) = last_frame.get() {
      match frame.cmd {
        // Pong 响应 (0x81)
        cmd if cmd == Command::Pong as u8 => {
          log::info!("收到 Pong 响应");
        }
        // 温度响应 (0x85)
        cmd if cmd == Command::TempResp as u8 => {
          if frame.payload.len() >= 2 {
            // 温度值为 i16，单位 0.01°C
            let raw = i16::from_le_bytes([frame.payload[0], frame.payload[1]]);
            let temp = f32::from(raw) / 100.0;
            set_temperature.set(format!("{temp:.1}°C"));
          } else if !frame.payload.is_empty() {
            // 单字节温度（整数）
            let temp = frame.payload[0] as i8;
            set_temperature.set(format!("{temp}°C"));
          }
          set_updating.set(false);
        }
        // 按键事件 (0x90)
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
      Err(e) => log_error(format!("构建帧失败: {e}")),
    }
  };

  // 请求温度读取
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
        log_error(format!("构建帧失败: {e}"));
        set_updating.set(false);
      }
    }
  };

  // 订阅/取消订阅按键状态
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
      Err(e) => log_error(format!("构建帧失败: {e}")),
    }
  };

  // 按键状态显示
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
          <h2>"板载状态"</h2>
          <div class="row">
              <button disabled=move || !connected.get() on:click=on_ping>"🏓 Ping"</button>
              <button disabled=move || !connected.get() || updating.get() on:click=request_temperature>
                  {move || if updating.get() { "🌡 读取中..." } else { "🌡 读取温度" }}
              </button>
              <span class="stat">{move || format!("温度: {}", temperature.get())}</span>
          </div>
          <div class="row" style="margin-top: 14px;">
              <label>
                  <input
                      type="checkbox"
                      disabled=move || !connected.get()
                      checked=move || btn_subscribed.get()
                      on:change=toggle_btn_subscribe
                  />
                  " 订阅按键 A/B 事件"
              </label>
          </div>
          <div class="row">
              <span class="stat">"A: "<span class=btn_a_class>{move || if button_a.get() { "按下" } else { "--" }}</span></span>
              <span class="stat">"B: "<span class=btn_b_class>{move || if button_b.get() { "按下" } else { "--" }}</span></span>
          </div>
          <p class="hint">"订阅后按下板载按键 A 或 B，实时显示状态。"</p>
      </section>
  }
}
