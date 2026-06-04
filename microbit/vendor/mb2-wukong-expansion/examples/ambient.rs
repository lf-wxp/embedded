#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal::Timer};

use mb2_wukong_expansion::{WuKongAmbient, RGB8};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut delay = Timer::new(board.TIMER0);
    let mut wka = WuKongAmbient::new(board.PWM0, board.edge.e16).unwrap();
    let mut red = 0u8;
    loop {
        let rgb = RGB8::new(red, 64, 64);
        wka.set_color(3, rgb).unwrap();
        delay.delay_ms(20);
        red = red.wrapping_add(1);
    }
}
