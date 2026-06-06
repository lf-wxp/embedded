# microbit Bare-Metal Examples

A collection of bare-metal Rust examples for the BBC micro:bit V2 (nRF52833), demonstrating direct peripheral access without an OS or SoftDevice.

## Requirements

- Rust toolchain with `thumbv7em-none-eabihf` target
- A BBC micro:bit V2 board
- A J-Link or CMSIS-DAP debugger (for flashing and RTT logging)
- (Optional) Waveshare WuKong Expansion Board — required for `ambient` and `buzzer` examples

## Building

```bash
# Add the target
rustup target add thumbv7em-none-eabihf

# Build a specific example
cargo build --example ambient --release

# Flash with cargo-embed
cargo embed --example ambient --release
```

> **Note:** RTT logging requires `rtt_target`. Make sure your debugger supports RTT (e.g., J-Link RTT Viewer).

## Examples

### `ambient` — WuKong Ambient LED Breathing

Demonstrates controlling the WuKong Expansion Board's ambient LED (PWM-driven RGB) with button-triggered start/stop and a timer interrupt for color cycling.

- **Button A** — toggles the LED breathing effect on/off
- **TIMER1 interrupt** — cycles the red channel every 20 ms when running
- **Debounce** — uses TIMER0 for 100 ms software debounce

### `ble_beacon` — Bare-Metal BLE Advertiser

Sends BLE non-connectable advertising packets (`ADV_NONCONN_IND`) by directly programming the nRF52833 RADIO peripheral — **no SoftDevice**. Scannable by nRF Connect or similar BLE tools.

- Broadcasts on all 3 BLE advertising channels (37 / 38 / 39)
- Advertised name: `MBit-Beacon`
- Interval: 1000 ms
- **Limitations:** non-connectable, no GATT, no full BLE stack — for learning RADIO internals only

### `buzzer` — WuKong Buzzer (Twinkle Twinkle Little Star)

Plays *Twinkle Twinkle Little Star* on the WuKong Expansion Board's buzzer via PWM tone generation.

- Uses MIDI note numbers for pitch mapping
- Note durations defined as quarter / half notes (500 ms / 1000 ms)

## Project Structure

```
microbit/
├── src/
│   └── main.rs          # Minimal no-op bare-metal entry point
├── examples/
│   ├── ambient.rs        # Button + ambient LED + timer interrupt
│   ├── ble_beacon.rs     # Direct RADIO BLE advertising
│   └── buzzer.rs        # PWM buzzer music playback
├── vendor/
│   ├── mb2-wukong-expansion/
│   └── ws2812-nrf52833-pwm/
├── Cargo.toml
├── Embed.toml
└── .cargo/config.toml
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `microbit-v2` | micro:bit V2 HAL |
| `nrf52833-pac` | nRF52833 peripheral access crate |
| `cortex-m-rt` | Cortex-M runtime |
| `rtt-target` / `panic-rtt-target` | RTT logging and panic output |
| `mb2-wukong-expansion` | WuKong Expansion Board driver (local vendor crate) |
| `tiny-led-matrix` | micro:bit 5x5 LED matrix |
| `embedded-hal` | Embedded HAL traits |

## License

[Add your license here]
