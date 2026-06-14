# 嵌入式项目集 (Embedded Projects)

BBC micro:bit V2 的 Rust 嵌入式项目集合，展示了裸机编程、BLE 固件开发和 Web Bluetooth 集成。

[English README](./README.md)

## 项目结构

```
embedded/
├── microbit/                # 裸机 Rust 示例（无操作系统/SoftDevice）
├── microbit-ble/            # BLE 外设固件（Embassy + SoftDevice）
├── microbit-ble-protocol/   # 共享 BLE 二进制协议库
└── microbit-ble-web-demo/   # Web Bluetooth 控制台（Leptos + WASM）
```

## 子项目

### 1. [microbit](./microbit/) — 裸机示例

BBC micro:bit V2 (nRF52833) 的裸机 Rust 示例集合，展示在无操作系统或 SoftDevice 的情况下直接访问外设。

**主要示例：**
- `ambient` — 悟空扩展板环境 LED 呼吸灯效果
- `ble_beacon` — 使用直接 RADIO 外设的裸机 BLE 广播器
- `buzzer` — PWM 蜂鸣器音乐播放（小星星）

**技术栈：** `microbit-v2` HAL, `nrf52833-pac`, `cortex-m-rt`, RTT 日志

---

### 2. [microbit-ble](./microbit-ble/) — BLE 外设固件

基于 Embassy 异步运行时 + nrf-softdevice 的 micro:bit V2 蓝牙 BLE 外设示例。

**功能特性：**
- 🔵 BLE 广播和可连接外设
- 🔋 电池服务 (BAS)
- 📟 带有自定义二进制协议的 Nordic UART 服务 (NUS)
- 🔘 按钮 A/B 事件处理
- 💡 LED 5x5 矩阵驱动

**技术栈：** Embassy, nrf-softdevice S113, defmt + RTT, probe-rs

**前置要求：**
```bash
# 烧录 SoftDevice（仅首次需要）
probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex

# 构建并运行
cargo run --release
```

---

### 3. [microbit-ble-protocol](./microbit-ble-protocol/) — 共享协议库

定义 micro:bit 固件和 Web 前端之间通信所用二进制协议的共享库。

**协议帧格式：**
```
[SOF(0xAA), CMD, LEN, ...payload, CRC]
```

**命令：**

| 命令 | 值 | 描述 |
|---------|-------|-------------|
| PING | 0x01 | 心跳测试 |
| LED_SET | 0x02 | 设置 LED 矩阵 |
| LED_CLEAR | 0x03 | 清除 LED 矩阵 |
| LED_CHAR | 0x04 | 显示字符 |
| TEMP_GET | 0x05 | 读取温度 |
| BTN_SUBSCRIBE | 0x06 | 订阅按钮事件 |
| ECHO | 0x07 | 回显测试 |

CRC-8 算法：`poly 0x07, init 0x00`

---

### 4. [microbit-ble-web-demo](./microbit-ble-web-demo/) — Web Bluetooth 控制台

使用 Leptos v0.8 和 WebAssembly 构建的 Web 应用，通过 Web Bluetooth API 连接到 micro:bit。

**功能特性：**
- 🔵 Web Bluetooth 连接
- 📊 可视化 5x5 LED 矩阵编辑器
- 🌡️ 温度传感器读取
- 🔘 实时按钮事件显示
- 🔁 回显环回测试
- 📝 实时 TX/RX 通信日志

**技术栈：** Leptos v0.8, Trunk, Web Bluetooth API, Rust WASM

**使用方法：**
```bash
# 安装 trunk
cargo install trunk

# 启动热重载服务
cargo make serve

# 打开 http://127.0.0.1:8080
```

> ⚠️ **浏览器兼容性：** Web Bluetooth 需要 Chrome/Edge/Opera（桌面或 Android）。不支持 Safari、Firefox 或 iOS 浏览器。

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                     Web 浏览器 (WASM)                        │
│  microbit-ble-web-demo (Leptos + Web Bluetooth API)        │
└──────────────────────────┬──────────────────────────────────┘
                           │ Web Bluetooth
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  BBC micro:bit V2 (nRF52833)                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  SoftDevice S113 (BLE 协议栈)                         │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  microbit-ble (Embassy 异步固件)                      │   │
│  │  - NUS (Nordic UART 服务)                           │   │
│  │  - 电池服务                                           │   │
│  │  - LED 矩阵驱动                                       │   │
│  │  - 按钮事件                                           │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 前置要求

### 通用工具

```bash
# 安装 Rust 嵌入式目标平台
rustup target add thumbv7em-none-eabihf

# 安装 WASM 目标平台（用于 Web 演示）
rustup target add wasm32-unknown-unknown

# 安装 probe-rs（烧录/调试）
cargo install probe-rs-tools

# 安装 Trunk（WASM 打包器）
cargo install trunk

# 安装 cargo-make（任务运行器）
cargo install cargo-make
```

### 硬件要求

- BBC micro:bit V2 开发板
- J-Link 或 CMSIS-DAP 调试器（用于固件烧录）
- （可选）微雪悟空扩展板

## 快速开始

1. **烧录 BLE 固件：**
   ```bash
   cd microbit-ble
   # 烧录 SoftDevice（仅首次需要）
   probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex
   # 烧录应用程序
   cargo run --release
   ```

2. **启动 Web 演示：**
   ```bash
   cd microbit-ble-web-demo
   cargo make serve
   ```

3. **通过浏览器连接：**
   - 打开 `http://127.0.0.1:8080`
   - 点击"连接 micro:bit"
   - 选择你的设备

## 许可证

MIT

## 相关资源

- [BBC micro:bit V2 文档](https://tech.microbit.org/hardware/)
- [Nordic nRF52833 文档](https://www.nordicsemi.com/Products/nRF52833)
- [Embassy 框架](https://embassy.dev/)
- [Leptos 框架](https://leptos.dev/)
- [Web Bluetooth API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Bluetooth_API)
