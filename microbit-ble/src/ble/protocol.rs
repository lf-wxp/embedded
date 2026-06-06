//! Binary protocol encode/decode
//!
//! Frame format:
//! ```text
//! ┌──────┬──────┬──────────┬──────────────┬──────┐
//! │ SOF  │ CMD  │  LEN     │  PAYLOAD     │ CRC8 │
//! │ 0xAA │ 1B   │  1B      │  0..N bytes  │ 1B   │
//! └──────┴──────┴──────────┴──────────────┴──────┘
//! ```
//! - SOF: frame start 0xAA
//! - CMD: command code (see [`Cmd`])
//! - LEN: payload length (0~MAX_PAYLOAD)
//! - CRC8: polynomial 0x07, covers CMD + LEN + PAYLOAD
//!
//! This module works in no_std environment without heap allocation.

#![allow(dead_code)]

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
/// Read chip temperature (no payload, responds with [`CMD_TEMP_RESP`])
pub const CMD_TEMP_GET: u8 = 0x05;
/// Subscribe to button events (payload: 1 byte, 0=unsubscribe 1=subscribe)
pub const CMD_BTN_SUBSCRIBE: u8 = 0x06;
/// Echo (payload: arbitrary bytes, responds with [`CMD_ECHO_RESP`])
pub const CMD_ECHO: u8 = 0x07;

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
/// Error response (payload: 1 byte error code)
pub const CMD_ERROR: u8 = 0xFF;

// ===== Error codes =====

pub const ERR_BAD_FRAME: u8 = 0x01;
pub const ERR_BAD_CRC: u8 = 0x02;
pub const ERR_UNKNOWN_CMD: u8 = 0x03;
pub const ERR_BAD_PAYLOAD: u8 = 0x04;

/// Parsed command frame (zero-copy reference to payload)
#[derive(Debug, Clone, Copy)]
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

/// Parse one frame. Returns [`Frame`] on success, [`ParseError`] on failure
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

/// Calculate CRC8 over cmd+len+payload
fn crc8_for_frame(cmd: u8, payload: &[u8]) -> u8 {
  // Reuse crc8: cmd + len first, then append payload
  let header = [cmd, payload.len() as u8];
  let mut crc: u8 = 0x00;
  // Inline expansion to avoid allocation
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

/// Encode one frame into `out` buffer, return number of bytes written
///
/// Returns None if payload is too long or out capacity is insufficient.
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
}
