#![no_main]
#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};

use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use microbit::{
  board::Board,
  hal::{
    Timer, gpiote,
    pac::{self, interrupt},
  },
};
use nrf52833_pac::PWM0;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use mb2_wukong_expansion::{RGB8, WuKongAmbient};

// Global running flag, modified by button interrupt, read by timer interrupt
static RUNNING: AtomicBool = AtomicBool::new(false);

// Global LED controller
struct Ambient {
  wka: WuKongAmbient<PWM0>,
  red: u8,
}
static AMBIENT: LockMut<Ambient> = LockMut::new();

// Global GPIOTE peripheral for clearing events
static GPIOTE_PERIPHERAL: LockMut<gpiote::Gpiote> = LockMut::new();

// Global timer instance for use in interrupts
static TIMER1: LockMut<Option<Timer<pac::TIMER1>>> = LockMut::new();

static DEBOUNCE_TIMER: LockMut<Option<Timer<pac::TIMER0>>> = LockMut::new();

// 100ms debounce time (Timer default 1MHz, 1 cycle = 1μs)
const DEBOUNCE_TIME: u32 = 100_000;

// Button A interrupt handler
#[interrupt]
fn GPIOTE() {
  // debounce: only accept button press when timer is not running
  let mut accepted = false;
  DEBOUNCE_TIMER.with_lock(|timer_opt| {
    if let Some(timer) = timer_opt {
      if timer.reset_if_finished() {
        // debounce finished, restart timer
        timer.start(DEBOUNCE_TIME);
        accepted = true;
      }
    }
  });

  if accepted {
    RUNNING.fetch_xor(true, Ordering::Relaxed);
    rprintln!(
      "button pressed, running = {}",
      RUNNING.load(Ordering::Relaxed)
    );
  }

  GPIOTE_PERIPHERAL.with_lock(|gpiote| {
    gpiote.channel0().reset_events();
  });
}

// TIMER1 interrupt handler
#[interrupt]
fn TIMER1() {
  if RUNNING.load(Ordering::Relaxed) {
    AMBIENT.with_lock(|ambient| {
      let rgb = RGB8::new(ambient.red, 64, 64);
      ambient.wka.set_color(3, rgb).unwrap();
      ambient.red = ambient.red.wrapping_add(1);
    });
  } else {
    // Turn off LED and reset color counter when stopped
    AMBIENT.with_lock(|ambient| {
      ambient.wka.set_color(3, RGB8::new(0, 0, 0)).unwrap();
      ambient.red = 0;
    });
  }

  // Restart timer for periodic triggering (20ms)
  TIMER1.with_lock(|timer_opt| {
    if let Some(timer) = timer_opt {
      timer.reset_event();
      timer.start(20_000); // 20ms
    }
  });
}

#[entry]
fn main() -> ! {
  rtt_init_print!();
  let board = Board::take().unwrap();

  // 1. Initialize button A and GPIOTE interrupt
  let button_a = board.buttons.button_a.into_pullup_input();
  let gpiote = gpiote::Gpiote::new(board.GPIOTE);
  let channel = gpiote.channel0();
  channel
    .input_pin(&button_a.degrade())
    .hi_to_lo()
    .enable_interrupt();
  channel.reset_events();
  GPIOTE_PERIPHERAL.init(gpiote);

  // 2. Initialize Ambient LED
  let ambient = Ambient {
    wka: WuKongAmbient::new(board.PWM0, board.edge.e16).unwrap(),
    red: 0,
  };
  AMBIENT.init(ambient);

  // 3. Initialize TIMER1
  let mut timer1 = Timer::new(board.TIMER1);
  timer1.enable_interrupt();
  timer1.start(20_000);
  TIMER1.init(Some(timer1));

  // 4. Initialize debounce timer (polling only, no interrupt)
  let mut debounce_timer = Timer::new(board.TIMER0);
  debounce_timer.disable_interrupt();
  // Start a 1-cycle timer so timer completes immediately,
  // making events_compare[0] "triggered", so reset_if_finished() returns true on first button press
  debounce_timer.start(1);
  cortex_m::asm::delay(100); // Busy-wait far beyond 1μs to ensure timer completes
  DEBOUNCE_TIMER.init(Some(debounce_timer));

  // 5. Enable NVIC interrupts
  unsafe { pac::NVIC::unmask(pac::Interrupt::GPIOTE) };
  pac::NVIC::unpend(pac::Interrupt::GPIOTE);
  unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER1) };
  pac::NVIC::unpend(pac::Interrupt::TIMER1);

  rprintln!("Ready: press button A to start/stop LED breathing");

  loop {
    wfi();
  }
}
