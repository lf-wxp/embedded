//! micro:bit V2 buzzer/speaker control
//!
//! The micro:bit V2 has a built-in speaker connected to P0_00.
//! We use PWM to generate square wave tones at specified frequencies.
//!
//! Public API:
//! - [`play_tone`] start playing a tone at given frequency and duration
//! - [`stop_tone`] stop playing immediately
//! - [`sound_task`] background task that manages tone playback with duration

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_nrf::Peri;
use embassy_nrf::peripherals::P0_00;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

/// Signal to notify sound task of new tone request
static TONE_SIGNAL: Signal<CriticalSectionRawMutex, ToneCommand> = Signal::new();

/// Whether sound is currently playing
static PLAYING: AtomicBool = AtomicBool::new(false);

/// Tone command
#[derive(Clone, Copy)]
pub enum ToneCommand {
  /// Play tone at frequency (Hz) for duration (ms)
  Play { freq_hz: u16, duration_ms: u16 },
  /// Stop playing immediately
  Stop,
}

/// Request to play a tone
pub fn play_tone(freq_hz: u16, duration_ms: u16) {
  info!("Play tone: {}Hz for {}ms", freq_hz, duration_ms);
  TONE_SIGNAL.signal(ToneCommand::Play {
    freq_hz,
    duration_ms,
  });
}

/// Request to stop playing
pub fn stop_tone() {
  info!("Stop tone");
  TONE_SIGNAL.signal(ToneCommand::Stop);
}

/// Check if currently playing
pub fn is_playing() -> bool {
  PLAYING.load(Ordering::Relaxed)
}

/// Sound pin set
pub struct SoundPins {
  pub speaker: Peri<'static, P0_00>,
}

/// Sound task: manages PWM-based tone generation with duration control
///
/// micro:bit V2 speaker is on P0_00. We use SimplePwm to generate a square wave.
/// The PWM clock is 16MHz. For a given frequency f:
///   period_ticks = 16_000_000 / f
///   duty = period_ticks / 2 (50% duty cycle for square wave)
#[embassy_executor::task]
pub async fn sound_task(pins: SoundPins) {
  // We'll use PWM0 for sound generation
  // Note: embassy-nrf SimplePwm needs a PWM peripheral instance
  // On nrf52833, PWM0/1/2/3 are available
  // We configure it but keep it disabled until a tone is requested

  // For now, use a simple GPIO toggle approach since PWM peripheral
  // allocation in embassy-nrf with SoftDevice can be complex.
  // We'll use a busy-loop tone generation approach within the task.

  use embassy_nrf::gpio::{Level, Output, OutputDrive};

  let mut speaker = Output::new(pins.speaker, Level::Low, OutputDrive::Standard);

  loop {
    // Wait for a tone command
    let cmd = TONE_SIGNAL.wait().await;

    match cmd {
      ToneCommand::Play {
        freq_hz,
        duration_ms,
      } => {
        if freq_hz == 0 || duration_ms == 0 {
          continue;
        }
        PLAYING.store(true, Ordering::Relaxed);

        // Calculate half-period in microseconds
        // half_period_us = 1_000_000 / (2 * freq_hz)
        let half_period_us: u64 = 500_000 / (freq_hz as u64);
        let total_cycles = (duration_ms as u64 * 1000) / (half_period_us * 2);

        for _ in 0..total_cycles {
          // Check if stop was requested
          if TONE_SIGNAL.signaled() {
            break;
          }
          speaker.set_high();
          Timer::after_micros(half_period_us).await;
          speaker.set_low();
          Timer::after_micros(half_period_us).await;
        }

        speaker.set_low();
        PLAYING.store(false, Ordering::Relaxed);
      }
      ToneCommand::Stop => {
        speaker.set_low();
        PLAYING.store(false, Ordering::Relaxed);
      }
    }
  }
}
