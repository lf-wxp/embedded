//! micro:bit V2 BLE Web Demo - Leptos 前端
//!
//! 基于 Leptos v0.8 重写的 Web Bluetooth 控制台
//! 原 HTML/JS 版本已重构为 Rust + WebAssembly + Leptos 组件

#![allow(non_snake_case)]

pub mod components;
pub mod context;
pub mod services;
pub mod utils;

use context::{init_global_ble, AppState, SharedBleService};
use leptos::context::provide_context;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// 应用程序主入口
#[wasm_bindgen(start)]
pub fn main() {
  // 初始化 panic hook 和日志
  console_error_panic_hook::set_once();
  console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

  log::info!("micro:bit BLE Web Demo - Leptos v0.8");

  // 挂载 Leptos 应用到 DOM
  mount_to_body(|| view! { <App /> });
}

/// 主应用组件
#[component]
fn App() -> impl IntoView {
  // 创建全局应用状态
  let app_state = AppState::new();
  provide_context(app_state);

  // 创建全局共享 BLE 服务（通过 thread_local 存储，不需要 Send+Sync）
  let shared_ble = SharedBleService::new();
  init_global_ble(shared_ble);

  view! {
      // 顶部导航栏 - 与原始 HTML 结构一致
      <header>
          <h1>"🔵 micro:bit V2 BLE 控制台"</h1>
          <span class="hint">"Web Bluetooth · Nordic UART Service"</span>
          // 连接状态指示器
          <components::status_indicator::StatusIndicator />
          // 连接/断开按钮
          <components::connect_buttons::ConnectButtons />
      </header>

      // 主内容区域
      <main>
          // LED 5×5 矩阵控制
          <components::led_matrix::LedMatrixCard />
          // 板载传感器 / 按键状态
          <components::sensor_panel::SensorPanel />
          // Echo 回环测试
          <components::echo_panel::EchoPanel />
          // 通信日志
          <components::comm_log::CommLog />
      </main>
  }
}
