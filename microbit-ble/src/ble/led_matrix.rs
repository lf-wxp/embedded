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

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_nrf::Peri;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::peripherals::{
  P0_11, P0_15, P0_19, P0_21, P0_22, P0_24, P0_28, P0_30, P0_31, P1_05,
};
use embassy_time::Timer;

/// LED display frame: uses lower 25 bits of a 32-bit integer to represent 5x5 pixels, bit(row*5 + col) = 1 means lit
static DISPLAY_FRAME: AtomicU32 = AtomicU32::new(0);

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
  DISPLAY_FRAME.store(frame, Ordering::Relaxed);
}

/// Set display frame directly with 25-bit bitmap
pub fn set_frame_bitmap(bitmap: u32) {
  DISPLAY_FRAME.store(bitmap & 0x01FF_FFFF, Ordering::Relaxed);
}

/// Clear display
pub fn clear_frame() {
  DISPLAY_FRAME.store(0, Ordering::Relaxed);
}

/// Show a simple pattern: display 5x5 character based on ASCII (supports limited subset, simplified version)
pub fn show_char(c: u8) {
  let bitmap = font_5x5(c);
  DISPLAY_FRAME.store(bitmap, Ordering::Relaxed);
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
    let frame = DISPLAY_FRAME.load(Ordering::Relaxed);

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
      // Set columns to light up for this row (low = conducting)
      for c_idx in 0..5 {
        let bit_idx = r * 5 + c_idx;
        if frame & (1 << bit_idx) != 0 {
          cols[c_idx].set_low();
        }
      }
      // This row lit for ~2ms (5 rows ~ 10ms period ~ 100Hz)
      Timer::after_millis(2).await;
    }

    // Turn off frame after scanning to avoid residual on last row
    for row_pin in rows.iter_mut() {
      row_pin.set_low();
    }
    for c in cols.iter_mut() {
      c.set_high();
    }
  }
}
