/*!
Interface for Wukong DC motor drivers. The Wukong has two DC
motor outputs that are controlled by the I2C bus. The
interface here mostly follows that of the MicroPython and
PXT implementations.
*/

use crate::bus;

use nrf52833_hal::twim;

/// Motor operation error.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// Requested motor does not exist.
    InvalidIndex(u8),
    /// Requested absolute motor speed too large.
    Overspeed(i8),
}

impl From<Error> for bus::Error {
    fn from(error: Error) -> bus::Error {
        bus::Error::Motor(error)
    }
}

/// Motor to be controlled.
#[derive(Debug, Clone, Copy)]
pub struct Motor(u8);

impl Motor {
    /// Make a new motor id.  Uses one-based numbering: the
    /// first motor is `1`, not `0`.
    ///
    /// # Errors
    ///
    /// Returns an error when given an out-of-range ID.
    pub fn new(motor: u8) -> Result<Self, Error> {
        motor.try_into()
    }
}

impl From<Motor> for u8 {
    fn from(motor: Motor) -> Self {
        motor.0
    }
}

impl core::convert::TryFrom<u8> for Motor {
    type Error = Error;

    fn try_from(motor: u8) -> Result<Self, Error> {
        if !(1..=2).contains(&motor) {
            return Err(Error::InvalidIndex(motor));
        }
        Ok(Motor(motor - 1))
    }
}

impl<TWIM> bus::WuKongBus<TWIM>
where
    TWIM: twim::Instance,
{
    /// Set the given `motor` to the given rotational `velocity` (-100..=100).
    ///
    /// # Errors
    ///
    /// Returns an error if the I2C write fails.
    pub fn set_motor_velocity(&mut self, motor: Motor, velocity: i8) -> Result<(), bus::Error> {
        if !(-100..=100).contains(&velocity) {
            return Err(Error::Overspeed(velocity).into());
        }
        let motor_value = u8::from(motor) + 1;
        let sign = if velocity >= 0 { 1 } else { 2 };
        let speed = velocity.unsigned_abs();
        let buf = [motor_value, sign, speed, 0];
        self.i2c.write(Self::I2C_ADDR, &buf)?;
        Ok(())
    }
}
