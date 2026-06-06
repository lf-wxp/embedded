# micro:bit V2 BLE Web Demo

基于 Web Bluetooth API 的 micro:bit V2 蓝牙控制台演示项目。

## 项目结构

```
microbit-ble-web-demo/
├── index.html          # Web Bluetooth 控制台页面
├── web-server/        # Rust 静态文件服务器
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── README.md
└── .gitignore
```

## 功能特性

- 🔵 **Web Bluetooth** - 通过浏览器直接连接 micro:bit
- 📊 **LED 矩阵控制** - 可视化 5×5 LED 矩阵编辑器
- 🌡️ **温度传感器** - 读取 micro:bit 芯片温度
- 🔘 **按键事件** - 订阅并实时显示按键 A/B 状态
- 🔁 **Echo 回环测试** - 验证数据通信
- 📝 **通信日志** - 实时显示 TX/RX 数据帧

## 使用方法

### 1. 使用 cargo-make 启动 Web 服务器（推荐）

```bash
# 安装 cargo-make（如果尚未安装）
cargo install cargo-make

# 编译并启动服务器（默认 http://127.0.0.1:8080）
cargo make serve

# 仅编译 Release 版本
cargo make build

# 仅编译 Debug 版本
cargo make build-debug

# 清理构建产物
cargo make clean

# 代码格式化
cargo make fmt

# 运行 Clippy 静态分析
cargo make clippy
```

### 2. 使用 Cargo 直接启动 Web 服务器

```bash
# 进入 web-server 目录
cd web-server

# 编译并运行（默认 http://127.0.0.1:8080）
cargo run --release

# 自定义端口
PORT=9000 cargo run

# 自定义静态文件目录
WEB_ROOT=/path/to/web cargo run
```

### 2. 连接 micro:bit

1. 确保 micro:bit V2 已烧录支持 BLE 的固件（如 [microbit-ble](../microbit-ble) 项目）
2. 在浏览器中打开 `http://127.0.0.1:8080`
3. 点击「连接 micro:bit」按钮
4. 在弹出的设备选择对话框中选择你的 micro:bit
5. 连接成功后即可使用各项功能

### 3. 浏览器要求

Web Bluetooth API 需要以下环境：
- **桌面端**: Chrome 56+, Edge 79+, Opera 43+
- **Android**: Chrome 56+
- **不支持**: Safari, Firefox, iOS 浏览器

> ⚠️ 必须通过 `localhost` 或 HTTPS 访问，不能直接用 `file://` 打开。

## 协议说明

Web 控制台与 micro:bit 固件使用相同的二进制协议通信：

| 命令 | 值 | 说明 |
|------|-----|------|
| PING | 0x01 | 心跳测试 |
| LED_SET | 0x02 | 设置 LED 矩阵 |
| LED_CLEAR | 0x03 | 清空 LED 矩阵 |
| LED_CHAR | 0x04 | 显示字符 |
| TEMP_GET | 0x05 | 读取温度 |
| BTN_SUBSCRIBE | 0x06 | 订阅按键事件 |
| ECHO | 0x07 | 回显测试 |

帧格式：`[SOF(0xAA), CMD, LEN, ...payload, CRC]`

CRC-8 算法：`poly 0x07, init 0x00`

## 相关项目

- [microbit-ble](../microbit-ble) - micro:bit V2 BLE 固件（Rust + Embassy）

## 许可证

MIT
