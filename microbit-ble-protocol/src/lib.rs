//! Shared BLE binary protocol for micro:bit firmware and web frontend
//!
//! Frame format:
//! ```text
//! ┌──────┬──────┬──────────┬──────────────┬──────┐
//! │ SOF  │ CMD  │  LEN     │  PAYLOAD     │ CRC8 │
//! │ 0xAA │ 1B   │  1B      │  0..N bytes  │ 1B   │
//! └──────┴──────┴──────────┴──────────────┴──────┘
//! ```
//! - SOF: frame start 0xAA
//! - CMD: command code (see `CMD_*` constants)
//! - LEN: payload length (0~MAX_PAYLOAD)
//! - CRC8: polynomial 0x07, covers CMD + LEN + PAYLOAD
//!
//! This crate works in `no_std` environment without heap allocation by default.
//! Enable the `alloc` feature for `Vec<u8>`-based APIs, `Command` enum, and `to_hex()`.

#![no_std]
#![allow(dead_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

// ===== Nordic UART Service (NUS) UUID =====

/// NUS Service UUID
pub const NUS_SERVICE: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
/// NUS RX Characteristic UUID (Write: Browser -> Board)
pub const NUS_RX_CHAR: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
/// NUS TX Characteristic UUID (Notify: Board -> Browser)
pub const NUS_TX_CHAR: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

// ===== Frame constants =====

/// Frame start byte
pub const SOF: u8 = 0xAA;

/// Maximum payload length per frame (limited by ATT MTU, with margin)
pub const MAX_PAYLOAD: usize = 60;

/// Maximum total frame length = SOF(1) + CMD(1) + LEN(1) + PAYLOAD + CRC(1)
pub const MAX_FRAME_LEN: usize = 4 + MAX_PAYLOAD;

// ===== Command code constants =====

/// Ping request (no payload)
pub const CMD_PING: u8 = 0x01;
/// LED matrix set (payload: 25 bytes brightness, 0=off 1=on)
pub const CMD_LED_SET: u8 = 0x02;
/// LED clear (no payload)
pub const CMD_LED_CLEAR: u8 = 0x03;
/// Display a character (payload: 1 byte ASCII, simplified scrolling text)
pub const CMD_LED_CHAR: u8 = 0x04;
/// Read chip temperature (no payload, responds with `CMD_TEMP_RESP`)
pub const CMD_TEMP_GET: u8 = 0x05;
/// Subscribe to button events (payload: 1 byte, 0=unsubscribe 1=subscribe)
pub const CMD_BTN_SUBSCRIBE: u8 = 0x06;
/// Echo (payload: arbitrary bytes, responds with `CMD_ECHO_RESP`)
pub const CMD_ECHO: u8 = 0x07;
/// Play a tone (payload: 2 bytes frequency LE + 2 bytes duration_ms LE)
pub const CMD_SOUND_PLAY: u8 = 0x08;
/// Stop playing tone (no payload)
pub const CMD_SOUND_STOP: u8 = 0x09;
/// Subscribe/unsubscribe accelerometer data (payload: 1 byte, 0=unsubscribe 1=subscribe)
pub const CMD_ACCEL_SUBSCRIBE: u8 = 0x0A;
/// Subscribe/unsubscribe magnetometer data (payload: 1 byte, 0=unsubscribe 1=subscribe)
pub const CMD_MAGNET_SUBSCRIBE: u8 = 0x0B;
/// Set LED brightness (payload: 25 bytes, each 0..255 grayscale)
pub const CMD_LED_BRIGHTNESS: u8 = 0x0C;
/// Scroll text on LED matrix (payload: ASCII string bytes)
pub const CMD_LED_SCROLL: u8 = 0x0D;
/// Subscribe/unsubscribe touch sensor events (payload: 1 byte, 0=unsubscribe 1=subscribe)
/// Touch events include: Logo touch, Pin0/1/2 capacitive touch
pub const CMD_TOUCH_SUBSCRIBE: u8 = 0x0E;

/// Ping response (no payload)
pub const CMD_PONG: u8 = 0x81;
/// LED operation acknowledgement (payload: 1 byte status, 0=OK other=error code)
pub const CMD_LED_ACK: u8 = 0x82;
/// Temperature response (payload: 4 bytes i32 LE, unit 0.01°C)
pub const CMD_TEMP_RESP: u8 = 0x85;
/// Echo response (payload: same as request)
pub const CMD_ECHO_RESP: u8 = 0x87;
/// Button event notification (payload: 2 bytes, [btn_id, state], btn_id: A=1 B=2, state: 0=released 1=pressed)
pub const CMD_BTN_EVENT: u8 = 0x90;
/// Accelerometer data notification (payload: 6 bytes, 3× i16 LE [x, y, z], unit: 0.01g)
pub const CMD_ACCEL_DATA: u8 = 0x8A;
/// Magnetometer data notification (payload: 6 bytes, 3× i16 LE [x, y, z], unit: 0.1μT)
pub const CMD_MAGNET_DATA: u8 = 0x8B;
/// Sound operation acknowledgement (payload: 1 byte status, 0=OK other=error code)
pub const CMD_SOUND_ACK: u8 = 0x88;
/// LED brightness acknowledgement (payload: 1 byte status, 0=OK other=error code)
pub const CMD_LED_BRIGHTNESS_ACK: u8 = 0x8C;
/// LED scroll acknowledgement (payload: 1 byte status, 0=OK other=error code)
pub const CMD_LED_SCROLL_ACK: u8 = 0x8D;
/// Touch sensor event notification (payload: 2 bytes, [touch_id, state], touch_id: Logo=0 Pin0=1 Pin1=2 Pin2=3, state: 0=released 1=pressed)
pub const CMD_TOUCH_EVENT: u8 = 0x91;
/// Error response (payload: 1 byte error code)
pub const CMD_ERROR: u8 = 0xFF;

// ===== Error codes =====

pub const ERR_BAD_FRAME: u8 = 0x01;
pub const ERR_BAD_CRC: u8 = 0x02;
pub const ERR_UNKNOWN_CMD: u8 = 0x03;
pub const ERR_BAD_PAYLOAD: u8 = 0x04;

// ===== Core types (no_std, no alloc) =====

/// Parsed command frame (zero-copy reference to payload)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
  pub cmd: u8,
  pub payload: &'a [u8],
}

/// Frame parse error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
  /// Frame too short to form a complete frame
  TooShort,
  /// Incorrect frame start byte
  BadSof,
  /// Length field exceeds MAX_PAYLOAD
  PayloadTooLong,
  /// CRC check failed
  BadCrc,
}

// ===== CRC-8 =====

/// Calculate CRC-8 (polynomial 0x07, initial value 0x00)
pub fn crc8(data: &[u8]) -> u8 {
  let mut crc: u8 = 0x00;
  for &b in data {
    crc ^= b;
    for _ in 0..8 {
      if crc & 0x80 != 0 {
        crc = (crc << 1) ^ 0x07;
      } else {
        crc <<= 1;
      }
    }
  }
  crc
}

/// Calculate CRC8 over cmd+len+payload
fn crc8_for_frame(cmd: u8, payload: &[u8]) -> u8 {
  let header = [cmd, payload.len() as u8];
  let mut crc: u8 = 0x00;
  for &b in header.iter().chain(payload.iter()) {
    crc ^= b;
    for _ in 0..8 {
      if crc & 0x80 != 0 {
        crc = (crc << 1) ^ 0x07;
      } else {
        crc <<= 1;
      }
    }
  }
  crc
}

// ===== no_std parse_frame (zero-copy) =====

/// Parse one frame. Returns `Frame` on success, `ParseError` on failure.
///
/// This is a zero-copy version that borrows from the input buffer.
pub fn parse_frame(buf: &[u8]) -> Result<Frame<'_>, ParseError> {
  if buf.len() < 4 {
    return Err(ParseError::TooShort);
  }
  if buf[0] != SOF {
    return Err(ParseError::BadSof);
  }
  let cmd = buf[1];
  let len = buf[2] as usize;
  if len > MAX_PAYLOAD {
    return Err(ParseError::PayloadTooLong);
  }
  let total = 4 + len;
  if buf.len() < total {
    return Err(ParseError::TooShort);
  }
  let payload = &buf[3..3 + len];
  let received_crc = buf[3 + len];
  let calculated_crc = crc8_for_frame(cmd, payload);
  if received_crc != calculated_crc {
    return Err(ParseError::BadCrc);
  }
  Ok(Frame { cmd, payload })
}

// ===== no_std build_frame (zero-alloc, writes into &mut [u8]) =====

/// Encode one frame into `out` buffer, return number of bytes written.
///
/// Returns `None` if payload is too long or out capacity is insufficient.
pub fn build_frame(cmd: u8, payload: &[u8], out: &mut [u8]) -> Option<usize> {
  if payload.len() > MAX_PAYLOAD {
    return None;
  }
  let total = 4 + payload.len();
  if out.len() < total {
    return None;
  }
  out[0] = SOF;
  out[1] = cmd;
  out[2] = payload.len() as u8;
  out[3..3 + payload.len()].copy_from_slice(payload);
  out[3 + payload.len()] = crc8_for_frame(cmd, payload);
  Some(total)
}

// ===== alloc feature: Vec-based APIs, Command enum, to_hex =====

#[cfg(feature = "alloc")]
mod alloc_impls {
  use alloc::format;
  use alloc::string::String;
  use alloc::vec::Vec;

  use super::*;

  /// Build a data frame, returning a `Vec<u8>`.
  ///
  /// Returns `Err(String)` if payload is too long.
  pub fn build_frame_vec(cmd: u8, payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.len() > MAX_PAYLOAD {
      return Err(format!(
        "Payload too long: {} > {MAX_PAYLOAD}",
        payload.len()
      ));
    }

    let len = payload.len() as u8;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.push(SOF);
    frame.push(cmd);
    frame.push(len);
    frame.extend_from_slice(payload);

    // CRC covers CMD + LEN + PAYLOAD
    let mut crc_input = Vec::with_capacity(2 + payload.len());
    crc_input.push(cmd);
    crc_input.push(len);
    crc_input.extend_from_slice(payload);
    frame.push(crc8(&crc_input));

    Ok(frame)
  }

  /// Parse a received data frame, returning `Some((cmd, payload))` or `None`.
  pub fn parse_frame_vec(bytes: &[u8]) -> Option<(u8, Vec<u8>)> {
    if bytes.len() < 4 {
      return None;
    }
    if bytes[0] != SOF {
      return None;
    }

    let cmd = bytes[1];
    let len = bytes[2] as usize;

    if bytes.len() < 4 + len {
      return None;
    }

    let payload = bytes[3..3 + len].to_vec();
    let recv_crc = bytes[3 + len];

    // Recalculate CRC
    let mut crc_input = Vec::with_capacity(2 + len);
    crc_input.push(cmd);
    crc_input.push(len as u8);
    crc_input.extend_from_slice(&payload);

    if crc8(&crc_input) != recv_crc {
      return None;
    }

    Some((cmd, payload))
  }

  /// Command enum with `TryFrom<u8>` conversion.
  #[repr(u8)]
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Command {
    // Host -> Device
    Ping = 0x01,
    LedSet = 0x02,
    LedClear = 0x03,
    LedChar = 0x04,
    TempGet = 0x05,
    BtnSubscribe = 0x06,
    Echo = 0x07,
    SoundPlay = 0x08,
    SoundStop = 0x09,
    AccelSubscribe = 0x0A,
    MagnetSubscribe = 0x0B,
    LedBrightness = 0x0C,
    LedScroll = 0x0D,
    TouchSubscribe = 0x0E,

    // Device -> Host
    Pong = 0x81,
    LedAck = 0x82,
    TempResp = 0x85,
    SoundAck = 0x88,
    EchoResp = 0x87,
    AccelData = 0x8A,
    MagnetData = 0x8B,
    LedBrightnessAck = 0x8C,
    LedScrollAck = 0x8D,
    BtnEvent = 0x90,
    TouchEvent = 0x91,
    Error = 0xFF,
  }

  impl TryFrom<u8> for Command {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, &'static str> {
      match value {
        0x01 => Ok(Command::Ping),
        0x02 => Ok(Command::LedSet),
        0x03 => Ok(Command::LedClear),
        0x04 => Ok(Command::LedChar),
        0x05 => Ok(Command::TempGet),
        0x06 => Ok(Command::BtnSubscribe),
        0x07 => Ok(Command::Echo),
        0x08 => Ok(Command::SoundPlay),
        0x09 => Ok(Command::SoundStop),
        0x0A => Ok(Command::AccelSubscribe),
        0x0B => Ok(Command::MagnetSubscribe),
        0x0C => Ok(Command::LedBrightness),
        0x0D => Ok(Command::LedScroll),
        0x0E => Ok(Command::TouchSubscribe),
        0x81 => Ok(Command::Pong),
        0x82 => Ok(Command::LedAck),
        0x85 => Ok(Command::TempResp),
        0x88 => Ok(Command::SoundAck),
        0x87 => Ok(Command::EchoResp),
        0x8A => Ok(Command::AccelData),
        0x8B => Ok(Command::MagnetData),
        0x8C => Ok(Command::LedBrightnessAck),
        0x8D => Ok(Command::LedScrollAck),
        0x90 => Ok(Command::BtnEvent),
        0x91 => Ok(Command::TouchEvent),
        0xFF => Ok(Command::Error),
        _ => Err("Unknown command"),
      }
    }
  }

  /// Convert byte array to hex string
  pub fn to_hex(bytes: &[u8]) -> String {
    bytes
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<Vec<_>>()
      .join(" ")
  }
}

// Re-export alloc items at crate root when feature is enabled
#[cfg(feature = "alloc")]
pub use alloc_impls::{Command, build_frame_vec, parse_frame_vec, to_hex};

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crc8_known() {
    // CRC-8/CCITT (poly 0x07) of "123456789" = 0xF4
    assert_eq!(crc8(b"123456789"), 0xF4);
  }

  #[test]
  fn test_roundtrip() {
    let payload = b"hello";
    let mut buf = [0u8; 64];
    let n = build_frame(CMD_ECHO, payload, &mut buf).unwrap();
    let frame = parse_frame(&buf[..n]).unwrap();
    assert_eq!(frame.cmd, CMD_ECHO);
    assert_eq!(frame.payload, payload);
  }

  #[test]
  fn test_bad_crc() {
    let payload = b"hi";
    let mut buf = [0u8; 16];
    let n = build_frame(CMD_PING, payload, &mut buf).unwrap();
    buf[n - 1] ^= 0xFF;
    assert_eq!(parse_frame(&buf[..n]), Err(ParseError::BadCrc));
  }

  #[test]
  fn test_build_frame_payload_too_long() {
    let payload = [0u8; MAX_PAYLOAD + 1];
    let mut buf = [0u8; MAX_FRAME_LEN + 1];
    assert!(build_frame(0x01, &payload, &mut buf).is_none());
  }

  #[test]
  fn test_parse_frame_too_short() {
    assert_eq!(parse_frame(&[SOF, 0x01]), Err(ParseError::TooShort));
  }

  #[test]
  fn test_parse_frame_bad_sof() {
    assert_eq!(
      parse_frame(&[0x00, 0x01, 0x00, 0x00]),
      Err(ParseError::BadSof)
    );
  }
}
