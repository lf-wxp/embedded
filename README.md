# Embedded Projects

[中文文档](./README_zh.md)

A collection of Rust embedded projects for BBC micro:bit V2, demonstrating bare-metal programming, BLE firmware development, and Web Bluetooth integration.

## Project Structure

```
embedded/
├── microbit/                # Bare-metal Rust examples (no OS/SoftDevice)
├── microbit-ble/            # BLE peripheral firmware (Embassy + SoftDevice)
├── microbit-ble-protocol/   # Shared BLE binary protocol crate
└── microbit-ble-web-demo/   # Web Bluetooth console (Leptos + WASM)
```

## Sub-Projects

### 1. [microbit](./microbit/) — Bare-Metal Examples

A collection of bare-metal Rust examples for BBC micro:bit V2 (nRF52833), demonstrating direct peripheral access without an OS or SoftDevice.

**Key Examples:**
- `ambient` — WuKong Expansion Board ambient LED breathing effect
- `ble_beacon` — Bare-metal BLE advertiser using direct RADIO peripheral
- `buzzer` — PWM buzzer music playback (Twinkle Twinkle Little Star)

**Tech Stack:** `microbit-v2` HAL, `nrf52833-pac`, `cortex-m-rt`, RTT logging

---

### 2. [microbit-ble](./microbit-ble/) — BLE Peripheral Firmware

A micro:bit V2 Bluetooth BLE peripheral example based on the Embassy async runtime + nrf-softdevice.

**Features:**
- 🔵 BLE advertising and connectable peripheral
- 🔋 Battery Service (BAS)
- 📟 Nordic UART Service (NUS) with custom binary protocol
- 🔘 Button A/B event handling
- 💡 LED 5x5 matrix driver

**Tech Stack:** Embassy, nrf-softdevice S113, defmt + RTT, probe-rs

**Prerequisites:**
```bash
# Flash SoftDevice (first time only)
probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex

# Build and run
cargo run --release
```

---

### 3. [microbit-ble-protocol](./microbit-ble-protocol/) — Shared Protocol Crate

A shared library defining the binary protocol used for communication between the micro:bit firmware and web frontend.

**Protocol Frame Format:**
```
[SOF(0xAA), CMD, LEN, ...payload, CRC]
```

**Commands:**

| Command | Value | Description |
|---------|-------|-------------|
| PING | 0x01 | Heartbeat test |
| LED_SET | 0x02 | Set LED matrix |
| LED_CLEAR | 0x03 | Clear LED matrix |
| LED_CHAR | 0x04 | Display character |
| TEMP_GET | 0x05 | Read temperature |
| BTN_SUBSCRIBE | 0x06 | Subscribe to button events |
| ECHO | 0x07 | Echo test |

CRC-8 algorithm: `poly 0x07, init 0x00`

---

### 4. [microbit-ble-web-demo](./microbit-ble-web-demo/) — Web Bluetooth Console

A web application built with Leptos v0.8 and WebAssembly that connects to the micro:bit via Web Bluetooth API.

**Features:**
- 🔵 Web Bluetooth connectivity
- 📊 Visual 5x5 LED matrix editor
- 🌡️ Temperature sensor reading
- 🔘 Real-time button event display
- 🔁 Echo loopback test
- 📝 Real-time TX/RX communication log

**Tech Stack:** Leptos v0.8, Trunk, Web Bluetooth API, Rust WASM

**Usage:**
```bash
# Install trunk
cargo install trunk

# Serve with hot-reload
cargo make serve

# Open http://127.0.0.1:8080
```

> ⚠️ **Browser Compatibility:** Web Bluetooth requires Chrome/Edge/Opera (desktop or Android). Not supported on Safari, Firefox, or iOS browsers.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Web Browser (WASM)                        │
│  microbit-ble-web-demo (Leptos + Web Bluetooth API)        │
└──────────────────────────┬──────────────────────────────────┘
                           │ Web Bluetooth
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                  BBC micro:bit V2 (nRF52833)                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  SoftDevice S113 (BLE Protocol Stack)                │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  microbit-ble (Embassy async firmware)               │   │
│  │  - NUS (Nordic UART Service)                        │   │
│  │  - Battery Service                                   │   │
│  │  - LED Matrix Driver                                 │   │
│  │  - Button Events                                     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

### Common Tools

```bash
# Install Rust embedded target
rustup target add thumbv7em-none-eabihf

# Install WASM target (for web demo)
rustup target add wasm32-unknown-unknown

# Install probe-rs (flashing/debugging)
cargo install probe-rs-tools

# Install Trunk (WASM bundler)
cargo install trunk

# Install cargo-make (task runner)
cargo install cargo-make
```

### Hardware Requirements

- BBC micro:bit V2 board
- J-Link or CMSIS-DAP debugger (for firmware flashing)
- (Optional) Waveshare WuKong Expansion Board

## Getting Started

1. **Flash the BLE firmware:**
   ```bash
   cd microbit-ble
   # Flash SoftDevice (first time only)
   probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex
   # Flash application
   cargo run --release
   ```

2. **Start the web demo:**
   ```bash
   cd microbit-ble-web-demo
   cargo make serve
   ```

3. **Connect via browser:**
   - Open `http://127.0.0.1:8080`
   - Click "Connect micro:bit"
   - Select your device

## License

MIT

## Resources

- [BBC micro:bit V2 Documentation](https://tech.microbit.org/hardware/)
- [Nordic nRF52833 Documentation](https://www.nordicsemi.com/Products/nRF52833)
- [Embassy Framework](https://embassy.dev/)
- [Leptos Framework](https://leptos.dev/)
- [Web Bluetooth API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Bluetooth_API)
