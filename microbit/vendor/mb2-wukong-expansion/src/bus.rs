/*!
Driver for Wukong I2C bus. This bus is used to control
the Mood Lights, Motors and and Servos.
*/

#[cfg(feature = "mood_lights")]
pub mod mood_lights;
#[cfg(feature = "motor")]
pub mod motor;
#[cfg(feature = "servo")]
pub mod servo;

#[cfg(feature = "mood_lights")]
pub use mood_lights::MoodLights;
#[cfg(feature = "motor")]
pub use motor::Motor;
#[cfg(feature = "servo")]
pub use servo::{Servo, ServoAngle, ServoConfig};

use nrf52833_hal::{gpio, pac::twim0, twim};

/// Error during bus operation.
pub enum Error {
    /// I2C error.
    I2c(twim::Error),
    /// Mood light error.
    MoodLight(mood_lights::Error),
    /// Motor error.
    Motor(motor::Error),
    /// Servo error.
    Servo(servo::Error),
}

impl From<twim::Error> for Error {
    fn from(err: twim::Error) -> Self {
        Self::I2c(err)
    }
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::I2c(err) => write!(f, "I2C error: {:?}", err),
            Self::MoodLight(err) => write!(f, "mood light error: {:?}", err),
            Self::Motor(err) => write!(f, "motor error: {:?}", err),
            Self::Servo(err) => write!(f, "servo error: {:?}", err),
        }
    }
}

pub struct WuKongBus<TWIM> {
    i2c: twim::Twim<TWIM>,
}

impl<TWIM> WuKongBus<TWIM>
where
    TWIM: twim::Instance,
{
    pub const I2C_ADDR: u8 = 0x10;

    /// Make a new I2C bus driver. Rquires a TWIM for
    /// `i2c`. Takes ownership of the specific MB2 external
    /// `scl` and `sda` pins, so can only be instantiated
    /// once.
    pub fn new<SclState, SdaState>(
        i2c: TWIM,
        scl: gpio::p0::P0_26<SclState>,
        sda: gpio::p1::P1_00<SdaState>,
    ) -> Self {
        let pins = twim::Pins {
            scl: scl.into_floating_input().degrade(),
            sda: sda.into_floating_input().degrade(),
        };
        let freq = twim0::frequency::FREQUENCY_A::K100;
        let i2c = twim::Twim::new(i2c, pins, freq);
        Self { i2c }
    }
}
