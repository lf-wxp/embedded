//! # 裸机 BLE 广播信标 (Bare-metal BLE Beacon)
//!
//! 本示例演示如何在 **不使用 SoftDevice** 的情况下，直接操作 nRF52833 的 RADIO 外设
//! 发送 BLE 广播包（ADV_NONCONN_IND）。手机上的 nRF Connect 等 BLE 扫描工具可以
//! 扫描到该信标设备。
//!
//! ## 原理说明
//!
//! BLE 广播使用 3 个固定频率通道（Channel 37/38/39），对应 2402/2426/2480 MHz。
//! 我们直接配置 RADIO 外设为 BLE 1Mbit 模式，构造符合 BLE 规范的广播 PDU，
//! 然后在 3 个广播通道上依次发送。
//!
//! ## 注意事项
//!
//! - 这是一个**不可连接**的广播信标（ADV_NONCONN_IND），仅用于演示
//! - 没有实现完整的 BLE 协议栈，无法被连接
//! - 适合学习 RADIO 外设的底层工作原理
//! - 如需完整 BLE 功能（连接、GATT 服务等），请使用 SoftDevice 方案

#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::hal::Timer;
use nrf52833_pac::Peripherals;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

/// BLE 广播通道频率偏移量（相对于 2400 MHz 基频）
/// Channel 37 = 2402 MHz, Channel 38 = 2426 MHz, Channel 39 = 2480 MHz
const BLE_ADV_CHANNEL_FREQ: [u8; 3] = [2, 26, 80];

/// BLE 广播通道的访问地址（Access Address）
/// 所有广播包使用固定的访问地址 0x8E89BED6
/// （通过 BASE0 + PREFIX0 寄存器配置，不需要单独的常量）

/// 广播间隔（毫秒）
const ADV_INTERVAL_MS: u32 = 1000;

/// 设备名称
const DEVICE_NAME: &[u8] = b"MBit-Beacon";

/// BLE 广播 PDU 缓冲区
/// 格式: [Header(2B)] [AdvA(6B)] [AD Structures...]
/// 注意: RADIO 外设会自动处理前导码和 CRC，我们只需提供 PDU 部分
#[repr(C, align(4))]
struct AdvPacket {
    /// PDU 数据（最大 39 字节: 2B header + 37B payload）
    data: [u8; 39],
    /// 实际使用的长度
    len: usize,
}

impl AdvPacket {
    /// 构造一个 ADV_NONCONN_IND 广播包
    ///
    /// PDU 格式:
    /// - Header[0]: PDU Type (4bit) | RFU (1bit) | ChSel (1bit) | TxAdd (1bit) | RxAdd (1bit)
    /// - Header[1]: Length (payload 长度)
    /// - Payload: AdvA (6B) + AdvData (0-31B)
    fn new_nonconn_ind(adv_addr: &[u8; 6], adv_data: &[u8]) -> Self {
        let mut pkt = AdvPacket {
            data: [0u8; 39],
            len: 0,
        };

        // PDU Header
        // ADV_NONCONN_IND = 0b0010, TxAdd = 1 (随机地址)
        pkt.data[0] = 0x02 | (1 << 6); // PDU Type = ADV_NONCONN_IND, TxAdd = Random
        let payload_len = 6 + adv_data.len();
        pkt.data[1] = payload_len as u8;

        // AdvA: 广播地址（6 字节，小端序）
        pkt.data[2..8].copy_from_slice(adv_addr);

        // AdvData: 广播数据
        pkt.data[8..8 + adv_data.len()].copy_from_slice(adv_data);

        pkt.len = 2 + payload_len;
        pkt
    }
}

/// 构造广播数据（AD Structures）
/// 包含: Flags + Complete Local Name
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

    // AD Structure 3: TX Power Level (可选)
    // Length = 2, Type = 0x0A, Value = 0 dBm
    buf[pos] = 0x02; // Length
    buf[pos + 1] = 0x0A; // AD Type: TX Power Level
    buf[pos + 2] = 0x00; // 0 dBm
    pos += 3;

    pos
}

/// 配置 RADIO 外设为 BLE 1Mbit 模式
fn radio_init(radio: &nrf52833_pac::RADIO) {
    // 禁用 RADIO
    radio.tasks_disable.write(|w| unsafe { w.bits(1) });
    while radio.events_disabled.read().bits() == 0 {}
    radio.events_disabled.write(|w| unsafe { w.bits(0) });

    // 设置模式为 BLE 1Mbit
    radio.mode.write(|w| w.mode().ble_1mbit());

    // 配置 PCNF0 (Packet Configuration 0)
    // BLE: LENGTH 字段 8 bit, 位于第 0 位, S0 = 1 byte, S1 = 0
    radio.pcnf0.write(|w| unsafe {
        w.lflen().bits(8)    // LENGTH 字段长度: 8 bits
         .s0len().bit(true)  // S0 字段长度: 1 byte (PDU header 第一个字节)
         .s1len().bits(0)    // S1 字段长度: 0
         .plen()._8bit()     // 前导码长度: 8 bit (BLE 1Mbit)
    });

    // 配置 PCNF1 (Packet Configuration 1)
    // MAXLEN = 37 (BLE 广播 payload 最大长度)
    // STATLEN = 0, BALEN = 3 (基地址长度 = 3+1 = 4 字节)
    // ENDIAN = Little, WHITEEN = 1 (启用数据白化)
    radio.pcnf1.write(|w| unsafe {
        w.maxlen().bits(37)     // 最大 payload 长度
         .statlen().bits(0)     // 静态长度附加: 0
         .balen().bits(3)       // 基地址长度: 3 (实际 = 3+1 = 4 字节)
         .endian().little()     // 小端序
         .whiteen().set_bit()   // 启用数据白化
    });

    // 配置访问地址 (Access Address)
    // BLE 广播固定使用 0x8E89BED6
    // BASE0 = 地址的低 4 字节（字节反转）, PREFIX0 = 地址的最高字节
    // 访问地址 0x8E89BED6 -> 空中传输顺序（LSB first）: D6 BE 89 8E
    // RADIO 配置: PREFIX0[0] = 0x8E, BASE0 = 0x89BED600 (左移 8 位)
    radio.base0.write(|w| unsafe { w.bits(0x89BED600) });
    radio.prefix0.write(|w| unsafe { w.bits(0x0000008E) });

    // 使用逻辑地址 0 发送
    radio.txaddress.write(|w| unsafe { w.txaddress().bits(0) });

    // 配置 CRC
    // BLE 使用 24-bit CRC, 多项式 = 0x00065B, 初始值 = 0x555555
    radio.crccnf.write(|w| w.len().three().skipaddr().skip());
    radio.crcpoly.write(|w| unsafe { w.crcpoly().bits(0x0006_5B) });
    radio.crcinit.write(|w| unsafe { w.crcinit().bits(0x55_5555) });

    // 配置发射功率: 0 dBm
    radio.txpower.write(|w| w.txpower()._0d_bm());

    // 配置快捷方式: READY -> START, END -> DISABLE
    radio.shorts.write(|w| {
        w.ready_start().enabled()
         .end_disable().enabled()
    });
}

/// 在指定的广播通道上发送一个广播包
fn radio_send(radio: &nrf52833_pac::RADIO, packet: &AdvPacket, channel_idx: usize) {
    // 设置频率
    radio.frequency.write(|w| unsafe {
        w.frequency().bits(BLE_ADV_CHANNEL_FREQ[channel_idx] as u8)
    });

    // 设置数据白化初始值（基于通道号）
    // BLE 白化 LFSR 初始值 = channel_index | 0x40
    let channel_num = match channel_idx {
        0 => 37u8,
        1 => 38,
        2 => 39,
        _ => 37,
    };
    radio.datawhiteiv.write(|w| unsafe {
        w.datawhiteiv().bits(channel_num | 0x40)
    });

    // 设置数据包指针（指向 PDU 数据）
    radio.packetptr.write(|w| unsafe {
        w.bits(packet.data.as_ptr() as u32)
    });

    // 清除事件
    radio.events_disabled.write(|w| unsafe { w.bits(0) });

    // 启动发送 (TXEN -> READY -> START -> END -> DISABLE 由 shorts 自动完成)
    radio.tasks_txen.write(|w| unsafe { w.bits(1) });

    // 等待发送完成（DISABLED 事件）
    while radio.events_disabled.read().bits() == 0 {}
    radio.events_disabled.write(|w| unsafe { w.bits(0) });
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("=== 裸机 BLE 广播信标 (Bare-metal BLE Beacon) ===");
    rprintln!("设备名: {}", core::str::from_utf8(DEVICE_NAME).unwrap_or("?"));
    rprintln!("广播间隔: {} ms", ADV_INTERVAL_MS);
    rprintln!("注意: 这是不可连接的广播 (ADV_NONCONN_IND)");

    let periph = Peripherals::take().unwrap();

    // 启用高频时钟 (HFCLK)，RADIO 需要外部高频晶振
    periph.CLOCK.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
    while periph.CLOCK.events_hfclkstarted.read().bits() == 0 {}
    periph.CLOCK.events_hfclkstarted.write(|w| unsafe { w.bits(0) });
    rprintln!("高频时钟已启动 (HFXO)");

    // 初始化 RADIO
    radio_init(&periph.RADIO);
    rprintln!("RADIO 已初始化 (BLE 1Mbit 模式)");

    // 生成一个随机静态地址 (Random Static Address)
    // 格式: 最高 2 位 = 11 (标识为随机静态地址)
    // 这里使用固定地址用于演示，实际应用中应使用 FICR 中的设备地址或 RNG
    let adv_addr: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xDE]; // 最高字节 bit7:6 = 11

    // 构造广播数据
    let mut adv_data_buf = [0u8; 31];
    let adv_data_len = build_adv_data(&mut adv_data_buf);

    // 构造广播包
    let packet = AdvPacket::new_nonconn_ind(&adv_addr, &adv_data_buf[..adv_data_len]);
    rprintln!("广播包已构造, PDU 长度: {} 字节", packet.len);

    // 初始化定时器用于广播间隔
    let mut timer = Timer::new(periph.TIMER0);
    let mut adv_count: u32 = 0;

    rprintln!("开始广播...");
    rprintln!("请使用 nRF Connect 等 BLE 扫描工具查看设备 \"{}\"",
        core::str::from_utf8(DEVICE_NAME).unwrap_or("?"));

    loop {
        // 在 3 个广播通道上依次发送
        for ch in 0..3 {
            radio_send(&periph.RADIO, &packet, ch);
        }

        adv_count += 1;
        if adv_count % 10 == 0 {
            rprintln!("已广播 {} 次", adv_count);
        }

        // 等待广播间隔
        timer.delay_ms(ADV_INTERVAL_MS);
    }
}
