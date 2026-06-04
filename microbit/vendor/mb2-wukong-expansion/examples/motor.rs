#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal::Timer};

use mb2_wukong_expansion::{Motor, WuKongBus};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let i2c = board.i2c_external;
    let mut wkb = WuKongBus::new(board.TWIM0, i2c.scl, i2c.sda);
    let m1 = Motor::new(1).unwrap();

    loop {
        for i in [-100, 0, 100] {
            wkb.set_motor_velocity(m1, i).unwrap();
            timer.delay_ms(500);
        }
    }
}
