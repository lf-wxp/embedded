#![doc(html_root_url = "https://docs.rs/mb2-wukong-expansion/0.1.2")]
/*!

This Rust crate provides support for the [Elecfreaks Wukong
Expansion
Board](https://shop.elecfreaks.com/products/elecfreaks-micro-bit-wukong-expansion-board-adapter)
(Wukong; see also their
[wiki](https://www.elecfreaks.com/learn-en/microbitExtensionModule/wukong.html))
for the [BBC micro:bit
v2](https://microbit.org/new-microbit/) (MB2).

This crate is currently built atop `nrf52833-hal` and is
probably best used with that.

The Wukong provides a rechargeable battery that can power
itself and the MB2, and provides expansion pins for 5V and
for the MB2 edge connector.

The Wukong also provides five mostly-disjoint features
visible from the MB2. Each is supported by a separate Cargo
feature listed here. (All features are on by default, but
you can turn off the ones you don't want to save a little
space.) The names were mostly taken from the Wukong
documentation.

* "*Ambient*" LEDs (`ambient`): Four WS2812 RGB "Smart LEDs" sit at the
  four corners of the Wukong. This crate will drive these LEDs as if
  they were directly addressable.

* "*Buzzer*" (`buzzer`): A speaker sits on the bottom of the
  board. This crate will play a square wave at a given
  frequency on this speaker.

* *Mood Lights* (`mood_lights`): There are blue LEDs under
  the board that are cooperatively controlled by the Wukong
  and the MB2. (These are referred to as "Breath" in the
  Wukong documentation, but they can be put in steady-on
  mode as well.) This crate can run these.

* *Motor* (`motor`): The Wukong has two DC motor controllers
  with pins on the board. This crate can set the speed of
  these motors.

* *Servo* (`servo`): The Wukong has eight servo controllers
  with pins on the board. This crate can set the angle of
  these servos.
*/

#![no_std]

#[cfg(feature = "ambient")]
pub mod ambient;
#[cfg(feature = "bus")]
pub mod bus;
#[cfg(feature = "buzzer")]
pub mod buzzer;

#[cfg(feature = "ambient")]
pub use ambient::{WuKongAmbient, RGB8};
#[cfg(feature = "buzzer")]
pub use buzzer::WuKongBuzzer;

#[cfg(feature = "bus")]
pub use bus::WuKongBus;
#[cfg(feature = "mood_lights")]
pub use bus::MoodLights;
#[cfg(feature = "motor")]
pub use bus::Motor;
#[cfg(feature = "servo")]
pub use bus::{Servo, ServoAngle, ServoConfig};
