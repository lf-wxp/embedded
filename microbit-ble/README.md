# micro:bit V2 BLE 蓝牙项目

基于 Embassy 异步运行时 + nrf-softdevice 的 micro:bit V2 蓝牙 BLE 外设示例。

## 项目结构

```
microbit-ble/
├── .cargo/config.toml   # 编译目标和链接配置
├── Cargo.toml           # 项目依赖
├── Embed.toml           # probe-rs 烧录配置
├── build.rs             # 构建脚本（处理 memory.x）
├── memory.x             # 链接脚本（为 SoftDevice 预留内存）
└── src/
    └── main.rs          # BLE 外设广播示例
```

## 前置条件

### 1. 安装工具链

```bash
# 安装 Rust 嵌入式目标
rustup target add thumbv7em-none-eabihf

# 安装 probe-rs（用于烧录和调试）
cargo install probe-rs-tools
```

### 2. 下载并烧录 SoftDevice S113

**重要：必须先烧录 SoftDevice 固件到 micro:bit V2，然后才能运行本项目的应用代码。**

SoftDevice 是 Nordic 的蓝牙协议栈二进制固件，占用 Flash 的低地址区域。

```bash
# 1. 从 Nordic 官网下载 SoftDevice S113 v7.3.0
#    下载地址: https://www.nordicsemi.com/Products/Development-software/s113/download
#    解压后得到 s113_nrf52_7.3.0_softdevice.hex

# 2. 使用 probe-rs 烧录 SoftDevice 到 micro:bit V2
probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex

# 3. 验证烧录成功（可选）
probe-rs info --chip nRF52833_xxAA
```

> **注意**: SoftDevice 只需烧录一次，之后应用代码的更新不会覆盖它（因为 memory.x 中应用代码从 0x26000 开始）。

## 编译和运行

```bash
# 编译（debug 模式）
cargo build

# 编译并烧录到 micro:bit V2
cargo run

# 编译 release 版本并烧录
cargo run --release
```

## 使用方法

1. 烧录 SoftDevice（仅首次需要）
2. 编译并烧录应用代码 (`cargo run`)
3. 打开手机上的 **nRF Connect** 应用（iOS/Android 均可）
4. 扫描 BLE 设备，找到名为 **"MicroBit-BLE"** 的设备
5. 点击连接，可以看到 Battery Service (0x180F)
6. 读取 Battery Level 特征值

## 内存布局说明

```
Flash (512KB):
┌──────────────────────────────────┐ 0x00080000
│                                  │
│        应用代码 (360KB)           │
│                                  │
├──────────────────────────────────┤ 0x00026000
│                                  │
│     SoftDevice S113 (152KB)      │
│                                  │
└──────────────────────────────────┘ 0x00000000

RAM (128KB):
┌──────────────────────────────────┐ 0x20020000
│                                  │
│        应用 RAM (~117KB)          │
│                                  │
├──────────────────────────────────┤ 0x20002AD8
│     SoftDevice RAM (~11KB)       │
└──────────────────────────────────┘ 0x20000000
```

## 技术栈

| 组件 | 说明 |
|------|------|
| Embassy | Rust 嵌入式异步运行时 |
| nrf-softdevice | Nordic SoftDevice 的 Rust 绑定 |
| SoftDevice S113 | Nordic BLE 协议栈（仅外设角色） |
| defmt + RTT | 高效的嵌入式日志系统 |
| probe-rs | 烧录和调试工具 |

## 常见问题

### Q: 运行时 panic 或 HardFault？
- 确认 SoftDevice 已正确烧录
- 确认 memory.x 中的 RAM 起始地址与 SoftDevice 实际使用量匹配
- 如果 SoftDevice 报告 RAM 不足，需要增大 RAM ORIGIN 地址

### Q: 手机扫描不到设备？
- 确认代码已成功烧录（RTT 日志应显示 "开始 BLE 广播"）
- 确认手机蓝牙已开启
- 尝试重启 micro:bit

### Q: 如何切换到 S140（支持中心角色）？
- 修改 Cargo.toml 中的 features: `s113` → `s140`
- 添加 `nrf-softdevice-s140` 依赖替换 `nrf-softdevice-s113`
- 下载并烧录 S140 SoftDevice
- 更新 memory.x 中的地址（S140 占用更多 Flash/RAM）
