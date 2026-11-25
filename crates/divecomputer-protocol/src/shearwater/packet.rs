//! Packet header and SLIP framing ([spec](https://michidk.github.io/divecomputer-rs/spec/families/shearwater.html#framing-and-integrity)).

use alloc::{format, vec::Vec};

use divecomputer_core::{Error, Result, slip};

/// The packet header variant a device uses.
///
/// Variant 2 is used only by the Perdix 3; every other model uses variant 1. The host must
/// know the variant before the first command, so it is chosen from the device name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Variant {
    /// 4-byte header with a 1-byte length, at most 254 payload bytes; BLE packets carry a
    /// 2-byte segmentation prefix.
    #[default]
    V1,
    /// 5-byte header with a 2-byte length (Perdix 3); no BLE segmentation prefix.
    V2,
}

impl Variant {
    /// Picks the variant from the Bluetooth device name: `Perdix 3…` uses
    /// [`V2`](Self::V2), every other name [`V1`](Self::V1).
    #[must_use]
    pub fn for_device_name(name: &str) -> Self {
        if name.starts_with("Perdix 3") {
            Self::V2
        } else {
            Self::V1
        }
    }

    /// Largest payload a header of this variant can describe.
    pub(crate) const fn max_payload(self) -> usize {
        match self {
            Self::V1 => 254,
            Self::V2 => 0xFFFF,
        }
    }

    /// Largest upload block the host accepts (spec: "Start upload").
    pub(crate) const fn max_block(self) -> usize {
        match self {
            Self::V1 => 252,
            Self::V2 => 4096,
        }
    }

    const fn header_len(self) -> usize {
        match self {
            Self::V1 => 4,
            Self::V2 => 5,
        }
    }

    /// Largest de-SLIPped frame a device can send.
    pub(crate) const fn max_frame(self) -> usize {
        self.header_len().saturating_add(self.max_payload())
    }

    /// The block fetch command for block number `seq`.
    pub(crate) fn block_request(self, seq: u8) -> Vec<u8> {
        match self {
            Self::V1 => [super::CMD_UPLOAD_BLOCK, seq].to_vec(),
            Self::V2 => [super::CMD_UPLOAD_BLOCK, seq, 0x00].to_vec(),
        }
    }
}

/// Appends the host-to-device packet for `payload`, SLIP-encoded, to `out`.
pub(crate) fn encode_request(variant: Variant, payload: &[u8], out: &mut Vec<u8>) -> Result<()> {
    if payload.len() > variant.max_payload() {
        return Err(Error::Protocol(format!(
            "request payload of {} bytes is too long",
            payload.len()
        )));
    }
    let mut packet = Vec::with_capacity(variant.header_len().saturating_add(payload.len()));
    packet.extend_from_slice(&[0xFF, 0x01]);
    match variant {
        // Length checked above: at most 254, so `+ 1` fits a byte.
        Variant::V1 => packet.extend_from_slice(&[length_byte(payload.len())?, 0x00]),
        Variant::V2 => {
            let len = u16::try_from(payload.len())
                .map_err(|_| Error::Protocol(format!("payload of {} bytes", payload.len())))?;
            packet.push(0x00);
            packet.extend_from_slice(&len.to_be_bytes());
        }
    }
    packet.extend_from_slice(payload);
    slip::encode(&packet, out);
    Ok(())
}

fn length_byte(payload_len: usize) -> Result<u8> {
    payload_len
        .checked_add(1)
        .and_then(|len| u8::try_from(len).ok())
        .ok_or_else(|| Error::Protocol(format!("payload of {payload_len} bytes")))
}

/// Checks the device-to-host header of a de-SLIPped `frame` and returns its payload.
pub(crate) fn decode_response(variant: Variant, frame: &[u8]) -> Result<&[u8]> {
    let (header, payload) = frame
        .split_at_checked(variant.header_len())
        .ok_or_else(|| Error::Protocol(format!("frame of {} bytes has no header", frame.len())))?;
    let declared = match (variant, header) {
        (Variant::V1, &[0x01, 0xFF, len, 0x00]) => usize::from(len).checked_sub(1),
        (Variant::V2, &[0x01, 0xFF, 0x00, hi, lo]) => {
            Some(usize::from(u16::from_be_bytes([hi, lo])))
        }
        _ => {
            return Err(Error::Protocol(format!(
                "invalid response header {header:02X?}"
            )));
        }
    };
    if declared != Some(payload.len()) {
        return Err(Error::Protocol(format!(
            "response header {header:02X?} does not match payload length {}",
            payload.len()
        )));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(variant: Variant, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        encode_request(variant, payload, &mut out).unwrap();
        out
    }

    #[test]
    fn worked_example_frames() {
        assert_eq!(
            request(Variant::V1, &[0x22, 0x80, 0x10]),
            [0xFF, 0x01, 0x04, 0x00, 0x22, 0x80, 0x10, 0xC0]
        );
        assert_eq!(
            request(Variant::V2, &[0x22, 0x80, 0x10]),
            [0xFF, 0x01, 0x00, 0x00, 0x03, 0x22, 0x80, 0x10, 0xC0]
        );
        assert_eq!(
            request(Variant::V2, &Variant::V2.block_request(1)),
            [0xFF, 0x01, 0x00, 0x00, 0x03, 0x36, 0x01, 0x00, 0xC0]
        );
        assert_eq!(
            request(
                Variant::V1,
                &[0x35, 0x10, 0x34, 0x80, 0x00, 0xC0, 0xDB, 0xFF, 0xFF, 0xFF]
            ),
            [
                0xFF, 0x01, 0x0B, 0x00, 0x35, 0x10, 0x34, 0x80, 0x00, 0xDB, 0xDC, 0xDB, 0xDD, 0xFF,
                0xFF, 0xFF, 0xC0
            ]
        );
    }

    #[test]
    fn rejects_oversized_v1_payload() {
        let mut out = Vec::new();
        assert!(encode_request(Variant::V1, &[0; 255], &mut out).is_err());
        assert!(encode_request(Variant::V1, &[0; 254], &mut out).is_ok());
    }

    #[test]
    fn decodes_response_headers() {
        assert_eq!(
            decode_response(Variant::V1, &[0x01, 0xFF, 0x03, 0x00, 0x77, 0x00]),
            Ok(&[0x77, 0x00][..])
        );
        assert_eq!(
            decode_response(Variant::V2, &[0x01, 0xFF, 0x00, 0x00, 0x02, 0x77, 0x00]),
            Ok(&[0x77, 0x00][..])
        );
        assert!(decode_response(Variant::V1, &[0x01, 0xFF, 0x04, 0x00, 0x77, 0x00]).is_err());
        assert!(decode_response(Variant::V1, &[0xFF, 0x01, 0x03, 0x00, 0x77, 0x00]).is_err());
        assert!(decode_response(Variant::V1, &[0x01, 0xFF]).is_err());
    }

    #[test]
    fn picks_variant_from_device_name() {
        assert_eq!(Variant::for_device_name("Perdix 3 1A2B3C"), Variant::V2);
        assert_eq!(Variant::for_device_name("Perdix 2 1A2B3C"), Variant::V1);
        assert_eq!(Variant::for_device_name("Petrel"), Variant::V1);
    }
}
