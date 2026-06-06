//! Connection status indicator component
//! Uses the same .status class names as the original HTML

use crate::context::AppState;
use leptos::prelude::*;

/// StatusIndicator component
/// Displays current BLE connection status (disconnected/connecting/connected)
/// Original HTML: `<div class="status" id="status"><span class="dot"></span><span>Disconnected</span></div>`
#[component]
pub fn StatusIndicator() -> impl IntoView {
  let app_state = expect_context::<AppState>();

  let AppState {
    connected,
    connecting,
    device_name,
    ..
  } = app_state;

  // Dynamically calculate extra class names for .status
  let status_class = move || {
    let mut cls = "status".to_string();
    if connecting.get() {
      cls.push_str(" connecting");
    } else if connected.get() {
      cls.push_str(" connected");
    }
    cls
  };

  // Status text
  let status_text = move || {
    if connecting.get() {
      "Connecting...".to_string()
    } else if connected.get() {
      device_name.get().unwrap_or_else(|| "Connected".to_string())
    } else {
      "Disconnected".to_string()
    }
  };

  view! {
      <div class=status_class>
          <span class="dot"></span>
          <span>{status_text}</span>
      </div>
  }
}
