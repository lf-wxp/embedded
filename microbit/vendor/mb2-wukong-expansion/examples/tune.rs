#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use cortex_m_rt::entry;
use microbit::{board::Board, hal::gpio::Level};

use mb2_wukong_expansion::WuKongBuzzer;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let pin = board.edge.e00.into_push_pull_output(Level::Low);
    let mut wkb = WuKongBuzzer::new(board.PWM0, pin);
    let scale = [72, 74, 76, 77, 79, 81, 83, 84];
    loop {
        for s in scale {
            wkb.play_note(s, 125);
        }
    }
}
