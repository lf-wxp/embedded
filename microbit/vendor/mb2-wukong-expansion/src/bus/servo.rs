/*!
Driver for Wukong servos. The Wukong has eight SVG servo
ports are controlled via the I2C bus. The interface here is
"inspired by" the MicroPython and PXT implementations, but
is a bit more flexible.
*/

use crate::bus;

use nrf52833_hal::twim;

/// Error during servo operation.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// Given servo index out of range.
    InvalidIndex(u8),
    /// Given servo angle out of range.
    InvalidAngle(u16),
    /// Given servo is a repeat in initialization.
    RepeatServo(Servo),
    /// Given servo is accessed while unconfigured.
    UnconfiguredServo(Servo),
    /// Attempted to drive given servo to the given angle,
    /// past its given max angle.
    Overangle(Servo, ServoAngle, ServoAngle),
}

impl From<Error> for bus::Error {
    fn from(error: Error) -> bus::Error {
        bus::Error::Servo(error)
    }
}


/// Servo angle.
#[derive(Debug, Clone, Copy)]
pub struct ServoAngle(u16);

impl ServoAngle {
    /// Make a new servo angle.
    ///
    /// # Errors
    ///
    /// Returns an error if `angle` is not a valid angle in
    /// degrees (0..=359).
    pub fn new(angle: u16) -> Result<Self, Error> {
        angle.try_into()
    }
}

impl From<ServoAngle> for u16 {
    fn from(angle: ServoAngle) -> Self {
        angle.0
    }
}

impl core::convert::TryFrom<u16> for ServoAngle {
    type Error = Error;

    fn try_from(angle: u16) -> Result<Self, Error> {
        if angle >= 360 {
            return Err(Error::InvalidAngle(angle));
        }
        Ok(ServoAngle(angle))
    }
}

/// Servo to be controlled (0..=8).
#[derive(Debug, Clone, Copy)]
pub struct Servo(u8);

impl Servo {
    /// Make a new servo id.  Uses one-based numbering: the
    /// first servo is `1`, not `0`.
    ///
    /// # Errors
    ///
    /// Returns an error when given an out-of-range ID.
    pub fn new(servo: u8) -> Result<Self, Error> {
        servo.try_into()
    }
}

impl From<Servo> for u8 {
    fn from(servo: Servo) -> Self {
        servo.0
    }
}

impl core::convert::TryFrom<u8> for Servo {
    type Error = Error;

    fn try_from(servo: u8) -> Result<Self, Error> {
        if !(1..=8).contains(&servo) {
            return Err(Error::InvalidIndex(servo));
        }
        Ok(Servo(servo - 1))
    }
}

type ServoMaxAngles = [Option<ServoAngle>; 8];

/// Configuration information for servos includes
/// per-servo enablement and max angles.
#[derive(Debug, Clone)]
pub struct ServoConfig {
    servo_max_angles: ServoMaxAngles,
}

impl ServoConfig {
    /// Make a new servo config from an iterator over servos
    /// and their max angles.
    ///
    /// # Errors
    ///
    /// * Returns an error if a servo is repeated in the iterator.
    /// * Returns an error if a max angle is 0°.
    pub fn new<C, I>(config: C) -> Result<Self, bus::Error>
    where
        C: IntoIterator<Item = I>,
        I: Into<(Servo, ServoAngle)>,
    {
        let mut servo_max_angles: ServoMaxAngles = Default::default();
        for item in config.into_iter() {
            let (servo, servo_angle) = item.into();
            let servo_value = u8::from(servo) as usize;
            let servo_angle_value = u16::from(servo_angle);
            if servo_angle_value < 1 {
                return Err(Error::InvalidAngle(servo_angle_value).into());
            }
            if servo_max_angles[servo_value].is_some() {
                return Err(Error::RepeatServo(servo).into());
            }
            servo_max_angles[servo_value] = Some(servo_angle);
        }
        Ok(Self { servo_max_angles })
    }
}

impl<TWIM> bus::WuKongBus<TWIM>
where
    TWIM: twim::Instance,
{
    /// Set the given `servo` to the `given` angle,
    /// taking into account the given `config`.
    ///
    /// # Errors
    ///
    /// * Returns an error if the given servo is not configured.
    /// * Returns an error on an attempt to drive the given servo
    ///   beyond its configured max angle.
    /// * Returns an error if the I2C write fails.
    pub fn set_servo_angle(
        &mut self,
        config: &ServoConfig,
        servo: Servo,
        angle: ServoAngle,
    ) -> Result<(), bus::Error> {
        let servo_value = u8::from(servo);
        let opt_max_angle = config.servo_max_angles[servo_value as usize];
        let max_angle = opt_max_angle.ok_or_else(
            || <bus::Error>::from(Error::UnconfiguredServo(servo))
        )?;
        let max_angle_value = u16::from(max_angle);
        let angle_value = u16::from(angle);
        if angle_value > max_angle_value {
            return Err(Error::Overangle(servo, angle, max_angle).into());
        }
        let scaled_angle = angle_value * 180 / max_angle_value;
        assert!(scaled_angle <= 180);
        let servo_value = servo_value + 3;

        let buf = [servo_value, scaled_angle as u8, 0, 0];
        self.i2c.write(Self::I2C_ADDR, &buf)?;
        Ok(())
    }
}
