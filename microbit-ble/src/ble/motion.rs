//! micro:bit V2 motion sensors (accelerometer + magnetometer)
//!
//! The micro:bit V2 has an LSM303AGR sensor on the internal I2C bus:
//! - SDA: P0_16
//! - SCL: P0_08
//! - Accelerometer I2C address: 0x19
//! - Magnetometer I2C address: 0x1E
//!
//! This module provides:
//! - [`motion_task`] background task that periodically reads sensor data when subscribed
//! - [`ACCEL_EVENTS`] channel for accelerometer data
//! - [`MAGNET_EVENTS`] channel for magnetometer data
//! - [`set_accel_subscribed`] / [`set_magnet_subscribed`] subscription control

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::{info, warn};
use embassy_nrf::Peri;
use embassy_nrf::bind_interrupts;
use embassy_nrf::peripherals::{P0_08, P0_16, TWISPI0};
use embassy_nrf::twim::{self, InterruptHandler, Twim};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;

// Bind TWISPI0 interrupt to the TWIM interrupt handler
bind_interrupts!(struct Irqs {
  TWISPI0 => InterruptHandler<TWISPI0>;
});

// LSM303AGR I2C addresses
const ACCEL_ADDR: u8 = 0x19;
const MAGNET_ADDR: u8 = 0x1E;

// LSM303AGR Accelerometer registers
const ACCEL_CTRL_REG1: u8 = 0x20;
const ACCEL_OUT_X_L: u8 = 0x28;

// LSM303AGR Magnetometer registers
const MAGNET_CFG_REG_A: u8 = 0x60;
const MAGNET_OUT_X_L: u8 = 0x68;

/// Accelerometer data event (3-axis, unit: raw i16, scale depends on config)
#[derive(Debug, Clone, Copy)]
pub struct AccelData {
  pub x: i16,
  pub y: i16,
  pub z: i16,
}

/// Magnetometer data event (3-axis, unit: raw i16)
#[derive(Debug, Clone, Copy)]
pub struct MagnetData {
  pub x: i16,
  pub y: i16,
  pub z: i16,
}

/// Accelerometer data channel
pub static ACCEL_EVENTS: Channel<CriticalSectionRawMutex, AccelData, 4> = Channel::new();

/// Magnetometer data channel
pub static MAGNET_EVENTS: Channel<CriticalSectionRawMutex, MagnetData, 4> = Channel::new();

/// Accelerometer subscription state
static ACCEL_SUBSCRIBED: AtomicBool = AtomicBool::new(false);

/// Magnetometer subscription state
static MAGNET_SUBSCRIBED: AtomicBool = AtomicBool::new(false);

pub fn set_accel_subscribed(yes: bool) {
  ACCEL_SUBSCRIBED.store(yes, Ordering::Relaxed);
  info!(
    "Accelerometer subscription {}",
    if yes { "enabled" } else { "disabled" }
  );
}

pub fn is_accel_subscribed() -> bool {
  ACCEL_SUBSCRIBED.load(Ordering::Relaxed)
}

pub fn set_magnet_subscribed(yes: bool) {
  MAGNET_SUBSCRIBED.store(yes, Ordering::Relaxed);
  info!(
    "Magnetometer subscription {}",
    if yes { "enabled" } else { "disabled" }
  );
}

pub fn is_magnet_subscribed() -> bool {
  MAGNET_SUBSCRIBED.load(Ordering::Relaxed)
}

/// Motion sensor pin set
pub struct MotionPins {
  pub twi: Peri<'static, TWISPI0>,
  pub sda: Peri<'static, P0_16>,
  pub scl: Peri<'static, P0_08>,
}

/// Initialize LSM303AGR accelerometer: 100Hz, normal mode, all axes enabled
async fn init_accel(twi: &mut Twim<'static>) -> bool {
  // CTRL_REG1_A: ODR=100Hz(0x5), LPen=0, Zen=Yen=Xen=1 => 0x57
  let data = [ACCEL_CTRL_REG1, 0x57];
  match twi.write(ACCEL_ADDR, &data).await {
    Ok(_) => {
      info!("LSM303AGR accelerometer initialized (100Hz, normal mode)");
      true
    }
    Err(e) => {
      warn!(
        "Failed to init accelerometer: {:?}",
        defmt::Debug2Format(&e)
      );
      false
    }
  }
}

/// Initialize LSM303AGR magnetometer: continuous mode, 100Hz
async fn init_magnet(twi: &mut Twim<'static>) -> bool {
  // CFG_REG_A_M: COMP_TEMP_EN=1, ODR=100Hz(0b11), MD=continuous(0b00) => 0x8C
  let data = [MAGNET_CFG_REG_A, 0x8C];
  match twi.write(MAGNET_ADDR, &data).await {
    Ok(_) => {
      info!("LSM303AGR magnetometer initialized (100Hz, continuous)");
      true
    }
    Err(e) => {
      warn!("Failed to init magnetometer: {:?}", defmt::Debug2Format(&e));
      false
    }
  }
}

/// Read accelerometer data (6 bytes: X_L, X_H, Y_L, Y_H, Z_L, Z_H)
async fn read_accel(twi: &mut Twim<'static>) -> Option<AccelData> {
  // Set MSB of register address for auto-increment (multi-byte read)
  let reg = ACCEL_OUT_X_L | 0x80;
  let mut buf = [0u8; 6];
  match twi.write_read(ACCEL_ADDR, &[reg], &mut buf).await {
    Ok(_) => {
      // LSM303AGR accelerometer in normal mode: 10-bit left-justified in 16-bit
      // Raw values are in units of ~4mg per LSB at ±2g range (normal mode)
      let x = i16::from_le_bytes([buf[0], buf[1]]) >> 6; // 10-bit, shift right 6
      let y = i16::from_le_bytes([buf[2], buf[3]]) >> 6;
      let z = i16::from_le_bytes([buf[4], buf[5]]) >> 6;
      // Scale to 0.01g units: at ±2g, 1 LSB ≈ 3.9mg ≈ 0.39 (in 0.01g)
      // Approximate: raw * 4 gives roughly 0.01g units
      Some(AccelData {
        x: x * 4,
        y: y * 4,
        z: z * 4,
      })
    }
    Err(_) => None,
  }
}

/// Read magnetometer data (6 bytes: X_L, X_H, Y_L, Y_H, Z_L, Z_H)
async fn read_magnet(twi: &mut Twim<'static>) -> Option<MagnetData> {
  // Magnetometer registers auto-increment by default
  let reg = MAGNET_OUT_X_L;
  let mut buf = [0u8; 6];
  match twi.write_read(MAGNET_ADDR, &[reg], &mut buf).await {
    Ok(_) => {
      // LSM303AGR magnetometer: 16-bit, 1.5 mgauss/LSB = 0.15 μT/LSB
      // We report in 0.1μT units, so raw * 1.5 ≈ raw + raw/2
      let x_raw = i16::from_le_bytes([buf[0], buf[1]]);
      let y_raw = i16::from_le_bytes([buf[2], buf[3]]);
      let z_raw = i16::from_le_bytes([buf[4], buf[5]]);
      // Convert to 0.1μT: raw * 1.5 ≈ (raw * 3) / 2
      Some(MagnetData {
        x: ((x_raw as i32 * 3) / 2) as i16,
        y: ((y_raw as i32 * 3) / 2) as i16,
        z: ((z_raw as i32 * 3) / 2) as i16,
      })
    }
    Err(_) => None,
  }
}

/// Motion sensor task: periodically reads accelerometer and magnetometer data
/// when subscribed, and posts events to channels
#[embassy_executor::task]
pub async fn motion_task(pins: MotionPins) {
  // Initialize TWIM (I2C master) with a TX RAM buffer for non-RAM writes
  let mut config = twim::Config::default();
  config.frequency = twim::Frequency::K400;

  // TX RAM buffer for register address writes (small buffer is sufficient)
  static TX_BUF: static_cell::StaticCell<[u8; 16]> = static_cell::StaticCell::new();
  let tx_buf = TX_BUF.init([0u8; 16]);

  let mut twi = Twim::new(pins.twi, Irqs, pins.sda, pins.scl, config, tx_buf);

  // Initialize sensors
  let accel_ok = init_accel(&mut twi).await;
  let magnet_ok = init_magnet(&mut twi).await;

  if !accel_ok && !magnet_ok {
    warn!("Both sensors failed to initialize, motion task exiting");
    return;
  }

  // Polling loop: read sensors at ~20Hz when subscribed
  loop {
    let accel_sub = is_accel_subscribed();
    let magnet_sub = is_magnet_subscribed();

    if accel_sub
      && accel_ok
      && let Some(data) = read_accel(&mut twi).await
    {
      let _ = ACCEL_EVENTS.try_send(data);
    }

    if magnet_sub
      && magnet_ok
      && let Some(data) = read_magnet(&mut twi).await
    {
      let _ = MAGNET_EVENTS.try_send(data);
    }

    // Poll at ~20Hz (50ms interval) when subscribed, slower when idle
    if accel_sub || magnet_sub {
      Timer::after_millis(50).await;
    } else {
      Timer::after_millis(200).await;
    }
  }
}
