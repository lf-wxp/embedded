//! micro:bit V2 onboard buttons A/B
//!
//! Pin mapping (micro:bit V2):
//! - Button A: P0_14 (pressed = low level)
//! - Button B: P0_23 (pressed = low level)
//!
//! Public API:
//! - [`button_task`] background task, monitors dual-button edge events and writes to signal
//! - [`BUTTON_EVENTS`] global event signal, read by BLE task and pushed to browser via NUS
//! - [`set_subscribed`] set subscription status (events are still recorded but not dispatched when unsubscribed)

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_nrf::Peri;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::peripherals::{P0_14, P0_23};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// Button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
  A = 1,
  B = 2,
}

/// Button event
#[derive(Debug, Clone, Copy)]
pub struct ButtonEvent {
  pub id: ButtonId,
  /// true = pressed (low level), false = released
  pub pressed: bool,
}

/// Button event channel (capacity 4, sufficient for buffering consecutive button presses)
pub static BUTTON_EVENTS: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/// Whether button events are subscribed (controlled by browser via [`crate::ble::protocol::CMD_BTN_SUBSCRIBE`])
static SUBSCRIBED: AtomicBool = AtomicBool::new(false);

pub fn set_subscribed(yes: bool) {
  SUBSCRIBED.store(yes, Ordering::Relaxed);
  info!(
    "Button event subscription {}",
    if yes { "enabled" } else { "disabled" }
  );
}

pub fn is_subscribed() -> bool {
  SUBSCRIBED.load(Ordering::Relaxed)
}

/// Button pin set
pub struct ButtonPins {
  pub btn_a: Peri<'static, P0_14>,
  pub btn_b: Peri<'static, P0_23>,
}

/// Button monitoring task: detects level changes on both buttons and posts events to [`BUTTON_EVENTS`]
#[embassy_executor::task]
pub async fn button_task(pins: ButtonPins) {
  let mut a = Input::new(pins.btn_a, Pull::Up);
  let mut b = Input::new(pins.btn_b, Pull::Up);

  // Publish initial state on power-up (released)
  let mut last_a = a.is_high();
  let mut last_b = b.is_high();

  loop {
    // Wait for any button level transition
    match select(a.wait_for_any_edge(), b.wait_for_any_edge()).await {
      Either::First(_) => {
        // Simple debounce
        embassy_time::Timer::after_millis(15).await;
        let now = a.is_high();
        if now != last_a {
          last_a = now;
          let evt = ButtonEvent {
            id: ButtonId::A,
            pressed: !now,
          };
          let _ = BUTTON_EVENTS.try_send(evt);
        }
      }
      Either::Second(_) => {
        embassy_time::Timer::after_millis(15).await;
        let now = b.is_high();
        if now != last_b {
          last_b = now;
          let evt = ButtonEvent {
            id: ButtonId::B,
            pressed: !now,
          };
          let _ = BUTTON_EVENTS.try_send(evt);
        }
      }
    }
  }
}
