//! micro:bit V2 板载按键 A/B
//!
//! 引脚映射（micro:bit V2）：
//! - 按键 A: P0_14 （按下=低电平）
//! - 按键 B: P0_23 （按下=低电平）
//!
//! 对外提供：
//! - [`button_task`] 后台任务，监听双键边沿事件并写入信号
//! - [`BUTTON_EVENTS`] 全局事件信号，由 BLE 任务读取并通过 NUS 推送给浏览器
//! - [`set_subscribed`] 设置订阅状态（取消订阅时事件仍记录但不上抛）

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::info;
use embassy_futures::select::{Either, select};
use embassy_nrf::Peri;
use embassy_nrf::gpio::{Input, Pull};
use embassy_nrf::peripherals::{P0_14, P0_23};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// 按键标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonId {
  A = 1,
  B = 2,
}

/// 按键事件
#[derive(Debug, Clone, Copy)]
pub struct ButtonEvent {
  pub id: ButtonId,
  /// true = 按下（低电平），false = 释放
  pub pressed: bool,
}

/// 按键事件通道（容量 4，足够缓冲连续按键）
pub static BUTTON_EVENTS: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/// 是否已订阅按键事件（由浏览器通过 [`crate::ble::protocol::CMD_BTN_SUBSCRIBE`] 控制）
static SUBSCRIBED: AtomicBool = AtomicBool::new(false);

pub fn set_subscribed(yes: bool) {
  SUBSCRIBED.store(yes, Ordering::Relaxed);
  info!(
    "按键事件订阅 {}",
    if yes {
      "已启用"
    } else {
      "已禁用"
    }
  );
}

pub fn is_subscribed() -> bool {
  SUBSCRIBED.load(Ordering::Relaxed)
}

/// 按键引脚集合
pub struct ButtonPins {
  pub btn_a: Peri<'static, P0_14>,
  pub btn_b: Peri<'static, P0_23>,
}

/// 按键监听任务：检测两个按键的电平变化，并把事件投递到 [`BUTTON_EVENTS`]
#[embassy_executor::task]
pub async fn button_task(pins: ButtonPins) {
  let mut a = Input::new(pins.btn_a, Pull::Up);
  let mut b = Input::new(pins.btn_b, Pull::Up);

  // 上电后先发布一次初始状态（释放）
  let mut last_a = a.is_high();
  let mut last_b = b.is_high();

  loop {
    // 等待任一按键电平翻转
    match select(a.wait_for_any_edge(), b.wait_for_any_edge()).await {
      Either::First(_) => {
        // 简单防抖
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
