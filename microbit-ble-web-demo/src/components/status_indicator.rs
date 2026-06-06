//! 连接状态指示器组件
//! 使用与原始 HTML 一致的 .status 类名

use crate::context::AppState;
use leptos::prelude::*;

/// StatusIndicator 组件
/// 显示当前 BLE 连接状态（未连接/连接中/已连接）
/// 原始 HTML: `<div class="status" id="status"><span class="dot"></span><span>未连接</span></div>`
#[component]
pub fn StatusIndicator() -> impl IntoView {
  let app_state = expect_context::<AppState>();

  let AppState {
    connected,
    connecting,
    device_name,
    ..
  } = app_state;

  // 动态计算 .status 的额外类名
  let status_class = move || {
    let mut cls = "status".to_string();
    if connecting.get() {
      cls.push_str(" connecting");
    } else if connected.get() {
      cls.push_str(" connected");
    }
    cls
  };

  // 状态文本
  let status_text = move || {
    if connecting.get() {
      "连接中...".to_string()
    } else if connected.get() {
      device_name.get().unwrap_or_else(|| "已连接".to_string())
    } else {
      "未连接".to_string()
    }
  };

  view! {
      <div class=status_class>
          <span class="dot"></span>
          <span>{status_text}</span>
      </div>
  }
}
