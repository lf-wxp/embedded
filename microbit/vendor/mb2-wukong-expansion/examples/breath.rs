#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal};

use mb2_wukong_expansion::{MoodLights, WuKongBus};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut delay = hal::Timer::new(board.TIMER0);
    let i2c = board.i2c_external;
    let mut wukong = WuKongBus::new(board.TWIM0, i2c.scl, i2c.sda);

    loop {
        wukong
            .set_mood_lights(&mut delay, MoodLights::Breath)
            .unwrap();
        delay.delay_ms(4000);
        for intensity in (0..=100).step_by(10) {
            wukong
                .set_mood_lights(&mut delay, MoodLights::Intensity(intensity))
                .unwrap();
            delay.delay_ms(1000);
        }
        wukong.set_mood_lights(&mut delay, MoodLights::Off).unwrap();
        delay.delay_ms(500);
    }
}
