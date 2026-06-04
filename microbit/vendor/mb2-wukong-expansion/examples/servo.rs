#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::rtt_init_print;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal::Timer};

use mb2_wukong_expansion::{Servo, ServoAngle, ServoConfig, WuKongBus};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let i2c = board.i2c_external;
    let mut wkb = WuKongBus::new(board.TWIM0, i2c.scl, i2c.sda);
    let s1 = Servo::new(1).unwrap();
    let servo_config = ServoConfig::new([
        (s1, ServoAngle::new(180).unwrap()),
    ]).unwrap();

    let a0 = ServoAngle::new(0).unwrap();
    loop {
        for i in 0..=180 {
            let a = ServoAngle::new(i).unwrap();
            wkb.set_servo_angle(&servo_config, s1, a).unwrap();
            timer.delay_ms(30);
        }
        wkb.set_servo_angle(&servo_config, s1, a0).unwrap();
        timer.delay_ms(1000);
    }
}
