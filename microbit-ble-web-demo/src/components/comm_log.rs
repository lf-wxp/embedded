//! Communication log component
//! Displays all BLE TX/RX communication data

use leptos::prelude::*;
use std::cell::RefCell;

/// Log entry type
#[derive(Clone, Debug)]
pub enum LogEntryType {
  Tx,    // Transmit
  Rx,    // Receive
  Info,  // Information
  Error, // Error
}

/// Log entry
#[derive(Clone, Debug)]
pub struct LogEntry {
  pub timestamp: u128,
  pub entry_type: LogEntryType,
  pub message: String,
  pub data: Option<Vec<u8>>,
}

impl LogEntry {
  pub fn new(entry_type: LogEntryType, message: String, data: Option<Vec<u8>>) -> Self {
    // Use js_sys::Date::now() to get timestamp (WASM doesn't support std::time::SystemTime)
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

// Global log write signal (WASM is single-threaded, safe to use thread_local)
thread_local! {
  static LOG_SIGNAL: RefCell<Option<WriteSignal<Vec<LogEntry>>>> = const { RefCell::new(None) };
}

/// Register log write signal (called by CommLog component)
fn register_log_signal(set_logs: WriteSignal<Vec<LogEntry>>) {
  LOG_SIGNAL.with(|s| {
    *s.borrow_mut() = Some(set_logs);
  });
}

/// Write log entry (can be called in any context, including JS callbacks)
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

/// CommLog component
#[component]
pub fn CommLog() -> impl IntoView {
  let (logs, set_logs) = signal(Vec::<LogEntry>::new());
  let (hex_only, set_hex_only) = signal(false);

  // Register global log write signal
  register_log_signal(set_logs);

  // Clear log
  let on_clear = move |_| {
    set_logs.set(Vec::new());
  };

  // Toggle hex mode
  let toggle_hex = move |_| {
    set_hex_only.update(|v| *v = !*v);
  };

  view! {
      <section class="card">
          <h2>"Communication Log"</h2>
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
              <button on:click=on_clear>"Clear Log"</button>
              <label>
                  <input
                      type="checkbox"
                      checked=move || hex_only.get()
                      on:change=toggle_hex
                  />
                  " Hex only"
              </label>
          </div>
      </section>
  }
}

/// Global log function - can be called in any context (including BLE callbacks)
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
