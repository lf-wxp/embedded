//! micro:bit V2 LED 5x5 matrix driver (row scanning)
//!
//! Pin mapping:
//! - Rows ROW1..ROW5: P0_21, P0_22, P0_15, P0_24, P0_19
//! - Cols COL1..COL5: P0_28, P0_11, P0_31, P1_05, P0_30
//!
//! Lighting logic: row output high + column output low -> that LED lights up.
//! Uses row scanning, each row lit for ~2ms, ~100Hz full frame refresh, no visible flicker.
//!
//! Display frame is controlled by global `DISPLAY_FRAME`, external tasks update content via [`set_frame`].

#![allow(clippy::declare_interior_mutable_const)]

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use defmt::info;
use embassy_nrf::Peri;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::peripherals::{
  P0_11, P0_15, P0_19, P0_21, P0_22, P0_24, P0_28, P0_30, P0_31, P1_05,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

/// LED display frame: uses lower 25 bits of a 32-bit integer to represent 5x5 pixels, bit(row*5 + col) = 1 means lit
static DISPLAY_FRAME: AtomicU32 = AtomicU32::new(0);

/// Brightness mode: when true, use per-LED brightness values instead of binary on/off
static BRIGHTNESS_MODE: AtomicBool = AtomicBool::new(false);

/// Per-LED brightness values (25 bytes, each 0-255)
/// Stored as 25 AtomicU8 for lock-free access
static BRIGHTNESS: [AtomicU8; 25] = {
  const INIT: AtomicU8 = AtomicU8::new(0);
  [INIT; 25]
};

/// Signal to trigger scroll text display
static SCROLL_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Scroll text buffer (max 60 chars)
static mut SCROLL_BUF: [u8; 60] = [0u8; 60];
static SCROLL_LEN: AtomicU8 = AtomicU8::new(0);
static SCROLLING: AtomicBool = AtomicBool::new(false);

/// LED pin set (passed when spawning task)
pub struct LedPins {
  pub row1: Peri<'static, P0_21>,
  pub row2: Peri<'static, P0_22>,
  pub row3: Peri<'static, P0_15>,
  pub row4: Peri<'static, P0_24>,
  pub row5: Peri<'static, P0_19>,
  pub col1: Peri<'static, P0_28>,
  pub col2: Peri<'static, P0_11>,
  pub col3: Peri<'static, P0_31>,
  pub col4: Peri<'static, P1_05>,
  pub col5: Peri<'static, P0_30>,
}

/// Set entire frame display data (25 bytes, each byte 0=off, non-zero=on)
pub fn set_frame_from_bytes(bytes: &[u8]) {
  let mut frame: u32 = 0;
  let n = bytes.len().min(25);
  for (i, &b) in bytes[..n].iter().enumerate() {
    if b != 0 {
      frame |= 1 << i;
    }
  }
  BRIGHTNESS_MODE.store(false, Ordering::Relaxed);
  DISPLAY_FRAME.store(frame, Ordering::Relaxed);
}

/// Set per-LED brightness values (25 bytes, each 0-255)
pub fn set_brightness_from_bytes(bytes: &[u8]) {
  let n = bytes.len().min(25);
  for (i, &b) in bytes[..n].iter().enumerate() {
    BRIGHTNESS[i].store(b, Ordering::Relaxed);
  }
  // Also update the binary frame for compatibility
  let mut frame: u32 = 0;
  for (i, b) in BRIGHTNESS.iter().enumerate() {
    if b.load(Ordering::Relaxed) > 0 {
      frame |= 1 << i;
    }
  }
  BRIGHTNESS_MODE.store(true, Ordering::Relaxed);
  DISPLAY_FRAME.store(frame, Ordering::Relaxed);
  info!("LED brightness mode set");
}

/// Set display frame directly with 25-bit bitmap
pub fn set_frame_bitmap(bitmap: u32) {
  BRIGHTNESS_MODE.store(false, Ordering::Relaxed);
  DISPLAY_FRAME.store(bitmap & 0x01FF_FFFF, Ordering::Relaxed);
}

/// Clear display
pub fn clear_frame() {
  BRIGHTNESS_MODE.store(false, Ordering::Relaxed);
  DISPLAY_FRAME.store(0, Ordering::Relaxed);
  for b in BRIGHTNESS.iter() {
    b.store(0, Ordering::Relaxed);
  }
}

/// Show a simple pattern: display 5x5 character based on ASCII (supports limited subset, simplified version)
pub fn show_char(c: u8) {
  let bitmap = font_5x5(c);
  BRIGHTNESS_MODE.store(false, Ordering::Relaxed);
  DISPLAY_FRAME.store(bitmap, Ordering::Relaxed);
}

/// Start scrolling text on the LED matrix
/// The text will be displayed character by character with a scrolling animation
pub fn scroll_text(text: &[u8]) {
  let len = text.len().min(60);
  unsafe {
    SCROLL_BUF[..len].copy_from_slice(&text[..len]);
  }
  SCROLL_LEN.store(len as u8, Ordering::Relaxed);
  SCROLL_SIGNAL.signal(());
  info!("Scroll text queued, {} chars", len);
}

/// Check if currently scrolling
pub fn is_scrolling() -> bool {
  SCROLLING.load(Ordering::Relaxed)
}

/// Minimal 5x5 font table, covering ASCII digits, letters, a few symbols; returns smiley pattern for other characters
fn font_5x5(c: u8) -> u32 {
  // Each row uses 5 bits, packed from row0 into 25-bit bitmap
  // pack(rows) packs 5 u8s (lower 5 bits each) into bitmap
  const fn pack(r: [u8; 5]) -> u32 {
    let mut v: u32 = 0;
    let mut i = 0;
    while i < 5 {
      v |= (r[i] as u32 & 0x1F) << (i * 5);
      i += 1;
    }
    v
  }
  match c {
    b'0' => pack([0b01110, 0b10001, 0b10001, 0b10001, 0b01110]),
    b'1' => pack([0b00100, 0b01100, 0b00100, 0b00100, 0b01110]),
    b'2' => pack([0b01110, 0b10001, 0b00010, 0b00100, 0b11111]),
    b'3' => pack([0b11110, 0b00001, 0b01110, 0b00001, 0b11110]),
    b'4' => pack([0b00010, 0b00110, 0b01010, 0b11111, 0b00010]),
    b'5' => pack([0b11111, 0b10000, 0b11110, 0b00001, 0b11110]),
    b'6' => pack([0b00110, 0b01000, 0b11110, 0b10001, 0b01110]),
    b'7' => pack([0b11111, 0b00001, 0b00010, 0b00100, 0b00100]),
    b'8' => pack([0b01110, 0b10001, 0b01110, 0b10001, 0b01110]),
    b'9' => pack([0b01110, 0b10001, 0b01111, 0b00010, 0b01100]),
    b'A' | b'a' => pack([0b01110, 0b10001, 0b11111, 0b10001, 0b10001]),
    b'B' | b'b' => pack([0b11110, 0b10001, 0b11110, 0b10001, 0b11110]),
    b'C' | b'c' => pack([0b01110, 0b10001, 0b10000, 0b10001, 0b01110]),
    b'H' | b'h' => pack([0b10001, 0b10001, 0b11111, 0b10001, 0b10001]),
    b'I' | b'i' => pack([0b01110, 0b00100, 0b00100, 0b00100, 0b01110]),
    b'O' | b'o' => pack([0b01110, 0b10001, 0b10001, 0b10001, 0b01110]),
    b'!' => pack([0b00100, 0b00100, 0b00100, 0b00000, 0b00100]),
    b'?' => pack([0b01110, 0b10001, 0b00110, 0b00000, 0b00100]),
    b' ' => 0,
    // Default smiley face
    _ => pack([0b00000, 0b01010, 0b00000, 0b10001, 0b01110]),
  }
}

/// LED matrix refresh task: continuously display [`DISPLAY_FRAME`] content using row scanning
///
/// Supports two modes:
/// - Binary mode: LED is either on or off (standard row scanning)
/// - Brightness mode: LED brightness is controlled by varying on-time within each row period
///
/// Also handles scroll text requests via [`SCROLL_SIGNAL`]
#[embassy_executor::task]
pub async fn led_matrix_task(pins: LedPins) {
  // Initialize GPIO: rows/cols all output, starting at low level (all LEDs off)
  let mut rows: [Output<'static>; 5] = [
    Output::new(pins.row1, Level::Low, OutputDrive::Standard),
    Output::new(pins.row2, Level::Low, OutputDrive::Standard),
    Output::new(pins.row3, Level::Low, OutputDrive::Standard),
    Output::new(pins.row4, Level::Low, OutputDrive::Standard),
    Output::new(pins.row5, Level::Low, OutputDrive::Standard),
  ];
  let mut cols: [Output<'static>; 5] = [
    Output::new(pins.col1, Level::High, OutputDrive::Standard),
    Output::new(pins.col2, Level::High, OutputDrive::Standard),
    Output::new(pins.col3, Level::High, OutputDrive::Standard),
    Output::new(pins.col4, Level::High, OutputDrive::Standard),
    Output::new(pins.col5, Level::High, OutputDrive::Standard),
  ];

  loop {
    // Check for scroll text request (non-blocking)
    if SCROLL_SIGNAL.signaled() {
      SCROLL_SIGNAL.reset();
      SCROLLING.store(true, Ordering::Relaxed);

      let len = SCROLL_LEN.load(Ordering::Relaxed) as usize;
      // Display each character for ~500ms
      for &c in unsafe { &SCROLL_BUF[..len] } {
        let bitmap = font_5x5(c);
        DISPLAY_FRAME.store(bitmap, Ordering::Relaxed);
        BRIGHTNESS_MODE.store(false, Ordering::Relaxed);

        // Display this character for 500ms (50 refresh cycles at 10ms each)
        for _ in 0..50 {
          display_one_frame(&mut rows, &mut cols).await;
        }

        // Brief blank between characters
        DISPLAY_FRAME.store(0, Ordering::Relaxed);
        for _ in 0..5 {
          display_one_frame(&mut rows, &mut cols).await;
        }
      }

      SCROLLING.store(false, Ordering::Relaxed);
      DISPLAY_FRAME.store(0, Ordering::Relaxed);
    }

    // Normal display refresh
    display_one_frame(&mut rows, &mut cols).await;
  }
}

/// Display one complete frame (all 5 rows scanned once)
async fn display_one_frame(rows: &mut [Output<'static>; 5], cols: &mut [Output<'static>; 5]) {
  let frame = DISPLAY_FRAME.load(Ordering::Relaxed);
  let brightness_mode = BRIGHTNESS_MODE.load(Ordering::Relaxed);

  for r in 0..5 {
    // Turn off all columns (high = not conducting)
    for c in cols.iter_mut() {
      c.set_high();
    }
    // Select current row (high = powered)
    for (i, row_pin) in rows.iter_mut().enumerate() {
      if i == r {
        row_pin.set_high();
      } else {
        row_pin.set_low();
      }
    }

    if brightness_mode {
      // Brightness mode: use time-division to simulate brightness levels
      // Total row time = 2ms, divide into 8 time slots
      // LED with brightness > threshold is lit during that slot
      for slot in 0..8u8 {
        let threshold = slot * 32; // 0, 32, 64, 96, 128, 160, 192, 224
        for (c_idx, col) in cols.iter_mut().enumerate() {
          let bit_idx = r * 5 + c_idx;
          let br = BRIGHTNESS[bit_idx].load(Ordering::Relaxed);
          if br > threshold {
            col.set_low(); // LED on
          } else {
            col.set_high(); // LED off
          }
        }
        Timer::after_micros(250).await; // 8 slots × 250μs = 2ms per row
      }
    } else {
      // Binary mode: simple on/off
      for (c_idx, col) in cols.iter_mut().enumerate() {
        let bit_idx = r * 5 + c_idx;
        if frame & (1 << bit_idx) != 0 {
          col.set_low();
        }
      }
      // This row lit for ~2ms (5 rows ~ 10ms period ~ 100Hz)
      Timer::after_millis(2).await;
    }
  }

  // Turn off frame after scanning to avoid residual on last row
  for row_pin in rows.iter_mut() {
    row_pin.set_low();
  }
  for c in cols.iter_mut() {
    c.set_high();
  }
}
