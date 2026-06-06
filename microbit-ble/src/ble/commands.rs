//! 指令分发：解析 NUS RX 收到的二进制帧，执行对应动作并构造响应。
//!
//! 由 BLE 主循环调用 [`handle_rx`]，输入是 RX 写入字节，输出是要通过 TX 回写的字节。

use defmt::{info, warn};
use nrf_softdevice::Softdevice;
use nrf_softdevice::temperature_celsius;

use super::buttons;
use super::led_matrix;
use super::protocol::{
  self, CMD_BTN_EVENT, CMD_BTN_SUBSCRIBE, CMD_ECHO, CMD_ECHO_RESP, CMD_ERROR, CMD_LED_ACK,
  CMD_LED_CHAR, CMD_LED_CLEAR, CMD_LED_SET, CMD_PING, CMD_PONG, CMD_TEMP_GET, CMD_TEMP_RESP,
  ERR_BAD_CRC, ERR_BAD_FRAME, ERR_BAD_PAYLOAD, ERR_UNKNOWN_CMD, MAX_FRAME_LEN,
};

/// 处理结果：编码后的响应帧（最多一帧）
pub struct Response {
  buf: [u8; MAX_FRAME_LEN],
  len: usize,
}

impl Response {
  pub fn new() -> Self {
    Self {
      buf: [0u8; MAX_FRAME_LEN],
      len: 0,
    }
  }

  pub fn build(cmd: u8, payload: &[u8]) -> Option<Self> {
    let mut r = Self::new();
    let n = protocol::build_frame(cmd, payload, &mut r.buf)?;
    r.len = n;
    Some(r)
  }

  pub fn as_slice(&self) -> &[u8] {
    &self.buf[..self.len]
  }
}

/// 把按钮事件编码成一帧
pub fn encode_button_event(evt: buttons::ButtonEvent) -> Option<Response> {
  let payload = [evt.id as u8, evt.pressed as u8];
  Response::build(CMD_BTN_EVENT, &payload)
}

/// 处理一次 NUS RX 写入的数据，返回需要回写的响应（如果有）
pub fn handle_rx(sd: &Softdevice, data: &[u8]) -> Option<Response> {
  // 解析帧
  let frame = match protocol::parse_frame(data) {
    Ok(f) => f,
    Err(e) => {
      warn!("帧解析失败: {:?}", defmt::Debug2Format(&e));
      let code = match e {
        protocol::ParseError::BadCrc => ERR_BAD_CRC,
        _ => ERR_BAD_FRAME,
      };
      return Response::build(CMD_ERROR, &[code]);
    }
  };

  match frame.cmd {
    CMD_PING => {
      info!("收到 PING");
      Response::build(CMD_PONG, &[])
    }

    CMD_LED_SET => {
      if frame.payload.len() != 25 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::set_frame_from_bytes(frame.payload);
      info!("LED 矩阵已更新");
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_CLEAR => {
      led_matrix::clear_frame();
      info!("LED 矩阵已清空");
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_CHAR => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::show_char(frame.payload[0]);
      info!("LED 显示字符: {}", frame.payload[0] as char);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_TEMP_GET => {
      let payload: [u8; 4] = match temperature_celsius(sd) {
        Ok(t) => {
          // I30F2: 整数部分 30 位 + 2 位小数（每 LSB = 0.25°C）
          // 转换为 0.01°C 为单位的 i32（避免浮点）
          // value_in_quarters = t.to_bits()  即 4 倍温度
          // hundredths = quarters * 25
          let quarters: i32 = t.to_bits();
          let hundredths: i32 = quarters * 25;
          info!("温度: {} (0.01°C)", hundredths);
          hundredths.to_le_bytes()
        }
        Err(_) => {
          warn!("温度读取失败");
          return Response::build(CMD_ERROR, &[0xFE]);
        }
      };
      Response::build(CMD_TEMP_RESP, &payload)
    }

    CMD_BTN_SUBSCRIBE => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      buttons::set_subscribed(frame.payload[0] != 0);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_ECHO => {
      info!("ECHO {} bytes", frame.payload.len());
      Response::build(CMD_ECHO_RESP, frame.payload)
    }

    other => {
      warn!("未知指令: 0x{:02X}", other);
      Response::build(CMD_ERROR, &[ERR_UNKNOWN_CMD])
    }
  }
}
