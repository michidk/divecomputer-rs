//! SLIP framing ([RFC 1055](https://www.rfc-editor.org/rfc/rfc1055)).
//!
//! Each frame ends with `C0`; inside a frame `C0` is sent as `DB DC` and `DB` as `DB DD`.
//! Senders send no leading `C0`.
//!
//! ```
//! use divecomputer_core::slip::{SlipDecoder, encode};
//!
//! let mut frame = Vec::new();
//! encode(&[0x01, 0xC0, 0xDB], &mut frame);
//! assert_eq!(frame, [0x01, 0xDB, 0xDC, 0xDB, 0xDD, 0xC0]);
//!
//! let mut decoder = SlipDecoder::new(16);
//! let decoded: Vec<_> = frame.iter().filter_map(|&b| decoder.push(b).transpose()).collect();
//! assert_eq!(decoded, [Ok(vec![0x01, 0xC0, 0xDB])]);
//! ```

use alloc::{format, vec::Vec};

use crate::{Error, Result};

const END: u8 = 0xC0;
const ESC: u8 = 0xDB;
const ESC_END: u8 = 0xDC;
const ESC_ESC: u8 = 0xDD;

/// Appends the SLIP encoding of `data`, including the terminating `C0`, to `out`.
pub fn encode(data: &[u8], out: &mut Vec<u8>) {
    for &byte in data {
        match byte {
            END => out.extend_from_slice(&[ESC, ESC_END]),
            ESC => out.extend_from_slice(&[ESC, ESC_ESC]),
            _ => out.push(byte),
        }
    }
    out.push(END);
}

/// Incremental SLIP decoder: feed it received bytes one at a time.
#[derive(Clone, Debug)]
pub struct SlipDecoder {
    frame: Vec<u8>,
    escaped: bool,
    max_len: usize,
}

impl SlipDecoder {
    /// Creates a decoder that rejects frames longer than `max_len` decoded bytes.
    #[must_use]
    pub const fn new(max_len: usize) -> Self {
        Self {
            frame: Vec::new(),
            escaped: false,
            max_len,
        }
    }

    /// Feeds one received byte. Returns the decoded frame when `byte` ends it. Empty
    /// frames (consecutive `C0`s) are returned as empty vectors for the caller to skip.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] for an escape byte followed by anything other than `DC` or
    /// `DD`, or for a frame longer than the limit. The partial frame is discarded and
    /// decoding restarts with the next byte.
    pub fn push(&mut self, byte: u8) -> Result<Option<Vec<u8>>> {
        let decoded = match (self.escaped, byte) {
            (true, ESC_END) => END,
            (true, ESC_ESC) => ESC,
            (true, other) => {
                self.reset();
                return Err(Error::Protocol(format!(
                    "SLIP escape followed by {other:#04x}"
                )));
            }
            (false, ESC) => {
                self.escaped = true;
                return Ok(None);
            }
            (false, END) => return Ok(Some(core::mem::take(&mut self.frame))),
            (false, other) => other,
        };
        self.escaped = false;
        if self.frame.len() >= self.max_len {
            self.reset();
            return Err(Error::Protocol(format!(
                "SLIP frame longer than {} bytes",
                self.max_len
            )));
        }
        self.frame.push(decoded);
        Ok(None)
    }

    fn reset(&mut self) {
        self.frame.clear();
        self.escaped = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn decode_all(decoder: &mut SlipDecoder, bytes: &[u8]) -> Vec<Result<Vec<u8>>> {
        bytes
            .iter()
            .filter_map(|&b| decoder.push(b).transpose())
            .collect()
    }

    #[test]
    fn round_trips_every_byte_value() {
        let data: Vec<u8> = (0..=255).collect();
        let mut frame = Vec::new();
        encode(&data, &mut frame);
        let mut decoder = SlipDecoder::new(256);
        assert_eq!(decode_all(&mut decoder, &frame), [Ok(data)]);
    }

    #[test]
    fn reports_empty_frames() {
        let mut decoder = SlipDecoder::new(4);
        assert_eq!(
            decode_all(&mut decoder, &[END, 0x01, END]),
            [Ok(vec![]), Ok(vec![0x01])]
        );
    }

    #[test]
    fn rejects_invalid_escape_and_recovers() {
        let mut decoder = SlipDecoder::new(4);
        let results = decode_all(&mut decoder, &[0x01, ESC, 0x02, 0x03, END]);
        assert!(matches!(results.first(), Some(Err(Error::Protocol(_)))));
        assert_eq!(results.get(1), Some(&Ok(vec![0x03])));
    }

    #[test]
    fn rejects_oversized_frames() {
        let mut decoder = SlipDecoder::new(2);
        let results = decode_all(&mut decoder, &[1, 2, 3, END]);
        assert!(matches!(results.first(), Some(Err(Error::Protocol(_)))));
    }
}
