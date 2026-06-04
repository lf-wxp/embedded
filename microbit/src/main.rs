#![no_main]
#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};

use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use microbit::{
  board::Board,
  hal::{
    Timer,
    gpiote,
    pac::{self, interrupt},
  },
};
use nrf52833_pac::PWM0;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use mb2_wukong_expansion::{RGB8, WuKongAmbient};

// 全局运行标志，由按钮中断修改，定时器中断读取
static RUNNING: AtomicBool = AtomicBool::new(false);

// 全局 LED 控制器
struct Ambient {
  wka: WuKongAmbient<PWM0>,
  red: u8,
}
static AMBIENT: LockMut<Ambient> = LockMut::new();

// 全局 GPIOTE 外设，用于清除事件
static GPIOTE_PERIPHERAL: LockMut<gpiote::Gpiote> = LockMut::new();

// 全局定时器实例，供中断使用
static TIMER1: LockMut<Option<Timer<pac::TIMER1>>> = LockMut::new();

// 按钮 A 的中断处理
#[interrupt]
fn GPIOTE() {
  RUNNING.fetch_xor(true, Ordering::Relaxed); // 切换标志
  rprintln!(
    "button pressed, running = {}",
    RUNNING.load(Ordering::Relaxed)
  );

  GPIOTE_PERIPHERAL.with_lock(|gpiote| {
    gpiote.channel0().reset_events();
  });
}

// 定时器 TIMER1 的中断处理
#[interrupt]
fn TIMER1() {
  if RUNNING.load(Ordering::Relaxed) {
    AMBIENT.with_lock(|ambient| {
      let rgb = RGB8::new(ambient.red, 64, 64);
      ambient.wka.set_color(3, rgb).unwrap();
      ambient.red = ambient.red.wrapping_add(1);
    });
  }

  // 重新启动定时器，实现周期性（20ms）
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

  // 1. 初始化按钮 A 和 GPIOTE 中断
  let button_a = board.buttons.button_a.into_pullup_input();
  let gpiote = gpiote::Gpiote::new(board.GPIOTE);
  let channel = gpiote.channel0();
  channel
    .input_pin(&button_a.degrade())
    .hi_to_lo()
    .enable_interrupt();
  channel.reset_events();
  GPIOTE_PERIPHERAL.init(gpiote);

  // 2. 初始化 Ambient LED
  let ambient = Ambient {
    wka: WuKongAmbient::new(board.PWM0, board.edge.e16).unwrap(),
    red: 0,
  };
  AMBIENT.init(ambient);

  // 3. 初始化定时器 TIMER1
  let mut timer1 = Timer::new(board.TIMER1);
  timer1.enable_interrupt();
  timer1.start(20_000);
  TIMER1.init(Some(timer1));

  // 4. 使能 NVIC 中断
  unsafe { pac::NVIC::unmask(pac::Interrupt::GPIOTE) };
  pac::NVIC::unpend(pac::Interrupt::GPIOTE);
  unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER1) };
  pac::NVIC::unpend(pac::Interrupt::TIMER1);

  rprintln!("Ready: press button A to start/stop LED breathing");

  loop {
    wfi();
  }
}
