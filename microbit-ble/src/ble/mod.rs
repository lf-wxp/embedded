//! BLE 蓝牙模块
//!
//! 封装 nrf-softdevice 的蓝牙操作，提供简洁的 API 用于：
//! - 启用/禁用 SoftDevice
//! - 配置和注册 GATT Server
//! - 控制 BLE 广播（开启/停止）
//! - 管理 BLE 连接

use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use nrf_softdevice::Softdevice;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::raw;

/// BLE 广播控制信号
static ADVERTISING_ENABLED: AtomicBool = AtomicBool::new(true);

/// BLE 断开连接信号（用于主动断开当前连接）
static DISCONNECT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// 电池服务 (Battery Service UUID: 0x180F)
#[nrf_softdevice::gatt_server]
pub struct BatteryServer {
  /// 电池服务
  pub battery: BatteryService,
}

/// 电池服务定义
#[nrf_softdevice::gatt_service(uuid = "180f")]
pub struct BatteryService {
  /// 电池电量特征值 (Battery Level UUID: 0x2A19)
  /// 值范围: 0-100，表示电池百分比
  #[characteristic(uuid = "2a19", read, notify)]
  pub battery_level: u8,
}

/// BLE 配置参数
pub struct BleConfig {
  /// 设备名称（显示在蓝牙扫描列表中）
  pub device_name: &'static [u8],
  /// 最大同时连接数
  pub max_connections: u8,
  /// ATT MTU 大小
  pub att_mtu: u16,
}

impl Default for BleConfig {
  fn default() -> Self {
    Self {
      device_name: b"MicroBit-BLE",
      max_connections: 1,
      att_mtu: 256,
    }
  }
}

/// BLE 控制器
///
/// 管理蓝牙协议栈的生命周期和操作
pub struct BleController {
  sd: &'static Softdevice,
  server: BatteryServer,
}

impl BleController {
  /// 启用 SoftDevice 并初始化 BLE 控制器
  ///
  /// 此函数完成以下操作：
  /// 1. 配置 SoftDevice（时钟、GAP、GATT 参数）
  /// 2. 启用 SoftDevice
  /// 3. 注册 GATT Server
  /// 4. 启动 SoftDevice 后台任务
  pub fn enable(spawner: &Spawner, config: &BleConfig) -> Self {
    let sd_config = nrf_softdevice::Config {
      clock: Some(raw::nrf_clock_lf_cfg_t {
        // micro:bit V2 使用外部 32.768kHz 晶振
        source: raw::NRF_CLOCK_LF_SRC_XTAL as u8,
        rc_ctiv: 0,
        rc_temp_ctiv: 0,
        accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
      }),
      conn_gap: Some(raw::ble_gap_conn_cfg_t {
        conn_count: config.max_connections,
        event_length: 24,
      }),
      conn_gatt: Some(raw::ble_gatt_conn_cfg_t {
        att_mtu: config.att_mtu,
      }),
      gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
        attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
      }),
      gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
        adv_set_count: 1,
        periph_role_count: 1,
      }),
      gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
        p_value: config.device_name.as_ptr() as *mut u8,
        current_len: config.device_name.len() as u16,
        max_len: config.device_name.len() as u16,
        write_perm: unsafe { mem::zeroed() },
        _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
          raw::BLE_GATTS_VLOC_STACK as u8,
        ),
      }),
      ..Default::default()
    };

    let sd = Softdevice::enable(&sd_config);
    info!("SoftDevice 已启用");

    let server = BatteryServer::new(sd).expect("GATT Server 注册失败");
    info!("GATT Server 已注册");

    // 启动 SoftDevice 后台任务
    spawner.spawn(softdevice_task(sd).expect("softdevice_task spawn token 创建失败"));

    Self { sd, server }
  }

  /// 获取 SoftDevice 引用
  pub fn softdevice(&self) -> &'static Softdevice {
    self.sd
  }

  /// 获取 GATT Server 引用
  pub fn server(&self) -> &BatteryServer {
    &self.server
  }

  /// 开启 BLE 广播
  ///
  /// 设置广播使能标志，下一次广播循环将开始广播
  pub fn start_advertising(&self) {
    ADVERTISING_ENABLED.store(true, Ordering::SeqCst);
    info!("BLE 广播已开启");
  }

  /// 停止 BLE 广播
  ///
  /// 清除广播使能标志，当前广播周期结束后将停止广播
  pub fn stop_advertising(&self) {
    ADVERTISING_ENABLED.store(false, Ordering::SeqCst);
    info!("BLE 广播已停止");
  }

  /// 查询广播是否开启
  pub fn is_advertising(&self) -> bool {
    ADVERTISING_ENABLED.load(Ordering::SeqCst)
  }

  /// 主动断开当前 BLE 连接
  ///
  /// 如果当前有活跃连接，发送断开信号
  pub fn disconnect(&self) {
    DISCONNECT_SIGNAL.signal(());
    info!("已发送断开连接信号");
  }

  /// 设置电池电量并通知已连接的设备
  ///
  /// # 参数
  /// - `level`: 电池电量百分比 (0-100)
  /// - `conn`: 当前活跃的 BLE 连接（如果有）
  pub fn set_battery_level(&self, level: u8, conn: Option<&nrf_softdevice::ble::Connection>) {
    let level = level.min(100);
    if let Err(_e) = self.server.battery.battery_level_set(&level) {
      warn!("设置电池电量失败");
    }
    if let Some(conn) = conn
      && let Err(_e) = self.server.battery.battery_level_notify(conn, &level)
    {
      // 通知失败可能是因为客户端未订阅，不需要报错
    }
  }

  /// 运行 BLE 广播和连接处理循环
  ///
  /// 此函数会无限循环执行以下操作：
  /// 1. 检查广播使能标志
  /// 2. 开始 BLE 广播，等待中心设备连接
  /// 3. 连接建立后处理 GATT 事件
  /// 4. 连接断开后重新开始广播
  ///
  /// # 参数
  /// - `adv_data`: 广播数据包
  /// - `scan_data`: 扫描响应数据包
  pub async fn run(&self, adv_data: &[u8], scan_data: &[u8]) -> ! {
    loop {
      // 检查广播是否启用
      if !ADVERTISING_ENABLED.load(Ordering::SeqCst) {
        // 广播未启用，等待一段时间后重新检查
        embassy_time::Timer::after_millis(100).await;
        continue;
      }

      let config = peripheral::Config::default();
      let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
        adv_data,
        scan_data,
      };

      // 等待 BLE 中心设备连接
      let conn = match peripheral::advertise_connectable(self.sd, adv, &config).await {
        Ok(conn) => conn,
        Err(_e) => {
          warn!("广播错误");
          continue;
        }
      };

      info!("BLE 设备已连接!");

      // 处理 GATT 事件，直到连接断开或收到断开信号
      let gatt_future = gatt_server::run(&conn, &self.server, |event| match event {
        BatteryServerEvent::Battery(battery_event) => match battery_event {
          BatteryServiceEvent::BatteryLevelCccdWrite { notifications } => {
            info!(
              "电池电量通知 {}",
              if notifications {
                "已启用"
              } else {
                "已禁用"
              }
            );
          }
        },
      });

      let disconnect_future = DISCONNECT_SIGNAL.wait();

      // 使用 select 同时等待 GATT 事件循环结束或断开信号
      use embassy_futures::select::{Either, select};
      match select(gatt_future, disconnect_future).await {
        Either::First(_) => {
          info!("连接已断开，重新开始广播...");
        }
        Either::Second(_) => {
          // 收到主动断开信号，断开连接
          // 连接会在 conn 被 drop 时自动断开
          info!("主动断开连接，重新开始广播...");
        }
      }
    }
  }
}

/// SoftDevice 运行任务
///
/// 必须作为独立任务运行，处理 SoftDevice 的内部事件循环
#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
  sd.run().await
}

/// 构建标准的 BLE 广播数据包
///
/// 包含：Flags + 完整设备名称 + 16-bit UUID 列表
///
/// # 参数
/// - `device_name`: 设备名称字节数组
/// - `service_uuid`: 16-bit 服务 UUID（小端序）
///
/// # 返回
/// 广播数据字节数组（最大 31 字节）
pub fn build_adv_data(device_name: &[u8], service_uuid: u16) -> [u8; 31] {
  let mut data = [0u8; 31];
  let mut pos = 0;

  // Flags: LE General Discoverable + BR/EDR Not Supported
  data[pos] = 0x02;
  data[pos + 1] = 0x01;
  data[pos + 2] = raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8;
  pos += 3;

  // 完整设备名称
  let name_len = device_name.len().min(31 - pos - 5); // 预留 UUID 空间
  data[pos] = (1 + name_len) as u8;
  data[pos + 1] = 0x09; // AD Type: Complete Local Name
  pos += 2;
  data[pos..pos + name_len].copy_from_slice(&device_name[..name_len]);
  pos += name_len;

  // 16-bit UUID 列表
  data[pos] = 0x03;
  data[pos + 1] = 0x03; // AD Type: Complete List of 16-bit UUIDs
  data[pos + 2] = (service_uuid & 0xFF) as u8;
  data[pos + 3] = ((service_uuid >> 8) & 0xFF) as u8;

  data
}

/// 构建标准的扫描响应数据包
///
/// 包含制造商特定数据（Nordic Semiconductor）
pub fn build_scan_data() -> [u8; 6] {
  [
    0x05, 0xFF, // AD Type: Manufacturer Specific Data
    0x59, 0x00, // Nordic Semiconductor 公司 ID
    0x01, 0x00, // 自定义数据
  ]
}
