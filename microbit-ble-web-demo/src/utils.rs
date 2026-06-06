//! 工具模块：协议常量、CRC 计算、帧编解码
//! 与 micro:bit 固件 src/ble/protocol.rs 保持完全一致

// =========================================================
// Nordic UART Service (NUS) UUID
// =========================================================
pub const NUS_SERVICE: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
pub const NUS_RX_CHAR: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e"; // 写入方向 (Browser -> Board)
pub const NUS_TX_CHAR: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e"; // 通知方向 (Board -> Browser)

// =========================================================
// 命令字节
// =========================================================
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
  // 主机 -> 设备
  Ping = 0x01,
  LedSet = 0x02,
  LedClear = 0x03,
  LedChar = 0x04,
  TempGet = 0x05,
  BtnSubscribe = 0x06,
  Echo = 0x07,

  // 设备 -> 主机
  Pong = 0x81,
  LedAck = 0x82,
  TempResp = 0x85,
  EchoResp = 0x87,
  BtnEvent = 0x90,
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
      0x81 => Ok(Command::Pong),
      0x82 => Ok(Command::LedAck),
      0x85 => Ok(Command::TempResp),
      0x87 => Ok(Command::EchoResp),
      0x90 => Ok(Command::BtnEvent),
      0xFF => Ok(Command::Error),
      _ => Err("Unknown command"),
    }
  }
}

// =========================================================
// 帧格式：[SOF(0xAA), CMD, LEN, ...payload, CRC]
// =========================================================
pub const SOF: u8 = 0xAA;
pub const MAX_PAYLOAD: usize = 60;

/// CRC-8 计算 (poly 0x07, init 0x00)
/// 与固件实现完全一致
pub fn crc8(bytes: &[u8]) -> u8 {
  let mut crc: u8 = 0x00;
  for &b in bytes {
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

/// 构建数据帧
/// 返回 `(frame, crc_input)` 元组，其中 `crc_input = [CMD, LEN, ...payload]`
pub fn build_frame(cmd: u8, payload: &[u8]) -> Result<Vec<u8>, String> {
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

  // CRC 覆盖 CMD + LEN + PAYLOAD
  let mut crc_input = Vec::with_capacity(2 + payload.len());
  crc_input.push(cmd);
  crc_input.push(len);
  crc_input.extend_from_slice(payload);
  frame.push(crc8(&crc_input));

  Ok(frame)
}

/// 解析接收到的数据帧
/// 返回 `Some((cmd, payload))` 或 `None`（解析失败）
pub fn parse_frame(bytes: &[u8]) -> Option<(u8, Vec<u8>)> {
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

  // 重新计算 CRC
  let mut crc_input = Vec::with_capacity(2 + len);
  crc_input.push(cmd);
  crc_input.push(len as u8);
  crc_input.extend_from_slice(&payload);

  if crc8(&crc_input) != recv_crc {
    return None;
  }

  Some((cmd, payload))
}

/// 将字节数组转换为十六进制字符串
pub fn to_hex(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect::<Vec<_>>()
    .join(" ")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crc8() {
    // 测试用例：CMD=Ping(0x01), LEN=0, payload=[]
    let input = [0x01, 0x00];
    let crc = crc8(&input);
    // 预期 CRC 值（需与固件一致）
    assert_eq!(crc, 0x96);
  }

  #[test]
  fn test_build_and_parse_roundtrip() {
    let cmd = Command::Ping as u8;
    let payload = [];
    let frame = build_frame(cmd, &payload).unwrap();
    let parsed = parse_frame(&frame).unwrap();
    assert_eq!(parsed.0, cmd);
    assert_eq!(parsed.1, payload);
  }

  #[test]
  fn test_build_frame_too_long() {
    let payload = [0u8; 61];
    assert!(build_frame(0x01, &payload).is_err());
  }
}
