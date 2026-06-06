//! LED 5×5 矩阵控制组件

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// 通过全局 BLE 服务发送数据帧的辅助函数
fn ble_send_frame(frame: Vec<u8>) {
  spawn_local(async move {
    if let Some(shared_ble) = get_global_ble() {
      let ble = shared_ble.0.borrow().clone();
      if let Err(e) = ble.send(&frame).await {
        log_error(format!("发送失败: {e}"));
      }
    } else {
      log_error("BLE 服务未初始化".to_string());
    }
  });
}

/// LED 矩阵卡片组件
#[component]
pub fn LedMatrixCard() -> impl IntoView {
  // 从上下文获取连接状态
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;

  // LED 状态：25 个 LED 的开关状态
  let (led_state, set_led_state) = signal([false; 25]);

  // 切换单个 LED
  let toggle_led = move |index: usize| {
    let mut new_state = led_state.get();
    new_state[index] = !new_state[index];
    set_led_state.set(new_state);
  };

  // 发送 LED 状态到设备
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
      Err(e) => log_error(format!("构建帧失败: {e}")),
    }
  };

  // 清空 LED
  let clear_led = move |_| {
    set_led_state.set([false; 25]);
    if connected.get() {
      match build_frame(Command::LedClear as u8, &[]) {
        Ok(frame) => {
          log_tx("LED Clear".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("构建帧失败: {e}")),
      }
    }
  };

  // 全部点亮
  let all_led = move |_| {
    set_led_state.set([true; 25]);
    if connected.get() {
      let payload = vec![1u8; 25];
      match build_frame(Command::LedSet as u8, &payload) {
        Ok(frame) => {
          log_tx("LED All On".to_string(), Some(frame.clone()));
          ble_send_frame(frame);
        }
        Err(e) => log_error(format!("构建帧失败: {e}")),
      }
    }
  };

  // 显示字符
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
          Err(e) => log_error(format!("构建帧失败: {e}")),
        }
      }
    }
  };

  view! {
      <section class="card">
          <h2>"LED 5×5 矩阵"</h2>

          // 5x5 LED 网格
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

          // 控制按钮
          <div class="row">
              <button
                  disabled=move || !connected.get()
                  on:click=send_led
              >
                  "📤 应用到设备"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=clear_led
              >
                  "清空"
              </button>
              <button
                  disabled=move || !connected.get()
                  on:click=all_led
              >
                  "全亮"
              </button>
          </div>

          // 字符输入
          <div class="row">
              <input
                  type="text"
                  placeholder="单个字符 (A-Z 0-9 ! ?)"
                  maxlength="1"
                  on:input=move |ev| {
                      set_char_input.set(event_target_value(&ev));
                  }
              />
              <button
                  disabled=move || !connected.get() || char_input.get().is_empty()
                  on:click=send_char
              >
                  "显示字符"
              </button>
          </div>

          <p class="hint">"点击格子切换亮/灭，再按「应用」通过 BLE 写入设备。"</p>
      </section>
  }
}
