#![no_main]
#![no_std]

use cortex_m_rt::entry;
use mb2_wukong_expansion::WuKongBuzzer;
use microbit::{board::Board, hal::gpio::Level};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

// The following are the score data for "Twinkle Twinkle Little Star"
// Note (MIDI number) and duration (milliseconds)
const NOTE_MIDI_C4: u8 = 60; // Middle C (Do)
const NOTE_MIDI_D4: u8 = 62; // Re
const NOTE_MIDI_E4: u8 = 64; // Mi
const NOTE_MIDI_F4: u8 = 65; // Fa
const NOTE_MIDI_G4: u8 = 67; // Sol
const NOTE_MIDI_A4: u8 = 69; // La
const NOTE_MIDI_B4: u8 = 71; // Si
const NOTE_MIDI_C5: u8 = 72; // High C (Do)

// Duration constants (milliseconds)
const QUARTER_NOTE: u32 = 500; // Quarter note, one beat
const HALF_NOTE: u32 = 1000; // Half note, two beats

// The entire score is a tuple array: (MIDI note number, duration)
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
  // Initialize buzzer driver, connected to P0 pin on WuKong board
  let pin = board.edge.e00.into_push_pull_output(Level::Low);
  let mut buzzer = WuKongBuzzer::new(board.PWM0, pin);

  for &(note, duration) in SCORE.iter() {
    buzzer.play_note(note, duration);
    rprintln!("Playing note: MIDI {}, duration {} ms", note, duration);
  }

  rprintln!("Song finished!");
  loop {}
}
