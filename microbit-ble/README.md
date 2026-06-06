# micro:bit V2 BLE Project

A micro:bit V2 Bluetooth BLE peripheral example based on the Embassy async runtime + nrf-softdevice.

## Project Structure

```
microbit-ble/
├── .cargo/config.toml   # Compile target and linker configuration
├── Cargo.toml           # Project dependencies
├── Dockerfile           # Docker build support
├── Makefile.toml        # cargo-make task definitions
├── Embed.toml           # probe-rs flash configuration
├── build.rs             # Build script (handles memory.x)
├── memory.x             # Linker script (reserves memory for SoftDevice)
└── src/
    ├── main.rs          # BLE peripheral broadcast example
    └── ble/             # BLE module directory
        ├── mod.rs       # Module declarations and BleController
        ├── buttons.rs   # Button A/B event handling
        ├── commands.rs  # Command dispatch and protocol handling
        ├── led_matrix.rs # LED 5x5 matrix driver (row scanning)
        ├── nus.rs       # Nordic UART Service (NUS) implementation
        ├── battery.rs   # Battery Service (BAS) implementation
        └── protocol.rs  # Binary protocol encode/decode
```

## Prerequisites

### 1. Install Toolchain

```bash
# Install Rust embedded target
rustup target add thumbv7em-none-eabihf

# Install probe-rs (for flashing and debugging)
cargo install probe-rs-tools
```

### 2. Download and Flash SoftDevice S113

**Important: You must flash the SoftDevice firmware to micro:bit V2 before running the application code.**

SoftDevice is Nordic's Bluetooth protocol stack binary firmware that occupies the lower address region of Flash.

```bash
# 1. Download SoftDevice S113 v7.3.0 from Nordic's official website
#    Download URL: https://www.nordicsemi.com/Products/Development-software/s113/download
#    Extract to get s113_nrf52_7.3.0_softdevice.hex

# 2. Flash SoftDevice to micro:bit V2 using probe-rs
probe-rs download --chip nRF52833_xxAA --format hex s113_nrf52_7.3.0_softdevice.hex

# 3. Verify flash succeeded (optional)
probe-rs info --chip nRF52833_xxAA
```

> **Note**: SoftDevice only needs to be flashed once. Subsequent application code updates will not overwrite it (because application code starts at 0x26000 in memory.x).

## Building and Running

```bash
# Build (debug mode)
cargo build

# Build and flash to micro:bit V2
cargo run

# Build release version and flash
cargo run --release
```

## Usage

1. Flash SoftDevice (first time only)
2. Build and flash application code (`cargo run`)
3. Open the **nRF Connect** app on your phone (iOS/Android)
4. Scan for BLE devices, find the device named **"MicroBit-BLE"**
5. Click to connect, you can see Battery Service (0x180F)
6. Read the Battery Level characteristic value

## Memory Layout

```
Flash (512KB):
┌──────────────────────────────────┐ 0x00080000
│                                  │
│      Application Code (360KB)    │
│                                  │
├──────────────────────────────────┤ 0x00026000
│                                  │
│    SoftDevice S113 (152KB)       │
│                                  │
└──────────────────────────────────┘ 0x00000000

RAM (128KB):
┌──────────────────────────────────┐ 0x20020000
│                                  │
│       Application RAM (~117KB)    │
│                                  │
├──────────────────────────────────┤ 0x20002AD8
│     SoftDevice RAM (~11KB)       │
└──────────────────────────────────┘ 0x20000000
```

## Architecture

This project uses **Embassy** (async runtime) + **nrf-softdevice** (BLE stack) and is therefore **incompatible** with the `microbit` crate (which is based on `nrf52833-hal`, a blocking HAL). The `nrf-softdevice` SoftDevice requires exclusive control of clocks and interrupts, so mixing it with `nrf52833-hal` would cause conflicts.

## Tech Stack

| Component | Description |
|-----------|-------------|
| Embassy | Rust embedded async runtime |
| nrf-softdevice | Rust bindings for Nordic SoftDevice |
| SoftDevice S113 | Nordic BLE protocol stack (peripheral role only) |
| defmt + RTT | Efficient embedded logging system |
| probe-rs | Flashing and debugging tool |

## Protocol

The device communicates via the **Nordic UART Service (NUS)** using a custom binary protocol:

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

## FAQ

### Q: Panic or HardFault at runtime?
- Verify SoftDevice is correctly flashed
- Verify the RAM start address in memory.x matches the actual SoftDevice usage
- If SoftDevice reports insufficient RAM, increase the RAM ORIGIN address

### Q: Phone cannot scan the device?
- Verify the code was successfully flashed (RTT log should show "BLE advertising started")
- Verify phone Bluetooth is enabled
- Try restarting the micro:bit

### Q: How to switch to S140 (supports central role)?
- Modify features in Cargo.toml: `s113` -> `s140`
- Replace `nrf-softdevice-s113` dependency with `nrf-softdevice-s140`
- Download and flash S140 SoftDevice
- Update addresses in memory.x (S140 occupies more Flash/RAM)

## Docker Support

```bash
# Build Docker image
cargo make docker-build

# Run with docker-compose
cargo make docker-run

# View logs
cargo make docker-logs

# Stop container
cargo make docker-stop
```

## License

MIT
