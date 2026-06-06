//! Utility module: re-exports from shared protocol crate
//!
//! Protocol constants, CRC calculation, frame encoding/decoding are now defined
//! in the shared `microbit-ble-protocol` crate. This module re-exports them
//! for backward compatibility with existing code.

// Re-export all protocol items from the shared crate
pub use microbit_ble_protocol::*;
