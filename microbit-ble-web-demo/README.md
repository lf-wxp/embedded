# micro:bit V2 BLE Web Demo

A Web Bluetooth console demo project for micro:bit V2, built with Leptos v0.8 and Web Bluetooth API.

## Project Structure

```
microbit-ble-web-demo/
├── index.html          # Main web page (Leptos WASM entry point)
├── web-server/        # Rust static file server
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── src/               # Leptos frontend source (WASM)
│   ├── lib.rs         # App entry point and routing
│   ├── context.rs     # Global shared state (Rc<RefCell<>>)
│   ├── utils.rs       # Protocol constants, CRC, frame encode/decode
│   ├── components/    # UI components
│   │   ├── mod.rs
│   │   ├── connect_buttons.rs  # Connect/Disconnect buttons
│   │   ├── status_indicator.rs # Connection status indicator
│   │   ├── comm_log.rs         # TX/RX communication log
│   │   ├── echo_panel.rs       # Echo loopback test panel
│   │   ├── led_matrix.rs       # LED 5x5 matrix editor
│   │   └── sensor_panel.rs     # Temperature + button events
│   └── services/      # External service modules
│       ├── mod.rs
│       └── ble.rs     # Web Bluetooth API bindings
├── Cargo.toml
├── Makefile.toml      # cargo-make task definitions
├── Trunk.toml         # Trunk WASM build configuration
└── README.md
```

## Features

- 🔵 **Web Bluetooth** - Connect to micro:bit directly from the browser
- 📊 **LED Matrix Control** - Visual 5x5 LED matrix editor
- 🌡️ **Temperature Sensor** - Read micro:bit chip temperature
- 🔘 **Button Events** - Subscribe to and display button A/B status in real-time
- 🔁 **Echo Loopback Test** - Verify data communication
- 📝 **Communication Log** - Real-time display of TX/RX data frames

## Prerequisites

### 1. Install Toolchain

```bash
# Install Rust and wasm32 target
rustup target add wasm32-unknown-unknown

# Install Trunk (WASM web application bundler)
cargo install trunk

# Install cargo-make (task runner, optional but recommended)
cargo install cargo-make
```

### 2. Flash micro:bit Firmware

Ensure your micro:bit V2 has BLE firmware flashed (e.g., the [microbit-ble](../microbit-ble) project).

## Usage

### 1. Using cargo-make (Recommended)

```bash
# Install cargo-make (if not already installed)
cargo install cargo-make

# Build WASM frontend and start server (default http://127.0.0.1:8080)
cargo make serve

# Build release version only
cargo make build

# Build debug version only
cargo make build-debug

# Clean build artifacts
cargo make clean

# Format code
cargo make fmt

# Run Clippy static analysis
cargo make clippy
```

### 2. Using Cargo Directly

```bash
# Navigate to web-server directory
cd web-server

# Build and run (default http://127.0.0.1:8080)
cargo run --release

# Custom port
PORT=9000 cargo run

# Custom static file directory
WEB_ROOT=/path/to/web cargo run
```

### 3. Connect to micro:bit

1. Ensure micro:bit V2 has BLE firmware flashed (e.g., [microbit-ble](../microbit-ble) project)
2. Open `http://127.0.0.1:8080` in your browser
3. Click the "Connect micro:bit" button
4. Select your micro:bit from the device selection dialog
5. After successful connection, all features become available

### 4. Browser Requirements

Web Bluetooth API requires the following environment:
- **Desktop**: Chrome 56+, Edge 79+, Opera 43+
- **Android**: Chrome 56+
- **Not supported**: Safari, Firefox, iOS browsers

> ⚠️ Must be accessed via `localhost` or HTTPS, cannot be opened directly via `file://`.

## Protocol

The web console communicates with the micro:bit firmware using the same binary protocol:

| Command | Value | Description |
|---------|-------|-------------|
| PING | 0x01 | Heartbeat test |
| LED_SET | 0x02 | Set LED matrix |
| LED_CLEAR | 0x03 | Clear LED matrix |
| LED_CHAR | 0x04 | Display character |
| TEMP_GET | 0x05 | Read temperature |
| BTN_SUBSCRIBE | 0x06 | Subscribe to button events |
| ECHO | 0x07 | Echo test |

Frame format: `[SOF(0xAA), CMD, LEN, ...payload, CRC]`

CRC-8 algorithm: `poly 0x07, init 0x00`

The protocol implementation is shared between the firmware (`microbit-ble/src/ble/protocol.rs`) and the web frontend (`microbit-ble-web-demo/src/utils.rs`).

## Related Projects

- [microbit-ble](../microbit-ble) - micro:bit V2 BLE firmware (Rust + Embassy)

## Tech Stack

| Component | Description |
|-----------|-------------|
| Leptos v0.8 | Rust web framework (WASM) |
| Trunk | WASM web application bundler |
| Web Bluetooth API | Browser Bluetooth communication |
| cargo-make | Task runner |

## License

MIT
