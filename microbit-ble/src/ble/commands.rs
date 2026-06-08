//! Command dispatch: parse binary frames received on NUS RX, execute corresponding actions and construct responses.
//!
//! Called by the BLE main loop via [`handle_rx`], input is the RX written bytes, output is the response bytes to write back via TX.

use defmt::{info, warn};
use nrf_softdevice::Softdevice;
use nrf_softdevice::temperature_celsius;

use super::buttons;
use super::led_matrix;
use super::motion;
use super::sound;
use super::touch;
use microbit_ble_protocol::{
  CMD_ACCEL_DATA, CMD_ACCEL_SUBSCRIBE, CMD_BTN_EVENT, CMD_BTN_SUBSCRIBE, CMD_ECHO, CMD_ECHO_RESP,
  CMD_ERROR, CMD_LED_ACK, CMD_LED_BRIGHTNESS, CMD_LED_CHAR, CMD_LED_CLEAR, CMD_LED_SCROLL,
  CMD_LED_SET, CMD_MAGNET_DATA, CMD_MAGNET_SUBSCRIBE, CMD_PING, CMD_PONG, CMD_SOUND_ACK,
  CMD_SOUND_PLAY, CMD_SOUND_STOP, CMD_TEMP_GET, CMD_TEMP_RESP, CMD_TOUCH_EVENT,
  CMD_TOUCH_SUBSCRIBE, ERR_BAD_CRC, ERR_BAD_FRAME, ERR_BAD_PAYLOAD, ERR_UNKNOWN_CMD, MAX_FRAME_LEN,
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

/// Encode touch event into a frame
pub fn encode_touch_event(evt: touch::TouchEvent) -> Option<Response> {
  let payload = [evt.id as u8, evt.pressed as u8];
  Response::build(CMD_TOUCH_EVENT, &payload)
}

/// Encode accelerometer data into a frame
pub fn encode_accel_data(data: motion::AccelData) -> Option<Response> {
  let mut payload = [0u8; 6];
  payload[0..2].copy_from_slice(&data.x.to_le_bytes());
  payload[2..4].copy_from_slice(&data.y.to_le_bytes());
  payload[4..6].copy_from_slice(&data.z.to_le_bytes());
  Response::build(CMD_ACCEL_DATA, &payload)
}

/// Encode magnetometer data into a frame
pub fn encode_magnet_data(data: motion::MagnetData) -> Option<Response> {
  let mut payload = [0u8; 6];
  payload[0..2].copy_from_slice(&data.x.to_le_bytes());
  payload[2..4].copy_from_slice(&data.y.to_le_bytes());
  payload[4..6].copy_from_slice(&data.z.to_le_bytes());
  Response::build(CMD_MAGNET_DATA, &payload)
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

    CMD_TOUCH_SUBSCRIBE => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      touch::set_subscribed(frame.payload[0] != 0);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_SOUND_PLAY => {
      if frame.payload.len() != 4 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      let freq = u16::from_le_bytes([frame.payload[0], frame.payload[1]]);
      let duration = u16::from_le_bytes([frame.payload[2], frame.payload[3]]);
      sound::play_tone(freq, duration);
      Response::build(CMD_SOUND_ACK, &[0])
    }

    CMD_SOUND_STOP => {
      sound::stop_tone();
      Response::build(CMD_SOUND_ACK, &[0])
    }

    CMD_ACCEL_SUBSCRIBE => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      motion::set_accel_subscribed(frame.payload[0] != 0);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_MAGNET_SUBSCRIBE => {
      if frame.payload.len() != 1 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      motion::set_magnet_subscribed(frame.payload[0] != 0);
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_BRIGHTNESS => {
      if frame.payload.len() != 25 {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::set_brightness_from_bytes(frame.payload);
      info!("LED brightness updated");
      Response::build(CMD_LED_ACK, &[0])
    }

    CMD_LED_SCROLL => {
      if frame.payload.is_empty() {
        return Response::build(CMD_ERROR, &[ERR_BAD_PAYLOAD]);
      }
      led_matrix::scroll_text(frame.payload);
      info!("LED scroll text queued");
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
