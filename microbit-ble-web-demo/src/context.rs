//! 全局上下文模块
//! 提供应用级别的共享状态，包括 BLE 服务和响应式信号

use crate::services::ble::BleService;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// 接收到的帧数据（已解析）
#[derive(Clone, Debug)]
pub struct ReceivedFrame {
  pub cmd: u8,
  pub payload: Vec<u8>,
}

/// 全局应用状态
#[derive(Clone, Copy)]
pub struct AppState {
  /// 连接状态
  pub connected: RwSignal<bool>,
  /// 正在连接中
  pub connecting: RwSignal<bool>,
  /// 设备名称
  pub device_name: RwSignal<Option<String>>,
  /// 最新接收到的帧（其他组件通过监听此信号获取数据）
  pub last_frame: RwSignal<Option<ReceivedFrame>>,
}

impl Default for AppState {
  fn default() -> Self {
    Self::new()
  }
}

impl AppState {
  pub fn new() -> Self {
    Self {
      connected: RwSignal::new(false),
      connecting: RwSignal::new(false),
      device_name: RwSignal::new(None),
      last_frame: RwSignal::new(None),
    }
  }
}

/// 全局共享的 BLE 服务句柄
/// WASM 是单线程的，使用 Rc<RefCell<>> 即可
#[derive(Clone)]
pub struct SharedBleService(pub Rc<RefCell<BleService>>);

impl SharedBleService {
  pub fn new() -> Self {
    Self(Rc::new(RefCell::new(BleService::new())))
  }
}

impl Default for SharedBleService {
  fn default() -> Self {
    Self::new()
  }
}

// 全局 BLE 服务实例（WASM 单线程，使用 thread_local 安全）
thread_local! {
  static GLOBAL_BLE: RefCell<Option<SharedBleService>> = const { RefCell::new(None) };
}

/// 初始化全局 BLE 服务
pub fn init_global_ble(ble: SharedBleService) {
  GLOBAL_BLE.with(|g| {
    *g.borrow_mut() = Some(ble);
  });
}

/// 获取全局 BLE 服务的克隆
pub fn get_global_ble() -> Option<SharedBleService> {
  GLOBAL_BLE.with(|g| g.borrow().clone())
}
