//! 通信日志组件
//! 显示所有 BLE 通信的 TX/RX 数据

use leptos::prelude::*;
use std::cell::RefCell;

/// 日志条目类型
#[derive(Clone, Debug)]
pub enum LogEntryType {
  Tx,    // 发送
  Rx,    // 接收
  Info,  // 信息
  Error, // 错误
}

/// 日志条目
#[derive(Clone, Debug)]
pub struct LogEntry {
  pub timestamp: u128,
  pub entry_type: LogEntryType,
  pub message: String,
  pub data: Option<Vec<u8>>,
}

impl LogEntry {
  pub fn new(entry_type: LogEntryType, message: String, data: Option<Vec<u8>>) -> Self {
    // 使用 js_sys::Date::now() 获取时间戳（WASM 不支持 std::time::SystemTime）
    let timestamp = js_sys::Date::now() as u128;

    Self {
      timestamp,
      entry_type,
      message,
      data,
    }
  }

  pub fn format_time(&self) -> String {
    let secs = (self.timestamp / 1000) % 86400;
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let ms = self.timestamp % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
  }

  pub fn type_class(&self) -> &'static str {
    match self.entry_type {
      LogEntryType::Tx => "tx",
      LogEntryType::Rx => "rx",
      LogEntryType::Info => "info",
      LogEntryType::Error => "err",
    }
  }
}

// 全局日志写入信号（WASM 是单线程的，使用 thread_local 安全）
thread_local! {
  static LOG_SIGNAL: RefCell<Option<WriteSignal<Vec<LogEntry>>>> = const { RefCell::new(None) };
}

/// 注册日志写入信号（由 CommLog 组件调用）
fn register_log_signal(set_logs: WriteSignal<Vec<LogEntry>>) {
  LOG_SIGNAL.with(|s| {
    *s.borrow_mut() = Some(set_logs);
  });
}

/// 写入日志条目（可在任何上下文中调用，包括 JS 回调）
fn write_log(entry: LogEntry) {
  LOG_SIGNAL.with(|s| {
    if let Some(set_logs) = *s.borrow() {
      set_logs.update(|logs| {
        logs.push(entry);
        if logs.len() > 500 {
          logs.remove(0);
        }
      });
    }
  });
}

/// CommLog 组件
#[component]
pub fn CommLog() -> impl IntoView {
  let (logs, set_logs) = signal(Vec::<LogEntry>::new());
  let (hex_only, set_hex_only) = signal(false);

  // 注册全局日志写入信号
  register_log_signal(set_logs);

  // 清空日志
  let on_clear = move |_| {
    set_logs.set(Vec::new());
  };

  // 切换 hex 模式
  let toggle_hex = move |_| {
    set_hex_only.update(|v| *v = !*v);
  };

  view! {
      <section class="card">
          <h2>"通信日志"</h2>
          <div id="log">
              <For
                  each=move || logs.get()
                  key=|entry| entry.timestamp
                  children=move |entry| {
                      let cls = entry.type_class();
                      let time_str = entry.format_time();
                      let msg = if hex_only.get_untracked() {
                          entry.data.as_ref()
                              .map(hex::encode)
                              .unwrap_or_else(|| entry.message.clone())
                      } else {
                          let data_str = entry.data.as_ref()
                              .map(|d| format!(" [{}]", hex::encode(d)))
                              .unwrap_or_default();
                          format!("{}{}", entry.message, data_str)
                      };
                      view! {
                          <div class=cls>
                              {format!("[{}] {}", time_str, msg)}
                          </div>
                      }
                  }
              />
          </div>
          <div class="row" style="margin-top: 8px;">
              <button on:click=on_clear>"清空日志"</button>
              <label>
                  <input
                      type="checkbox"
                      checked=move || hex_only.get()
                      on:change=toggle_hex
                  />
                  " 仅显示 hex"
              </label>
          </div>
      </section>
  }
}

/// 全局日志函数 - 可在任何上下文中调用（包括 BLE 回调）
pub fn log_tx(message: String, data: Option<Vec<u8>>) {
  write_log(LogEntry::new(LogEntryType::Tx, message, data));
}

pub fn log_rx(message: String, data: Option<Vec<u8>>) {
  write_log(LogEntry::new(LogEntryType::Rx, message, data));
}

pub fn log_info(message: String) {
  write_log(LogEntry::new(LogEntryType::Info, message, None));
}

pub fn log_error(message: String) {
  write_log(LogEntry::new(LogEntryType::Error, message, None));
}
