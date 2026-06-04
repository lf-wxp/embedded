/*!
Driver for the Wukong "mood lights". These are blue LEDs on
the bottom of the Wukong board that are controlled via
the I2C bus. The LEDs move in unison, and can be set either
to "breathe" ("breath" in the Wukong documentation) or to a
specific intensity. The interface here mostly follows that
of the MicroPython and PXT implementations.
*/

use crate::bus;

use embedded_hal::delay;
use nrf52833_hal::twim;

/// Error in mood light operation.
#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// Attempted to set intensity too high.
    Overintensity(u8),
}

impl From<Error> for bus::Error {
    fn from(error: Error) -> bus::Error {
        bus::Error::MoodLight(error)
    }
}

/// Modes for the mood lights.
#[derive(Debug, Clone, Copy)]
pub enum MoodLights {
    /// Turned off (default).
    Off,
    /// "Breathing" with a period of a couple of seconds.
    Breath,
    /// On with given intensity (0..=100).
    Intensity(u8),
}

impl<TWIM> bus::WuKongBus<TWIM>
where
    TWIM: twim::Instance,
{
    /// Set the `mood_lights` to the given mode. A `delay` unit must
    /// be borrowed to properly implement the protocol.
    ///
    /// # Errors
    ///
    /// Returns an error if an I2C write fails.
    pub fn set_mood_lights<Delay>(
        &mut self,
        delay: &mut Delay,
        mood_lights: MoodLights,
    ) -> Result<(), bus::Error>
    where
        Delay: delay::DelayNs,
    {
        match mood_lights {
            MoodLights::Breath => {
                let buf = [0x11, 0, 0, 0];
                self.i2c.write(Self::I2C_ADDR, &buf)?;

                delay.delay_ms(100);

                let buf = [0x12, 150, 0, 0];
                self.i2c.write(Self::I2C_ADDR, &buf)?;
            }
            mood_lights => {
                let intensity = match mood_lights {
                    MoodLights::Off => 0,
                    MoodLights::Intensity(intensity) => {
                        if intensity > 100 {
                            return Err(Error::Overintensity(intensity).into());
                        }
                        intensity
                    }
                    MoodLights::Breath => unreachable!(),
                };
                let buf = [0x12, intensity, 0, 0];
                self.i2c.write(Self::I2C_ADDR, &buf)?;

                delay.delay_ms(100);

                let buf = [0x11, 160, 0, 0];
                self.i2c.write(Self::I2C_ADDR, &buf)?;
            }
        }
        Ok(())
    }
}
