//! 二进制协议编解码
//!
//! 帧格式：
//! ```text
//! ┌──────┬──────┬──────────┬──────────────┬──────┐
//! │ SOF  │ CMD  │  LEN     │  PAYLOAD     │ CRC8 │
//! │ 0xAA │ 1B   │  1B      │  0..N bytes  │ 1B   │
//! └──────┴──────┴──────────┴──────────────┴──────┘
//! ```
//! - SOF: 帧起始 0xAA
//! - CMD: 指令码（见 [`Cmd`]）
//! - LEN: payload 长度（0~MAX_PAYLOAD）
//! - CRC8: 多项式 0x07，覆盖 CMD + LEN + PAYLOAD
//!
//! 这个模块在 no_std 环境中工作，不分配堆内存。

#![allow(dead_code)]

/// 帧起始字节
pub const SOF: u8 = 0xAA;

/// 单帧最大 payload 长度（受 ATT MTU 限制，留余量）
pub const MAX_PAYLOAD: usize = 60;

/// 单帧最大总长度 = SOF(1) + CMD(1) + LEN(1) + PAYLOAD + CRC(1)
pub const MAX_FRAME_LEN: usize = 4 + MAX_PAYLOAD;

// ===== 指令码常量 =====

/// 心跳请求（无 payload）
pub const CMD_PING: u8 = 0x01;
/// LED 矩阵设置（payload: 25 字节亮度，0=灭 1=亮）
pub const CMD_LED_SET: u8 = 0x02;
/// LED 清空（无 payload）
pub const CMD_LED_CLEAR: u8 = 0x03;
/// 显示一个字符（payload: 1 字节 ASCII，简化版滚动文字）
pub const CMD_LED_CHAR: u8 = 0x04;
/// 读取芯片温度（无 payload，回 [`CMD_TEMP_RESP`]）
pub const CMD_TEMP_GET: u8 = 0x05;
/// 订阅按钮事件（payload: 1 字节，0=取消 1=订阅）
pub const CMD_BTN_SUBSCRIBE: u8 = 0x06;
/// 回显（payload: 任意字节，回 [`CMD_ECHO_RESP`]）
pub const CMD_ECHO: u8 = 0x07;

/// 心跳应答（无 payload）
pub const CMD_PONG: u8 = 0x81;
/// LED 操作应答（payload: 1 字节状态，0=OK 其它=错误码）
pub const CMD_LED_ACK: u8 = 0x82;
/// 温度应答（payload: 4 字节 i32 LE，单位 0.01℃）
pub const CMD_TEMP_RESP: u8 = 0x85;
/// 回显应答（payload: 与请求相同）
pub const CMD_ECHO_RESP: u8 = 0x87;
/// 按钮事件通知（payload: 2 字节，[btn_id, state]，btn_id: A=1 B=2，state: 0=释放 1=按下）
pub const CMD_BTN_EVENT: u8 = 0x90;
/// 错误响应（payload: 1 字节错误码）
pub const CMD_ERROR: u8 = 0xFF;

// ===== 错误码 =====

pub const ERR_BAD_FRAME: u8 = 0x01;
pub const ERR_BAD_CRC: u8 = 0x02;
pub const ERR_UNKNOWN_CMD: u8 = 0x03;
pub const ERR_BAD_PAYLOAD: u8 = 0x04;

/// 解析后的指令帧（零拷贝引用 payload）
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
  pub cmd: u8,
  pub payload: &'a [u8],
}

/// 帧解析错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
  /// 帧太短，无法构成完整帧
  TooShort,
  /// 帧起始字节不正确
  BadSof,
  /// 长度字段超过 MAX_PAYLOAD
  PayloadTooLong,
  /// CRC 校验失败
  BadCrc,
}

/// 计算 CRC-8 (多项式 0x07，初始值 0x00)
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

/// 解析一帧。成功返回 [`Frame`]，失败返回 [`ParseError`]
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

/// 计算 cmd+len+payload 的 CRC8
fn crc8_for_frame(cmd: u8, payload: &[u8]) -> u8 {
  // 复用 crc8: 先 cmd + len，再追加 payload
  let header = [cmd, payload.len() as u8];
  let mut crc: u8 = 0x00;
  // 内联展开，避免分配
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

/// 把一帧编码到 `out` 缓冲区，返回写入字节数
///
/// 如果 payload 太长或 out 容量不足，返回 None。
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
