#![no_main]
#![no_std]

use cortex_m_rt::entry;
use mb2_wukong_expansion::WuKongBuzzer;
use microbit::{board::Board, hal::gpio::Level};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

// 以下是《小星星》的曲谱数据
// 音符 (MIDI编号) 和 时值 (毫秒)
const NOTE_MIDI_C4: u8 = 60; // 中音Do
const NOTE_MIDI_D4: u8 = 62; // Re
const NOTE_MIDI_E4: u8 = 64; // Mi
const NOTE_MIDI_F4: u8 = 65; // Fa
const NOTE_MIDI_G4: u8 = 67; // Sol
const NOTE_MIDI_A4: u8 = 69; // La
const NOTE_MIDI_B4: u8 = 71; // Si
const NOTE_MIDI_C5: u8 = 72; // 高音Do

// 时值常量 (毫秒)
const QUARTER_NOTE: u32 = 500; // 四分音符，一拍
const HALF_NOTE: u32 = 1000; // 二分音符，两拍

// 整个乐谱是一个元组数组: (MIDI音符编号, 时值)
const SCORE: [(u8, u32); 20] = [
  (NOTE_MIDI_C4, QUARTER_NOTE),
  (NOTE_MIDI_C4, QUARTER_NOTE),
  (NOTE_MIDI_G4, QUARTER_NOTE),
  (NOTE_MIDI_G4, QUARTER_NOTE),
  (NOTE_MIDI_A4, QUARTER_NOTE),
  (NOTE_MIDI_A4, QUARTER_NOTE),
  (NOTE_MIDI_G4, HALF_NOTE),
  (NOTE_MIDI_F4, QUARTER_NOTE),
  (NOTE_MIDI_F4, QUARTER_NOTE),
  (NOTE_MIDI_E4, QUARTER_NOTE),
  (NOTE_MIDI_E4, QUARTER_NOTE),
  (NOTE_MIDI_D4, QUARTER_NOTE),
  (NOTE_MIDI_D4, QUARTER_NOTE),
  (NOTE_MIDI_C4, HALF_NOTE),
  (NOTE_MIDI_G4, QUARTER_NOTE),
  (NOTE_MIDI_G4, QUARTER_NOTE),
  (NOTE_MIDI_F4, QUARTER_NOTE),
  (NOTE_MIDI_F4, QUARTER_NOTE),
  (NOTE_MIDI_E4, QUARTER_NOTE),
  (NOTE_MIDI_E4, QUARTER_NOTE),
];

#[entry]
fn main() -> ! {
  rtt_init_print!();
  rprintln!("Starting to play 'Twinkle Twinkle Little Star'...");

  let board = Board::take().unwrap();
  // 初始化蜂鸣器驱动，连接到悟空板上的 P0 引脚
  let pin = board.edge.e00.into_push_pull_output(Level::Low);
  let mut buzzer = WuKongBuzzer::new(board.PWM0, pin);

  for &(note, duration) in SCORE.iter() {
    buzzer.play_note(note, duration);
    rprintln!("Playing note: MIDI {}, duration {} ms", note, duration);
  }

  rprintln!("Song finished!");
  loop {}
}
