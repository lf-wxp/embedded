//! # Bare-Metal BLE Beacon
//!
//! This example demonstrates how to send BLE advertising packets (ADV_NONCONN_IND)
//! by directly programming the nRF52833 RADIO peripheral **without using the SoftDevice**.
//! BLE scanning tools like nRF Connect on a phone can discover this beacon device.
//!
//! ## How It Works
//!
//! BLE advertising uses 3 fixed frequency channels (Channel 37/38/39), corresponding
//! to 2402/2426/2480 MHz. We directly configure the RADIO peripheral for BLE 1Mbit mode,
//! construct a broadcast PDU that complies with the BLE specification, and then transmit
//! sequentially on all 3 advertising channels.
//!
//! ## Notes
//!
//! - This is a **non-connectable** beacon (ADV_NONCONN_IND), for demonstration only
//! - Does not implement a full BLE protocol stack; cannot be connected to
//! - Suitable for learning the low-level workings of the RADIO peripheral
//! - For full BLE functionality (connection, GATT services, etc.), use the SoftDevice approach

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::hal::Timer;
use nrf52833_pac::Peripherals;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

/// BLE advertising channel frequency offsets (relative to 2400 MHz base frequency)
/// Channel 37 = 2402 MHz, Channel 38 = 2426 MHz, Channel 39 = 2480 MHz
const BLE_ADV_CHANNEL_FREQ: [u8; 3] = [2, 26, 80];

/// BLE advertising channel Access Address
/// All advertising packets use the fixed access address 0x8E89BED6
/// (configured via BASE0 + PREFIX0 registers; no separate constant needed)

/// Advertising interval (milliseconds)
const ADV_INTERVAL_MS: u32 = 1000;

/// Device name
const DEVICE_NAME: &[u8] = b"MBit-Beacon";

/// BLE advertising PDU buffer
/// Format: [Header(2B)] [AdvA(6B)] [AD Structures...]
/// Note: The RADIO peripheral automatically handles preamble and CRC; we only need to provide the PDU portion
#[repr(C, align(4))]
struct AdvPacket {
    /// PDU data (max 39 bytes: 2B header + 37B payload)
    data: [u8; 39],
    /// Actual length used
    len: usize,
}

impl AdvPacket {
    /// Construct an ADV_NONCONN_IND advertising packet
    ///
    /// PDU format:
    /// - Header[0]: PDU Type (4bit) | RFU (1bit) | ChSel (1bit) | TxAdd (1bit) | RxAdd (1bit)
    /// - Header[1]: Length (payload length)
    /// - Payload: AdvA (6B) + AdvData (0-31B)
    fn new_nonconn_ind(adv_addr: &[u8; 6], adv_data: &[u8]) -> Self {
        let mut pkt = AdvPacket {
            data: [0u8; 39],
            len: 0,
        };

        // PDU Header
        // ADV_NONCONN_IND = 0b0010, TxAdd = 1 (random address)
        pkt.data[0] = 0x02 | (1 << 6); // PDU Type = ADV_NONCONN_IND, TxAdd = Random
        let payload_len = 6 + adv_data.len();
        pkt.data[1] = payload_len as u8;

        // AdvA: advertising address (6 bytes, little-endian)
        pkt.data[2..8].copy_from_slice(adv_addr);

        // AdvData: advertising data
        pkt.data[8..8 + adv_data.len()].copy_from_slice(adv_data);

        pkt.len = 2 + payload_len;
        pkt
    }
}

/// Build advertising data (AD Structures)
/// Contains: Flags + Complete Local Name
fn build_adv_data(buf: &mut [u8]) -> usize {
    let mut pos = 0;

    // AD Structure 1: Flags
    // Length = 2, Type = 0x01 (Flags)
    // Value = 0x06 (LE General Discoverable + BR/EDR Not Supported)
    buf[pos] = 0x02; // Length
    buf[pos + 1] = 0x01; // AD Type: Flags
    buf[pos + 2] = 0x06; // LE General Discoverable | BR/EDR Not Supported
    pos += 3;

    // AD Structure 2: Complete Local Name
    // Length = 1 + name_len, Type = 0x09
    buf[pos] = (1 + DEVICE_NAME.len()) as u8; // Length
    buf[pos + 1] = 0x09; // AD Type: Complete Local Name
    buf[pos + 2..pos + 2 + DEVICE_NAME.len()].copy_from_slice(DEVICE_NAME);
    pos += 2 + DEVICE_NAME.len();

    // AD Structure 3: TX Power Level (optional)
    // Length = 2, Type = 0x0A, Value = 0 dBm
    buf[pos] = 0x02; // Length
    buf[pos + 1] = 0x0A; // AD Type: TX Power Level
    buf[pos + 2] = 0x00; // 0 dBm
    pos += 3;

    pos
}

/// Configure RADIO peripheral for BLE 1Mbit mode
fn radio_init(radio: &nrf52833_pac::RADIO) {
    // Disable RADIO
    radio.tasks_disable.write(|w| unsafe { w.bits(1) });
    while radio.events_disabled.read().bits() == 0 {}
    radio.events_disabled.write(|w| unsafe { w.bits(0) });

    // Set mode to BLE 1Mbit
    radio.mode.write(|w| w.mode().ble_1mbit());

    // Configure PCNF0 (Packet Configuration 0)
    // BLE: LENGTH field 8 bit, at bit 0, S0 = 1 byte, S1 = 0
    radio.pcnf0.write(|w| unsafe {
        w.lflen().bits(8)    // LENGTH field length: 8 bits
         .s0len().bit(true)  // S0 field length: 1 byte (first byte of PDU header)
         .s1len().bits(0)    // S1 field length: 0
         .plen()._8bit()     // Preamble length: 8 bit (BLE 1Mbit)
    });

    // Configure PCNF1 (Packet Configuration 1)
    // MAXLEN = 37 (max BLE advertising payload length)
    // STATLEN = 0, BALEN = 3 (base address length = 3+1 = 4 bytes)
    // ENDIAN = Little, WHITEEN = 1 (enable data whitening)
    radio.pcnf1.write(|w| unsafe {
        w.maxlen().bits(37)     // Max payload length
         .statlen().bits(0)     // Static length appendix: 0
         .balen().bits(3)       // Base address length: 3 (actual = 3+1 = 4 bytes)
         .endian().little()     // Little-endian
         .whiteen().set_bit()   // Enable data whitening
    });

    // Configure Access Address
    // BLE advertising always uses 0x8E89BED6
    // BASE0 = lower 4 bytes of address (byte-reversed), PREFIX0 = highest byte of address
    // Access Address 0x8E89BED6 -> over-the-air order (LSB first): D6 BE 89 8E
    // RADIO config: PREFIX0[0] = 0x8E, BASE0 = 0x89BED600 (shifted left by 8 bits)
    radio.base0.write(|w| unsafe { w.bits(0x89BED600) });
    radio.prefix0.write(|w| unsafe { w.bits(0x0000008E) });

    // Use logical address 0 for transmission
    radio.txaddress.write(|w| unsafe { w.txaddress().bits(0) });

    // Configure CRC
    // BLE uses 24-bit CRC, polynomial = 0x00065B, initial value = 0x555555
    radio.crccnf.write(|w| w.len().three().skipaddr().skip());
    radio.crcpoly.write(|w| unsafe { w.crcpoly().bits(0x0006_5B) });
    radio.crcinit.write(|w| unsafe { w.crcinit().bits(0x55_5555) });

    // Configure TX power: 0 dBm
    radio.txpower.write(|w| w.txpower()._0d_bm());

    // Configure shortcuts: READY -> START, END -> DISABLE
    radio.shorts.write(|w| {
        w.ready_start().enabled()
         .end_disable().enabled()
    });
}

/// Send an advertising packet on the specified advertising channel
fn radio_send(radio: &nrf52833_pac::RADIO, packet: &AdvPacket, channel_idx: usize) {
    // Set frequency
    radio.frequency.write(|w| unsafe {
        w.frequency().bits(BLE_ADV_CHANNEL_FREQ[channel_idx] as u8)
    });

    // Set data whitening initial value (based on channel number)
    // BLE whitening LFSR initial value = channel_index | 0x40
    let channel_num = match channel_idx {
        0 => 37u8,
        1 => 38,
        2 => 39,
        _ => 37,
    };
    radio.datawhiteiv.write(|w| unsafe {
        w.datawhiteiv().bits(channel_num | 0x40)
    });

    // Set packet pointer (points to PDU data)
    radio.packetptr.write(|w| unsafe {
        w.bits(packet.data.as_ptr() as u32)
    });

    // Clear events
    radio.events_disabled.write(|w| unsafe { w.bits(0) });

    // Start transmission (TXEN -> READY -> START -> END -> DISABLE automated via shorts)
    radio.tasks_txen.write(|w| unsafe { w.bits(1) });

    // Wait for transmission complete (DISABLED event)
    while radio.events_disabled.read().bits() == 0 {}
    radio.events_disabled.write(|w| unsafe { w.bits(0) });
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== Bare-Metal BLE Beacon ===");
    rprintln!("Device name: {}", core::str::from_utf8(DEVICE_NAME).unwrap_or("?"));
    rprintln!("Advertising interval: {} ms", ADV_INTERVAL_MS);
    rprintln!("Note: This is a non-connectable advertisement (ADV_NONCONN_IND)");

    let periph = Peripherals::take().unwrap();

    // Enable high-frequency clock (HFCLK); RADIO requires the external high-frequency crystal
    periph.CLOCK.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
    while periph.CLOCK.events_hfclkstarted.read().bits() == 0 {}
    periph.CLOCK.events_hfclkstarted.write(|w| unsafe { w.bits(0) });
    rprintln!("High-frequency clock started (HFXO)");

    // Initialize RADIO
    radio_init(&periph.RADIO);
    rprintln!("RADIO initialized (BLE 1Mbit mode)");

    // Generate a Random Static Address
    // Format: top 2 bits = 11 (identifies as random static address)
    // Using a fixed address here for demonstration; in practice, use the device address from FICR or RNG
    let adv_addr: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xDE]; // MSB bit[7:6] = 11

    // Build advertising data
    let mut adv_data_buf = [0u8; 31];
    let adv_data_len = build_adv_data(&mut adv_data_buf);

    // Construct advertising packet
    let packet = AdvPacket::new_nonconn_ind(&adv_addr, &adv_data_buf[..adv_data_len]);
    rprintln!("Advertising packet constructed, PDU length: {} bytes", packet.len);

    // Initialize timer for advertising interval
    let mut timer = Timer::new(periph.TIMER0);
    let mut adv_count: u32 = 0;

    rprintln!("Starting advertising...");
    rprintln!("Use nRF Connect or similar BLE scanning tool to view device \"{}\"",
        core::str::from_utf8(DEVICE_NAME).unwrap_or("?"));

    loop {
        // Transmit sequentially on all 3 advertising channels
        for ch in 0..3 {
            radio_send(&periph.RADIO, &packet, ch);
        }

        adv_count += 1;
        if adv_count % 10 == 0 {
            rprintln!("Advertised {} times", adv_count);
        }

        // Wait for advertising interval
        timer.delay_ms(ADV_INTERVAL_MS);
    }
}
