//! Echo 回环测试组件
//! 通过 BLE 发送 Echo 命令并监听响应

use crate::components::comm_log::{log_error, log_tx};
use crate::context::{get_global_ble, AppState};
use crate::utils::{build_frame, Command};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// EchoPanel 组件
#[component]
pub fn EchoPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;
  let last_frame = app_state.last_frame;

  let (input_text, set_input_text) = signal(String::new());
  let (echo_result, set_echo_result) = signal("--".to_string());
  let (testing, set_testing) = signal(false);

  // 监听 Echo 响应
  Effect::new(move |_| {
    if let Some(frame) = last_frame.get() {
      if frame.cmd == Command::EchoResp as u8 {
        // Echo 响应的 payload 就是原始发送的数据
        let text = String::from_utf8(frame.payload.clone())
          .unwrap_or_else(|_| format!("(hex) {}", hex::encode(&frame.payload)));
        set_echo_result.set(format!("回显: {text}"));
        set_testing.set(false);
      }
    }
  });

  // Echo 测试
  let on_echo = move |_| {
    if !connected.get() || testing.get() {
      return;
    }
    let text = input_text.get();
    if text.is_empty() {
      return;
    }

    set_testing.set(true);
    set_echo_result.set("发送中...".to_string());

    let payload = text.as_bytes().to_vec();
    match build_frame(Command::Echo as u8, &payload) {
      Ok(frame) => {
        log_tx(format!("Echo: {text}"), Some(frame.clone()));
        spawn_local(async move {
          if let Some(shared_ble) = get_global_ble() {
            let ble = shared_ble.0.borrow().clone();
            if let Err(e) = ble.send(&frame).await {
              log_error(format!("Echo 发送失败: {e}"));
              set_echo_result.set(format!("错误: {e}"));
              set_testing.set(false);
            }
            // 响应会通过 last_frame 信号触发 Effect 更新
          }
        });
      }
      Err(e) => {
        log_error(format!("构建帧失败: {e}"));
        set_echo_result.set(format!("错误: {e}"));
        set_testing.set(false);
      }
    }
  };

  view! {
      <section class="card">
          <h2>"Echo 回环测试"</h2>
          <div class="row">
              <input
                  type="text"
                  placeholder="输入文本（≤56 字符）"
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
                  {move || if testing.get() { "发送中..." } else { "发送" }}
              </button>
          </div>
          <div class="row">
              <span class="stat">{move || echo_result.get()}</span>
          </div>
      </section>
  }
}
