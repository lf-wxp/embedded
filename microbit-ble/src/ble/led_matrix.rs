//! micro:bit V2 LED 5x5 矩阵驱动（行扫描）
//!
//! 引脚映射：
//! - 行 ROW1..ROW5: P0_21, P0_22, P0_15, P0_24, P0_19
//! - 列 COL1..COL5: P0_28, P0_11, P0_31, P1_05, P0_30
//!
//! 点亮逻辑：行输出高电平 + 列输出低电平 -> 该 LED 点亮。
//! 采用行扫描方式，每行点亮约 2ms，~100Hz 全幅刷新，肉眼无明显闪烁。
//!
//! 显示帧由全局 `DISPLAY_FRAME` 控制，外部任务通过 [`set_frame`] 更新内容。

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_nrf::Peri;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::peripherals::{
  P0_11, P0_15, P0_19, P0_21, P0_22, P0_24, P0_28, P0_30, P0_31, P1_05,
};
use embassy_time::Timer;

/// LED 显示帧：用 32 位整数低 25 位表示 5x5 像素，bit(row*5 + col) = 1 表示点亮
static DISPLAY_FRAME: AtomicU32 = AtomicU32::new(0);

/// LED 引脚集合（用于 spawn task 时传入）
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

/// 设置整帧显示数据（25 字节，每字节 0=灭、非 0=亮）
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

/// 直接以 25 位 bitmap 设置显示帧
pub fn set_frame_bitmap(bitmap: u32) {
  DISPLAY_FRAME.store(bitmap & 0x01FF_FFFF, Ordering::Relaxed);
}

/// 清空显示
pub fn clear_frame() {
  DISPLAY_FRAME.store(0, Ordering::Relaxed);
}

/// 显示一个简单图案：根据 ASCII 字符显示 5x5 字符（仅支持有限子集，简化版）
pub fn show_char(c: u8) {
  let bitmap = font_5x5(c);
  DISPLAY_FRAME.store(bitmap, Ordering::Relaxed);
}

/// 极简 5x5 字体表，覆盖 ASCII 数字、字母、几个符号；其它字符返回笑脸图案
fn font_5x5(c: u8) -> u32 {
  // 每行用 5 位表示，从 row0 开始打包到 25 位 bitmap
  // pack(rows) 把 5 个 u8（每个低 5 位）打成 bitmap
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
    // 默认笑脸
    _ => pack([0b00000, 0b01010, 0b00000, 0b10001, 0b01110]),
  }
}

/// LED 矩阵刷新任务：以行扫描方式持续显示 [`DISPLAY_FRAME`] 内容
#[embassy_executor::task]
pub async fn led_matrix_task(pins: LedPins) {
  // 初始化 GPIO：行/列均输出，低电平起始（关闭所有 LED）
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
      // 关掉所有列（高 = 不导通）
      for c in cols.iter_mut() {
        c.set_high();
      }
      // 选中当前行（高 = 给电）
      for (i, row_pin) in rows.iter_mut().enumerate() {
        if i == r {
          row_pin.set_high();
        } else {
          row_pin.set_low();
        }
      }
      // 设置该行需要点亮的列（低 = 导通）
      for c_idx in 0..5 {
        let bit_idx = r * 5 + c_idx;
        if frame & (1 << bit_idx) != 0 {
          cols[c_idx].set_low();
        }
      }
      // 该行点亮约 2ms（5 行 ~ 10ms 周期 ~ 100Hz）
      Timer::after_millis(2).await;
    }

    // 扫描结束后熄灭一帧避免最后一行残留
    for row_pin in rows.iter_mut() {
      row_pin.set_low();
    }
    for c in cols.iter_mut() {
      c.set_high();
    }
  }
}
