//! Checksums and CRCs used by dive computer protocols.
//!
//! Each algorithm is defined once, named after its entry in the
//! [checksum catalogue](https://michidk.github.io/divecomputer-rs/spec/common/checksums.html).
//! CRCs are [`crc::Crc`] instances, so callers get both one-shot
//! ([`Crc::checksum`]) and incremental ([`Crc::digest`]) computation. The simple sums are
//! small digest types with the same `update`/`value` shape.
//!
//! ```
//! use divecomputer_core::checksum::{CRC_16_XMODEM, Sum8, Xor8};
//!
//! // Cressi Goa dive request: CRC over length, command and arguments.
//! assert_eq!(CRC_16_XMODEM.checksum(&[0x02, 0x22, 0x01, 0x00]), 0x36FF);
//!
//! // Suunto version query: XOR over the command bytes.
//! assert_eq!(Xor8::of(&[0x05, 0x00, 0x16, 0x14]).value(), 0x07);
//!
//! // Sum of data plus its negated sum8 is zero.
//! let data = [0x10, 0x20, 0x30];
//! assert_eq!(Sum8::of(&data).negated(), 0xA0);
//! ```

pub use crc::Crc;

/// CRC-8/SMBUS: polynomial `07`, init `00`, no reflection, no final XOR.
pub const CRC_8_SMBUS: Crc<u8> = Crc::<u8>::new(&crc::CRC_8_SMBUS);

/// Parameters of [`CRC_8_MSB_0X12`].
///
/// Not in the RevEng catalogue: the polynomial `12` has no `x^0` term.
const CRC_8_MSB_0X12_ALGORITHM: crc::Algorithm<u8> = crc::Algorithm {
    width: 8,
    poly: 0x12,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x00,
    check: 0xEA,
    residue: 0x00,
};

/// CRC-8 with polynomial `12`, init `00`, no reflection, no final XOR.
///
/// Used by the Pelagic i330R / Apeks DSX BLE packets.
pub const CRC_8_MSB_0X12: Crc<u8> = Crc::<u8>::new(&CRC_8_MSB_0X12_ALGORITHM);

/// CRC-16/IBM-3740, also known as CRC-16/CCITT-FALSE: polynomial `1021`, init `FFFF`.
pub const CRC_16_IBM_3740: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_3740);

/// CRC-16/XMODEM: polynomial `1021`, init `0000`.
pub const CRC_16_XMODEM: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_XMODEM);

/// CRC-16/IBM-SDLC, also known as CRC-16/X-25: polynomial `1021`, reflected, init and
/// final XOR `FFFF`.
pub const CRC_16_IBM_SDLC: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);

/// CRC-16/ARC: polynomial `8005`, reflected, init `0000`.
pub const CRC_16_ARC: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_ARC);

/// CRC-16/MODBUS: polynomial `8005`, reflected, init `FFFF`.
pub const CRC_16_MODBUS: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_MODBUS);

/// CRC-32/ISO-HDLC, the common "CRC-32": polynomial `04C11DB7`, reflected.
pub const CRC_32_ISO_HDLC: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

/// CRC-32/BZIP2: polynomial `04C11DB7`, not reflected.
pub const CRC_32_BZIP2: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_BZIP2);

/// Sum of bytes modulo 256.
///
/// Covers the catalogue entries `sum8`, `sum8-neg` ([`negated`](Self::negated)) and
/// `sum8-inv` ([`inverted`](Self::inverted)). Some families seed the sum; use
/// [`with_init`](Self::with_init) for those.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sum8(u8);

impl Sum8 {
    /// Creates a sum starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Creates a sum starting at `init`.
    #[must_use]
    pub const fn with_init(init: u8) -> Self {
        Self(init)
    }

    /// Computes the sum of `data`, starting at zero.
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        let mut sum = Self::new();
        sum.update(data);
        sum
    }

    /// Adds `data` to the sum.
    pub fn update(&mut self, data: &[u8]) {
        self.0 = data.iter().fold(self.0, |acc, &b| acc.wrapping_add(b));
    }

    /// The sum modulo 256.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }

    /// The two's complement of the sum, so that data plus this byte sums to zero.
    #[must_use]
    pub const fn negated(self) -> u8 {
        self.0.wrapping_neg()
    }

    /// The ones' complement of the sum.
    #[must_use]
    pub const fn inverted(self) -> u8 {
        !self.0
    }
}

/// Sum of bytes (each `0..=255`) modulo 65536: the catalogue entry `sum16`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sum16(u16);

impl Sum16 {
    /// Creates a sum starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Creates a sum starting at `init`.
    #[must_use]
    pub const fn with_init(init: u16) -> Self {
        Self(init)
    }

    /// Computes the sum of `data`, starting at zero.
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        let mut sum = Self::new();
        sum.update(data);
        sum
    }

    /// Adds `data` to the sum.
    pub fn update(&mut self, data: &[u8]) {
        self.0 = data
            .iter()
            .fold(self.0, |acc, &b| acc.wrapping_add(u16::from(b)));
    }

    /// The sum modulo 65536.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Sum of the high and low nibble of every byte, modulo 256: the catalogue entry
/// `nibble-sum`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NibbleSum(u8);

impl NibbleSum {
    /// Creates a sum starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Computes the nibble sum of `data`, starting at zero.
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        let mut sum = Self::new();
        sum.update(data);
        sum
    }

    /// Adds the nibbles of `data` to the sum.
    pub fn update(&mut self, data: &[u8]) {
        self.0 = data.iter().fold(self.0, |acc, &b| {
            acc.wrapping_add(b >> 4).wrapping_add(b & 0x0F)
        });
    }

    /// The sum modulo 256.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// XOR of all bytes: the catalogue entry `xor8`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Xor8(u8);

impl Xor8 {
    /// Creates an XOR starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Computes the XOR of `data`, starting at zero.
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        let mut xor = Self::new();
        xor.update(data);
        xor
    }

    /// XORs `data` into the value.
    pub fn update(&mut self, data: &[u8]) {
        self.0 = data.iter().fold(self.0, |acc, &b| acc ^ b);
    }

    /// The XOR of all bytes.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The standard check input used by the catalogue.
    const CHECK_INPUT: &[u8] = b"123456789";

    #[test]
    fn crc_check_values_match_catalogue() {
        assert_eq!(CRC_8_SMBUS.checksum(CHECK_INPUT), 0xF4);
        assert_eq!(CRC_8_MSB_0X12.checksum(CHECK_INPUT), 0xEA);
        assert_eq!(CRC_16_IBM_3740.checksum(CHECK_INPUT), 0x29B1);
        assert_eq!(CRC_16_XMODEM.checksum(CHECK_INPUT), 0x31C3);
        assert_eq!(CRC_16_IBM_SDLC.checksum(CHECK_INPUT), 0x906E);
        assert_eq!(CRC_16_ARC.checksum(CHECK_INPUT), 0xBB3D);
        assert_eq!(CRC_16_MODBUS.checksum(CHECK_INPUT), 0x4B37);
        assert_eq!(CRC_32_ISO_HDLC.checksum(CHECK_INPUT), 0xCBF4_3926);
        assert_eq!(CRC_32_BZIP2.checksum(CHECK_INPUT), 0xFC89_1918);
    }

    /// The byte-wise formulation given in the catalogue for `crc-8-msb-0x12`.
    fn crc_8_msb_0x12_bytewise(data: &[u8]) -> u8 {
        data.iter().fold(0u8, |crc, &b| {
            let a = crc ^ b;
            let t = a ^ (a >> 4) ^ (a >> 7);
            (t << 4) ^ (t << 1)
        })
    }

    #[test]
    fn crc_8_msb_0x12_matches_bytewise_formulation() {
        let mut data = [0u8; 64];
        for (i, byte) in (0u8..).zip(data.iter_mut()) {
            *byte = i.wrapping_mul(37).wrapping_add(11);
        }
        for len in 0..=data.len() {
            let slice = data.get(..len).unwrap_or_default();
            assert_eq!(
                CRC_8_MSB_0X12.checksum(slice),
                crc_8_msb_0x12_bytewise(slice)
            );
        }
    }

    #[test]
    fn sum_check_values_match_catalogue() {
        let sum8 = Sum8::of(CHECK_INPUT);
        assert_eq!(sum8.value(), 0xDD);
        assert_eq!(sum8.negated(), 0x23);
        assert_eq!(sum8.inverted(), 0x22);
        assert_eq!(Sum16::of(CHECK_INPUT).value(), 0x01DD);
        assert_eq!(NibbleSum::of(CHECK_INPUT).value(), 0x48);
        assert_eq!(Xor8::of(CHECK_INPUT).value(), 0x31);
    }

    #[test]
    fn worked_examples_match_spec() {
        // Mares ASCII read command: sum of the characters "51000120".
        assert_eq!(Sum8::of(b"51000120").value(), 0x89);
        // Cressi Goa dive request for dive id 1.
        assert_eq!(CRC_16_XMODEM.checksum(&[0x02, 0x22, 0x01, 0x00]), 0x36FF);
        // Suunto version query.
        assert_eq!(Xor8::of(&[0x05, 0x00, 0x16, 0x14]).value(), 0x07);
    }

    #[test]
    fn incremental_updates_equal_one_shot() {
        let (head, tail) = CHECK_INPUT.split_at(4);
        let mut sum = Sum8::new();
        sum.update(head);
        sum.update(tail);
        assert_eq!(sum, Sum8::of(CHECK_INPUT));

        let mut digest = CRC_16_IBM_3740.digest();
        digest.update(head);
        digest.update(tail);
        assert_eq!(digest.finalize(), CRC_16_IBM_3740.checksum(CHECK_INPUT));
    }

    #[test]
    fn seeded_sums_start_at_init() {
        assert_eq!(Sum8::with_init(0xFF).value(), 0xFF);
        assert_eq!(Sum8::with_init(0x01).negated(), 0xFF);
        assert_eq!(Sum16::with_init(0xFFFF).value(), 0xFFFF);
        let mut sum = Sum16::with_init(0xFFFF);
        sum.update(&[0x02]);
        assert_eq!(sum.value(), 0x0001);
    }
}
