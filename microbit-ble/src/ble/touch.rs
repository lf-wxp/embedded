//! micro:bit V2 capacitive touch sensors
//!
//! Pin mapping (micro:bit V2):
//! - Logo touch: P0_10 (golden logo area)
//! - Pin 0 touch: P0_02
//! - Pin 1 touch: P0_03
//! - Pin 2 touch: P0_04
//!
//! Detection method:
//! The micro:bit V2 touch sensors work via capacitive sensing. The Logo pad and
//! edge connector pins act as one plate of a capacitor; when a finger touches them,
//! body capacitance increases the total capacitance on the pin.
//!
//! We use a polling approach with Pull::None (high-impedance input) to detect
//! the touch state. The nRF52833 GPIO input buffer reads the voltage level on the pin.
//! When touched, the capacitive coupling from the finger causes the pin to read HIGH.
//!
//! Polling at ~50Hz with debounce provides reliable touch detection.
//!
//! Public API:
//! - [`touch_task`] background task, monitors four touch sensors and posts events to channel
//! - [`TOUCH_EVENTS`] global event channel, read by BLE task and pushed to browser via NUS
//! - [`set_subscribed`] set subscription status

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_nrf::Peri;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::peripherals::{P0_02, P0_03, P0_04, P0_10};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;

/// Touch sensor identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchId {
  Logo = 0,
  Pin0 = 1,
  Pin1 = 2,
  Pin2 = 3,
}

/// Touch event
#[derive(Debug, Clone, Copy)]
pub struct TouchEvent {
  pub id: TouchId,
  /// true = touched, false = released
  pub pressed: bool,
}

/// Touch event channel (capacity 8, sufficient for buffering consecutive touch events from 4 sensors)
pub static TOUCH_EVENTS: Channel<CriticalSectionRawMutex, TouchEvent, 8> = Channel::new();

/// Whether touch events are subscribed (controlled by browser via CMD_TOUCH_SUBSCRIBE)
static SUBSCRIBED: AtomicBool = AtomicBool::new(false);

pub fn set_subscribed(yes: bool) {
  SUBSCRIBED.store(yes, Ordering::Relaxed);
  info!(
    "Touch event subscription {}",
    if yes { "enabled" } else { "disabled" }
  );
}

pub fn is_subscribed() -> bool {
  SUBSCRIBED.load(Ordering::Relaxed)
}

/// Touch pin set
pub struct TouchPins {
  pub logo: Peri<'static, P0_10>,
  pub pin0: Peri<'static, P0_02>,
  pub pin1: Peri<'static, P0_03>,
  pub pin2: Peri<'static, P0_04>,
}

/// Debounce state for a single touch sensor
struct TouchState {
  id: TouchId,
  pressed: bool,
  /// Number of consecutive samples in the new state (for debounce)
  count: u8,
}

impl TouchState {
  fn new(id: TouchId) -> Self {
    Self {
      id,
      pressed: false,
      count: 0,
    }
  }

  /// Update with a new sample. Returns Some(event) if state changed after debounce.
  /// Debounce threshold: 3 consecutive samples (~60ms at 50Hz polling)
  fn update(&mut self, is_high: bool) -> Option<TouchEvent> {
    const DEBOUNCE_THRESHOLD: u8 = 3;

    if is_high != self.pressed {
      self.count += 1;
      if self.count >= DEBOUNCE_THRESHOLD {
        self.pressed = is_high;
        self.count = 0;
        return Some(TouchEvent {
          id: self.id,
          pressed: is_high,
        });
      }
    } else {
      self.count = 0;
    }
    None
  }
}

/// Touch monitoring task: polls all four touch sensors and posts events to [`TOUCH_EVENTS`]
///
/// Uses Pull::None (high-impedance) input mode. The micro:bit V2 touch pads
/// have external circuitry that produces a readable digital level when touched.
/// Polling at ~50Hz with software debounce (3 consecutive samples = ~60ms).
#[embassy_executor::task]
pub async fn touch_task(pins: TouchPins) {
  // Configure pins as inputs with no pull resistor (high-impedance)
  // This allows the capacitive touch signal to be read without interference
  let logo = Input::new(pins.logo, Pull::None);
  let pin0 = Input::new(pins.pin0, Pull::None);
  let pin1 = Input::new(pins.pin1, Pull::None);
  let pin2 = Input::new(pins.pin2, Pull::None);

  // Initialize debounce state for each sensor
  let mut state_logo = TouchState::new(TouchId::Logo);
  let mut state_pin0 = TouchState::new(TouchId::Pin0);
  let mut state_pin1 = TouchState::new(TouchId::Pin1);
  let mut state_pin2 = TouchState::new(TouchId::Pin2);

  info!("Touch sensor task started (polling mode, Pull::None)");

  loop {
    // Read current state of all touch pins
    // Check Logo
    if let Some(evt) = state_logo.update(logo.is_high()) {
      let _ = TOUCH_EVENTS.try_send(evt);
    }

    // Check Pin 0
    if let Some(evt) = state_pin0.update(pin0.is_high()) {
      let _ = TOUCH_EVENTS.try_send(evt);
    }

    // Check Pin 1
    if let Some(evt) = state_pin1.update(pin1.is_high()) {
      let _ = TOUCH_EVENTS.try_send(evt);
    }

    // Check Pin 2
    if let Some(evt) = state_pin2.update(pin2.is_high()) {
      let _ = TOUCH_EVENTS.try_send(evt);
    }

    // Poll at ~50Hz (20ms interval)
    Timer::after_millis(20).await;
  }
}
