![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![crates-io](https://img.shields.io/crates/v/mb2-wukong-expansion.svg)](https://crates.io/crates/mb2-wukong-expansion)
[![api-docs](https://docs.rs/mb2-wukong-expansion/badge.svg)](https://docs.rs/mb2-wukong-expansion)
[![dependency-status](https://deps.rs/repo/github/BartMassey/mb2-wukong-expansion/status.svg)](https://deps.rs/repo/github/BartMassey/mb2-wukong-expansion)

# mb2-wukong-expansion: Rust for the Elecfreaks Wukong Expansion Board for the BBC micro:bit v2
Copyright © 2024 Bart Massey (Version 0.1.2)


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

# License

This work is licensed under the "MIT License". Please see the file
`LICENSE.txt` in this distribution for license terms.

# Acknowledgments

Thanks to Elecfreaks for doing this board, and to the folks
who wrote the Micropython and PTX Javascript that I cribbed
the I2C protocol from.

# Spelling

Is it "WuKong" or "Wukong"? Elecfreaks seems to randomly
switch between the two capitalizations in their products, so
I won't worry about it much: I haven't been consistent
either.
