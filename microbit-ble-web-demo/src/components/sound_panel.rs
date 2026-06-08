//! Sound/buzzer control component
//! Play tones on micro:bit V2 buzzer via BLE commands

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

/// Musical note with frequency (Hz)
struct Note {
  name: &'static str,
  freq: u16,
}

/// Predefined notes (octave 4)
const NOTES: [Note; 13] = [
  Note {
    name: "C4",
    freq: 262,
  },
  Note {
    name: "C#4",
    freq: 277,
  },
  Note {
    name: "D4",
    freq: 294,
  },
  Note {
    name: "D#4",
    freq: 311,
  },
  Note {
    name: "E4",
    freq: 330,
  },
  Note {
    name: "F4",
    freq: 349,
  },
  Note {
    name: "F#4",
    freq: 370,
  },
  Note {
    name: "G4",
    freq: 392,
  },
  Note {
    name: "G#4",
    freq: 415,
  },
  Note {
    name: "A4",
    freq: 440,
  },
  Note {
    name: "A#4",
    freq: 466,
  },
  Note {
    name: "B4",
    freq: 494,
  },
  Note {
    name: "C5",
    freq: 523,
  },
];

/// Predefined melodies: each entry is (note_index, duration_ms)
type MelodyNote = (usize, u16);

const MELODY_TWINKLE: [MelodyNote; 12] = [
  (0, 300),
  (0, 300),
  (4, 300),
  (4, 300),
  (5, 300),
  (5, 300),
  (4, 500),
  (3, 300),
  (3, 300),
  (2, 300),
  (2, 300),
  (0, 500),
];

const MELODY_SCALE: [MelodyNote; 8] = [
  (0, 250),
  (2, 250),
  (4, 250),
  (5, 250),
  (7, 250),
  (9, 250),
  (11, 250),
  (12, 400),
];

/// Play a single tone
fn play_tone(freq: u16, duration_ms: u16, connected: bool) {
  if !connected {
    return;
  }
  let payload = freq
    .to_le_bytes()
    .iter()
    .chain(duration_ms.to_le_bytes().iter())
    .copied()
    .collect::<Vec<u8>>();
  match build_frame(Command::SoundPlay as u8, &payload) {
    Ok(frame) => {
      log_tx(
        format!("SoundPlay {}Hz {}ms", freq, duration_ms),
        Some(frame.clone()),
      );
      ble_send_frame(frame);
    }
    Err(e) => log_error(format!("Build frame failed: {e}")),
  }
}

/// Stop playing tone
fn stop_tone(connected: bool) {
  if !connected {
    return;
  }
  match build_frame(Command::SoundStop as u8, &[]) {
    Ok(frame) => {
      log_tx("SoundStop".to_string(), Some(frame.clone()));
      ble_send_frame(frame);
    }
    Err(e) => log_error(format!("Build frame failed: {e}")),
  }
}

/// Play a melody by sending notes sequentially with delays
fn play_melody(melody: &[MelodyNote], connected: bool) {
  if !connected {
    return;
  }
  for (i, (note_idx, duration_ms)) in melody.iter().enumerate() {
    let freq = NOTES.get(*note_idx).map(|n| n.freq).unwrap_or(440);
    let dur = *duration_ms;
    let total_delay = dur as u32 + 80; // small gap between notes
    // Schedule each note with increasing delay
    let connected_clone = connected;
    spawn_local(async move {
      gloo_timers::future::TimeoutFuture::new(i as u32 * total_delay).await;
      play_tone(freq, dur, connected_clone);
    });
  }
}

/// SoundPanel component
#[component]
pub fn SoundPanel() -> impl IntoView {
  let app_state = expect_context::<AppState>();
  let connected = app_state.connected;

  // Custom frequency and duration inputs
  let (freq_input, set_freq_input) = signal("440".to_string());
  let (duration_input, set_duration_input) = signal("500".to_string());

  // Play custom tone
  let on_play_custom = move |_| {
    let freq = freq_input.get().parse::<u16>().unwrap_or(440);
    let dur = duration_input.get().parse::<u16>().unwrap_or(500);
    play_tone(freq, dur, connected.get());
  };

  // Stop
  let on_stop = move |_| {
    stop_tone(connected.get());
  };

  // Play melodies
  let on_twinkle = move |_| {
    play_melody(&MELODY_TWINKLE, connected.get());
  };
  let on_scale = move |_| {
    play_melody(&MELODY_SCALE, connected.get());
  };

  view! {
      <section class="card">
          <h2>"🎵 Sound / Buzzer"</h2>

          // Piano-style note buttons
          <div class="note-grid">
              <For
                  each=move || NOTES.iter().enumerate()
                  key=|(i, _)| *i
                  children=move |(_, note)| {
                      let is_sharp = note.name.contains('#');
                      let freq = note.freq;
                      let class = move || {
                          let mut c = "note-btn".to_string();
                          if is_sharp { c.push_str(" sharp"); }
                          c
                      };
                      let onclick = move |_| {
                          play_tone(freq, 300, connected.get());
                      };
                      view! {
                          <button class=class disabled=move || !connected.get() on:click=onclick>
                              {note.name}
                          </button>
                      }
                  }
              />
          </div>

          // Custom tone controls
          <div class="row" style="margin-top: 12px;">
              <input
                  type="text"
                  class="short-input"
                  placeholder="Frequency (Hz)"
                  value="440"
                  on:input=move |ev| set_freq_input.set(event_target_value(&ev))
              />
              <input
                  type="text"
                  class="short-input"
                  placeholder="Duration (ms)"
                  value="500"
                  on:input=move |ev| set_duration_input.set(event_target_value(&ev))
              />
              <button disabled=move || !connected.get() on:click=on_play_custom>
                  "▶ Play Tone"
              </button>
              <button disabled=move || !connected.get() on:click=on_stop class="danger">
                  "⏹ Stop"
              </button>
          </div>

          // Melody buttons
          <div class="row" style="margin-top: 10px;">
              <button disabled=move || !connected.get() on:click=on_twinkle>"🎶 Twinkle"</button>
              <button disabled=move || !connected.get() on:click=on_scale>"🎹 Scale"</button>
          </div>

          <p class="hint">"Click a note to play a 300ms tone, or set custom frequency/duration. Melodies play notes sequentially."</p>
      </section>
  }
}
