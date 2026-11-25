//! Decoding of compressed uploads
//! ([spec](https://michidk.github.io/divecomputer-rs/spec/families/shearwater.html#compression)):
//! 9-bit zero-run codes per block, then a 32-byte XOR delta over the whole output.

use alloc::{format, vec::Vec};

use divecomputer_core::{Error, Result};

/// Distance of the XOR delta: each 32-byte record is stored relative to the one before.
const DELTA_STRIDE: usize = 32;

/// Whether the zero-run stream continues after a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stream {
    /// More blocks follow.
    Continues,
    /// The block held the end-of-stream code.
    Ended,
}

/// Decodes the 9-bit zero-run codes of one block and appends the output to `out`.
pub(crate) fn decode_zero_runs(block: &[u8], out: &mut Vec<u8>) -> Result<Stream> {
    if block.len().checked_mul(8).is_none_or(|bits| bits % 9 != 0) {
        return Err(Error::Protocol(format!(
            "compressed block of {} bytes is not a whole number of 9-bit codes",
            block.len()
        )));
    }
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    for &byte in block {
        bits = (bits << 8) | u32::from(byte);
        bit_count = bit_count.saturating_add(8);
        if let Some(remaining) = bit_count.checked_sub(9) {
            let code = (bits >> remaining) & 0x1FF;
            bit_count = remaining;
            bits &= !(u32::MAX << remaining);
            match u8::try_from(code) {
                Ok(0) => return Ok(Stream::Ended),
                Ok(zeros) => out.resize(out.len().saturating_add(usize::from(zeros)), 0),
                Err(_) => out.push(code.to_le_bytes()[0]),
            }
        }
    }
    Ok(Stream::Continues)
}

/// Undoes the XOR delta in place: every byte from offset 32 on is combined by XOR with the
/// (already decoded) byte 32 positions earlier.
pub(crate) fn undo_xor_delta(data: &mut [u8]) {
    for (current, previous) in (DELTA_STRIDE..data.len()).zip(0..) {
        if let Some(&earlier) = data.get(previous) {
            if let Some(byte) = data.get_mut(current) {
                *byte ^= earlier;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn worked_decompression_example() {
        let mut out = Vec::new();
        let block = [0xD2, 0x87, 0xE0, 0x20, 0x30, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(decode_zero_runs(&block, &mut out), Ok(Stream::Ended));

        let mut expected = vec![0xA5];
        expected.extend([0; 31]);
        expected.extend([0x01, 0x00, 0x00, 0x00]);
        assert_eq!(out, expected);

        undo_xor_delta(&mut out);
        if let Some(byte) = expected.get_mut(32) {
            *byte = 0xA4;
        }
        assert_eq!(out, expected);
    }

    #[test]
    fn continues_without_end_code() {
        let mut out = Vec::new();
        // Eight literal codes 0x1FF: 72 one bits.
        assert_eq!(
            decode_zero_runs(&[0xFF; 9], &mut out),
            Ok(Stream::Continues)
        );
        assert_eq!(out, [0xFF; 8]);
    }

    #[test]
    fn rejects_partial_codes() {
        let mut out = Vec::new();
        assert!(decode_zero_runs(&[0xFF; 8], &mut out).is_err());
    }
}
