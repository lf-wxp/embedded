//! 连接/断开按钮组件
//! 使用全局共享的 BleService 进行 BLE 连接/断开操作
//! 连接成功后设置数据接收回调，解析帧并分发到 AppState

use crate::components::comm_log::{log_error, log_info, log_rx};
use crate::context::{get_global_ble, AppState, ReceivedFrame};
use crate::services::ble::BleConnectionState;
use crate::utils::{parse_frame, to_hex};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// ConnectButtons 组件
/// 包含"连接"和"断开"两个按钮
#[component]
pub fn ConnectButtons() -> impl IntoView {
  // 从上下文中获取应用状态
  let app_state = expect_context::<AppState>();

  let AppState {
    connected,
    connecting,
    device_name,
    last_frame,
  } = app_state;

  // 连接操作
  let on_connect = move |_| {
    if connected.get() || connecting.get() {
      return;
    }

    connecting.set(true);

    spawn_local(async move {
      let shared_ble = match get_global_ble() {
        Some(ble) => ble,
        None => {
          log_error("BLE 服务未初始化".to_string());
          connecting.set(false);
          return;
        }
      };

      // 克隆出来执行异步连接
      let mut ble_clone = shared_ble.0.borrow().clone();

      // 设置数据接收回调：解析帧并分发到 last_frame 信号
      ble_clone.set_on_data(move |data| {
        log::debug!("RX raw: {}", to_hex(&data));
        // 记录原始接收日志
        log_rx("RX".to_string(), Some(data.clone()));

        // 解析帧
        if let Some((cmd, payload)) = parse_frame(&data) {
          log::info!("RX frame: cmd=0x{:02x}, payload={}", cmd, to_hex(&payload));
          // 分发到全局信号
          last_frame.set(Some(ReceivedFrame { cmd, payload }));
        } else {
          log::warn!("无法解析帧: {}", to_hex(&data));
        }
      });

      // 设置状态变化回调
      ble_clone.set_on_state_change(move |state| match state {
        BleConnectionState::Disconnected => {
          connected.set(false);
          connecting.set(false);
          device_name.set(None);
          log_info("设备已断开连接".to_string());
        }
        BleConnectionState::Connecting => {
          connecting.set(true);
          connected.set(false);
        }
        BleConnectionState::Connected => {
          connected.set(true);
          connecting.set(false);
        }
      });

      // 执行连接
      let result = ble_clone.connect().await;

      match result {
        Ok(()) => {
          // 连接成功，将修改后的 ble_clone 写回共享状态
          let name = ble_clone.device_name();
          device_name.set(name.clone());

          // 写回共享状态（包含 rx_char/tx_char 等）
          *shared_ble.0.borrow_mut() = ble_clone;

          log_info(format!(
            "已连接: {}",
            name.unwrap_or_else(|| "未知设备".to_string())
          ));
        }
        Err(e) => {
          log::error!("连接失败: {e}");
          connecting.set(false);
          log_error(format!("连接失败: {e}"));
        }
      }
    });
  };

  // 断开操作
  let on_disconnect = move |_| {
    if !connected.get() {
      return;
    }

    if let Some(shared_ble) = get_global_ble() {
      shared_ble.0.borrow_mut().disconnect();
    }
  };

  view! {
      <button
          class="primary"
          disabled=move || connecting.get() || connected.get()
          on:click=on_connect
      >
          {move || {
              if connecting.get() {
                  "连接中..."
              } else {
                  "连接 micro:bit"
              }
          }}
      </button>
      <button
          class="danger"
          disabled=move || !connected.get()
          on:click=on_disconnect
      >
          "断开"
      </button>
  }
}
