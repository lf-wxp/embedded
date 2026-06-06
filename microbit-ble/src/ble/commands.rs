//! Command dispatch: parse binary frames received on NUS RX, execute corresponding actions and construct responses.
//!
//! Called by the BLE main loop via [`handle_rx`], input is the RX written bytes, output is the response bytes to write back via TX.

use defmt::{info, warn};
use nrf_softdevice::Softdevice;
use nrf_softdevice::temperature_celsius;

use super::buttons;
use super::led_matrix;
use microbit_ble_protocol::{
  CMD_BTN_EVENT, CMD_BTN_SUBSCRIBE, CMD_ECHO, CMD_ECHO_RESP, CMD_ERROR, CMD_LED_ACK, CMD_LED_CHAR,
  CMD_LED_CLEAR, CMD_LED_SET, CMD_PING, CMD_PONG, CMD_TEMP_GET, CMD_TEMP_RESP, ERR_BAD_CRC,
  ERR_BAD_FRAME, ERR_BAD_PAYLOAD, ERR_UNKNOWN_CMD, MAX_FRAME_LEN,
};

/// Processing result: encoded response frame (at most one frame)
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
    let n = microbit_ble_protocol::build_frame(cmd, payload, &mut r.buf)?;
    r.len = n;
    Some(r)
  }

  pub fn as_slice(&self) -> &[u8] {
    &self.buf[..self.len]
  }
}

/// Encode button event into a frame
pub fn encode_button_event(evt: buttons::ButtonEvent) -> Option<Response> {
  let payload = [evt.id as u8, evt.pressed as u8];
  Response::build(CMD_BTN_EVENT, &payload)
}

/// Process one NUS RX write, return the response to write back (if any)
pub fn handle_rx(sd: &Softdevice, data: &[u8]) -> Option<Response> {
  // Parse frame
  let frame = match microbit_ble_protocol::parse_frame(data) {
    Ok(f) => f,
    Err(e) => {
      warn!("Frame parse failed: {:?}", defmt::Debug2Format(&e));
      let code = match e {
        microbit_ble_protocol::ParseError::BadCrc => ERR_BAD_CRC,
        _ => ERR_BAD_FRAME,
      };
      return Response::build(CMD_ERROR, &[code]);
    }
  };

  match frame.cmd {
    CMD_PING => {
      info!("PING received");
      Response::build(CMD_PONG, &[])
    }

    CMD_LED_SET => {
      if frame.payload.len() != 25 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::set_frame_from_bytes(frame.payload);
      info!("LED matrix updated");
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_CLEAR => {
      led_matrix::clear_frame();
      info!("LED matrix cleared");
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_CHAR => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::show_char(frame.payload[0]);
      info!("LED display char: {}", frame.payload[0] as char);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_TEMP_GET => {
      let payload: [u8; 4] = match temperature_celsius(sd) {
        Ok(t) => {
          // I30F2: 30 integer bits + 2 fractional bits (each LSB = 0.25°C)
          // Convert to i32 in units of 0.01°C (avoid floating point)
          // value_in_quarters = t.to_bits()  i.e. 4x temperature
          // hundredths = quarters * 25
          let quarters: i32 = t.to_bits();
          let hundredths: i32 = quarters * 25;
          info!("Temperature: {} (0.01°C)", hundredths);
          hundredths.to_le_bytes()
        }
        Err(_) => {
          warn!("Temperature read failed");
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
      warn!("Unknown command: 0x{:02X}", other);
      Response::build(CMD_ERROR, &[ERR_UNKNOWN_CMD])
    }
  }
}
