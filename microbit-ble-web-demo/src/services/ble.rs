//! Web Bluetooth 服务模块
//! 封装 Web Bluetooth API 调用，提供 Rust 风格的异步接口
//!
//! 参考：https://developer.mozilla.org/en-US/docs/Web/API/Web_Bluetooth_API

use crate::utils::{NUS_RX_CHAR, NUS_SERVICE, NUS_TX_CHAR};
use js_sys::{JsString, Uint8Array};

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
  Bluetooth, BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
  BluetoothRemoteGattServer, BluetoothRemoteGattService, EventTarget, RequestDeviceOptions,
};

/// BLE 连接状态
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleConnectionState {
  Disconnected,
  Connecting,
  Connected,
}

/// 数据接收回调类型（WASM 单线程，使用 Rc<RefCell<>>）
type OnDataCallback = Rc<RefCell<Option<Box<dyn Fn(Vec<u8>)>>>>;
/// 状态变化回调类型
type OnStateChangeCallback = Rc<RefCell<Option<Box<dyn Fn(BleConnectionState)>>>>;

/// BLE 服务句柄（内部可变状态）
/// WASM 单线程，使用 Rc<RefCell<>> 安全共享
#[derive(Clone)]
pub struct BleService {
  pub device: Option<BluetoothDevice>,
  pub server: Option<BluetoothRemoteGattServer>,
  pub rx_char: Option<BluetoothRemoteGattCharacteristic>,
  pub tx_char: Option<BluetoothRemoteGattCharacteristic>,
  on_data: OnDataCallback,
  on_state_change: OnStateChangeCallback,
}

impl Default for BleService {
  fn default() -> Self {
    Self::new()
  }
}

impl BleService {
  pub fn new() -> Self {
    Self {
      device: None,
      server: None,
      rx_char: None,
      tx_char: None,
      on_data: Rc::new(RefCell::new(None)),
      on_state_change: Rc::new(RefCell::new(None)),
    }
  }

  /// 请求用户选择设备并连接
  pub async fn connect(&mut self) -> Result<(), String> {
    let bluetooth = get_bluetooth()?;

    // 构建 requestDevice 选项
    // https://developer.mozilla.org/en-US/docs/Web/API/Bluetooth/requestDevice
    let opts = RequestDeviceOptions::new();

    // filters: 按名称前缀过滤
    let filter = BluetoothLeScanFilterInit::new();
    filter.set_name_prefix("MicroBit");
    opts.set_filters(&[filter]);

    // optionalServices: 需要访问的服务 UUID
    opts.set_optional_services(&[JsString::from(NUS_SERVICE)]);

    // 请求设备 - request_device 返回 Promise<BluetoothDevice>
    // JsFuture<BluetoothDevice>.await 直接得到 BluetoothDevice
    let device: BluetoothDevice = JsFuture::from(bluetooth.request_device(&opts))
      .await
      .map_err(|e| format!("设备选择失败或用户取消: {e:?}"))?;

    // 设置断开连接监听器
    self.setup_disconnect_listener(&device)?;

    // 连接 GATT 服务器
    self.set_state(BleConnectionState::Connecting);
    let gatt_server = device.gatt().ok_or("设备不支持 GATT")?;
    // connect() 返回 Promise<BluetoothRemoteGattServer>
    let server: BluetoothRemoteGattServer = JsFuture::from(gatt_server.connect())
      .await
      .map_err(|e| format!("GATT 连接失败: {e:?}"))?;

    // 获取 Nordic UART Service
    // get_primary_services_with_str 返回 Promise<Array<BluetoothRemoteGattService>>
    let services = JsFuture::from(server.get_primary_services_with_str(NUS_SERVICE))
      .await
      .map_err(|e| format!("获取 NUS 服务失败: {e:?}"))?;
    // Array<T>.get(0) 直接返回 T 类型
    let service: BluetoothRemoteGattService = services.get(0);

    // 获取 RX 特征（写入）
    // get_characteristic_with_str 返回 Promise<BluetoothRemoteGattCharacteristic>
    let rx_char: BluetoothRemoteGattCharacteristic =
      JsFuture::from(service.get_characteristic_with_str(NUS_RX_CHAR))
        .await
        .map_err(|e| format!("获取 RX 特征失败: {e:?}"))?;

    // 获取 TX 特征（通知）
    let tx_char: BluetoothRemoteGattCharacteristic =
      JsFuture::from(service.get_characteristic_with_str(NUS_TX_CHAR))
        .await
        .map_err(|e| format!("获取 TX 特征失败: {e:?}"))?;

    // 设置通知回调（在启动通知之前设置，避免丢失数据）
    self.setup_notification_handler(&tx_char)?;

    // 启动通知 - start_notifications 返回 Promise<BluetoothRemoteGattCharacteristic>
    JsFuture::from(tx_char.start_notifications())
      .await
      .map_err(|e| format!("启动通知失败: {e:?}"))?;

    self.device = Some(device);
    self.server = Some(server);
    self.rx_char = Some(rx_char);
    self.tx_char = Some(tx_char);

    self.set_state(BleConnectionState::Connected);

    Ok(())
  }

  /// 设置断开连接监听器
  fn setup_disconnect_listener(&self, device: &BluetoothDevice) -> Result<(), String> {
    let on_state_change = self.on_state_change.clone();

    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      log::info!("设备已断开连接");
      let callback = on_state_change.borrow();
      if let Some(ref cb) = *callback {
        cb(BleConnectionState::Disconnected);
      }
    }) as Box<dyn FnMut(_)>);

    let callback: &js_sys::Function = closure.as_ref().unchecked_ref();
    device.set_ongattserverdisconnected(Some(callback));
    closure.forget();

    Ok(())
  }

  /// 设置 TX 特征的通知处理程序
  fn setup_notification_handler(
    &self,
    tx_char: &BluetoothRemoteGattCharacteristic,
  ) -> Result<(), String> {
    let on_data = self.on_data.clone();

    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
      let target = event.target().unwrap();
      let char: &BluetoothRemoteGattCharacteristic = target.unchecked_ref();

      // 获取特征值（DataView）
      let data_view = match char.value() {
        Some(v) => v,
        None => {
          log::warn!("通知事件中没有值");
          return;
        }
      };

      // 从 DataView 正确转换为 Vec<u8>
      // DataView 不能直接传给 Uint8Array::new()，需要使用 buffer + offset + length
      let buffer = data_view.buffer();
      let byte_offset = data_view.byte_offset() as u32;
      let byte_length = data_view.byte_length() as u32;
      let bytes =
        js_sys::Uint8Array::new_with_byte_offset_and_length(&buffer, byte_offset, byte_length)
          .to_vec();

      // 调用回调函数
      let callback = on_data.borrow();
      if let Some(ref cb) = *callback {
        cb(bytes);
      }
    }) as Box<dyn FnMut(_)>);

    // 设置通知处理程序
    // add_event_listener_with_callback 定义在 EventTarget 上
    // BluetoothRemoteGattCharacteristic 实现了 AsRef<EventTarget>
    let callback: &js_sys::Function = closure.as_ref().unchecked_ref();
    <BluetoothRemoteGattCharacteristic as AsRef<EventTarget>>::as_ref(tx_char)
      .add_event_listener_with_callback("characteristicvaluechanged", callback)
      .map_err(|e| format!("添加通知处理程序失败: {e:?}"))?;

    // 忘记闭包（避免内存泄漏）
    closure.forget();

    Ok(())
  }

  /// 发送数据帧
  pub async fn send(&self, data: &[u8]) -> Result<(), String> {
    let rx_char = self.rx_char.as_ref().ok_or("未连接")?;

    // 将 &[u8] 转换为 Uint8Array（write_value_with_u8_array 需要 &Uint8Array 参数）
    // write_value_with_u8_array 返回 Result<Promise<Undefined>, JsValue>
    // SAFETY: Uint8Array::view 要求 data 在调用期间不被修改或释放，
    // 由于我们在 await 之前创建 view 且 data 是传入的引用，
    // 在 JsFuture::from(promise).await 完成前 data 必须保持有效
    let js_data = unsafe { Uint8Array::view(data) };
    let promise = rx_char
      .write_value_with_u8_array(&js_data)
      .map_err(|e| format!("写入请求失败: {e:?}"))?;
    JsFuture::from(promise)
      .await
      .map_err(|e| format!("写入失败: {e:?}"))?;

    log::debug!("TX: {}", crate::utils::to_hex(data));
    Ok(())
  }

  /// 断开连接
  pub fn disconnect(&mut self) {
    if let Some(device) = &self.device {
      if let Some(gatt) = device.gatt() {
        gatt.disconnect();
      }
    }
    self.device = None;
    self.server = None;
    self.rx_char = None;
    self.tx_char = None;
    self.set_state(BleConnectionState::Disconnected);
  }

  /// 检查是否已连接
  pub fn is_connected(&self) -> bool {
    self.server.as_ref().is_some_and(|s| s.connected())
  }

  /// 获取设备名称
  pub fn device_name(&self) -> Option<String> {
    self.device.as_ref().and_then(|d| d.name())
  }

  /// 设置数据接收回调
  pub fn set_on_data<F>(&mut self, f: F)
  where
    F: Fn(Vec<u8>) + 'static,
  {
    *self.on_data.borrow_mut() = Some(Box::new(f));
  }

  /// 设置状态变化回调
  pub fn set_on_state_change<F>(&mut self, f: F)
  where
    F: Fn(BleConnectionState) + 'static,
  {
    *self.on_state_change.borrow_mut() = Some(Box::new(f));
  }

  /// 内部：设置状态并触发回调
  fn set_state(&self, state: BleConnectionState) {
    log::info!("BLE 状态变化: {state:?}");
    let callback = self.on_state_change.borrow();
    if let Some(ref cb) = *callback {
      cb(state);
    }
  }
}
/// 获取 Window 对象的 Bluetooth API
fn get_bluetooth() -> Result<Bluetooth, String> {
  let window = web_sys::window().ok_or("无法获取 window 对象")?;
  window
    .navigator()
    .bluetooth()
    .ok_or_else(|| "浏览器不支持 Web Bluetooth API".to_string())
}

/// 检查浏览器是否支持 Web Bluetooth
pub fn is_bluetooth_supported() -> bool {
  web_sys::window()
    .and_then(|w| w.navigator().bluetooth())
    .is_some()
}
