//! Shearwater Predator and Petrel family
//! ([spec](https://michidk.github.io/divecomputer-rs/spec/families/shearwater.html)).
//!
//! Implements the Petrel-family download: identify the device, walk the dive manifest
//! newest first, and upload each new dive compressed. The Predator's full-memory download
//! is not implemented yet.
//!
//! Over Bluetooth classic (RFCOMM) the protocol runs on any [`ByteStream`]. Over BLE,
//! wrap the characteristic's [`PacketLink`](divecomputer_core::transport::PacketLink) in
//! a [`BleStream`]:
//!
//! ```no_run
//! # use core::ops::ControlFlow;
//! # use divecomputer_core::{Result, transport::{Clock, PacketLink}};
//! use divecomputer_protocol::shearwater::{BleStream, Shearwater, Variant};
//!
//! # async fn example(link: impl PacketLink, clock: impl Clock) -> Result<()> {
//! let variant = Variant::for_device_name("Perdix 2 1A2B3C");
//! let mut device = Shearwater::connect(BleStream::new(link, variant), clock, variant).await?;
//! let identity = device.identify().await?;
//! device
//!     .download(&identity, None, |dive| {
//!         println!("dive {:02X?}: {} bytes", dive.fingerprint(), dive.data().len());
//!         ControlFlow::Continue(())
//!     })
//!     .await?;
//! device.close().await?;
//! # Ok(())
//! # }
//! ```

mod ble;
mod compression;
mod log;
mod packet;

use alloc::{format, string::String, vec::Vec};
use core::{ops::ControlFlow, time::Duration};

use divecomputer_core::{
    Error, Result,
    dive::RawDive,
    slip::SlipDecoder,
    transport::{ByteStream, Clock},
};

pub use ble::{BleStream, CHARACTERISTIC_UUID, SERVICE_UUID};
pub use log::parse;
pub use packet::Variant;

use compression::Stream;

/// Read data item (spec: "Commands").
const CMD_READ_ITEM: u8 = 0x22;
/// Write data item.
const CMD_WRITE_ITEM: u8 = 0x2E;
/// Start upload.
const CMD_UPLOAD_START: u8 = 0x35;
/// Fetch upload block.
const CMD_UPLOAD_BLOCK: u8 = 0x36;
/// Finish upload.
const CMD_UPLOAD_FINISH: u8 = 0x37;
/// A positive response code is the command code plus this offset.
const RESPONSE_OFFSET: u8 = 0x40;
/// Negative response: `7F <command> <error>`.
const NEGATIVE_RESPONSE: u8 = 0x7F;
/// Upload address-and-length format: 4-byte address, 3-byte size.
const UPLOAD_ADDRESS_FORMAT: u8 = 0x34;

/// Data item: serial number as 8 ASCII hex digits (spec: "Data items").
const ITEM_SERIAL: u16 = 0x8010;
/// Data item: firmware version as ASCII text.
const ITEM_FIRMWARE: u16 = 0x8011;
/// Data item: log-upload descriptor; bytes 1–4 are the dive base address.
const ITEM_LOG_UPLOAD: u16 = 0x8021;
/// Data item: model code.
const ITEM_MODEL: u16 = 0x8060;
/// Data item: write `00` to end the session.
const ITEM_SESSION_END: u16 = 0x9020;

/// Manifest page address (spec: "Upload address space").
const MANIFEST_ADDRESS: u32 = 0xE000_0000;
/// Manifest page size: 48 entries of 32 bytes.
const MANIFEST_SIZE: usize = 0x600;
/// Size of one manifest entry.
const MANIFEST_ENTRY_SIZE: usize = 32;
/// Entries per manifest page; a full page means another page follows.
const MANIFEST_ENTRIES: usize = MANIFEST_SIZE / MANIFEST_ENTRY_SIZE;
/// Manifest entry marker of a valid dive.
const ENTRY_VALID: [u8; 2] = [0xA5, 0xC4];
/// Manifest entry marker of a deleted dive.
const ENTRY_DELETED: [u8; 2] = [0x5A, 0x23];
/// Upload size meaning "until the end of the object", used for dives.
const SIZE_TO_END: u32 = 0x00FF_FFFF;

/// Wait after connecting before the first command (spec: 300–800 ms).
const CONNECT_DELAY: Duration = Duration::from_millis(500);
/// Response timeout; the spec recommends at least 3 s.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
/// Pause before the session-end command.
const CLOSE_DELAY: Duration = Duration::from_millis(500);

/// Shearwater model, from data item `0x8060` (spec: "Devices").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Model {
    /// Predator (2).
    Predator,
    /// Petrel and Petrel 2, which share code 3.
    Petrel,
    /// NERD (4).
    Nerd,
    /// Perdix (5).
    Perdix,
    /// Perdix AI (6).
    PerdixAi,
    /// NERD 2 (7).
    Nerd2,
    /// Teric (8).
    Teric,
    /// Peregrine (9).
    Peregrine,
    /// Petrel 3 (10).
    Petrel3,
    /// Perdix 2 (11).
    Perdix2,
    /// Tern and Tern TX (12).
    Tern,
    /// Peregrine TX (13).
    PeregrineTx,
    /// Perdix 3 (14).
    Perdix3,
    /// A model code not in the table.
    Unknown(u8),
}

impl Model {
    /// The model for a model code.
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match code {
            2 => Self::Predator,
            3 => Self::Petrel,
            4 => Self::Nerd,
            5 => Self::Perdix,
            6 => Self::PerdixAi,
            7 => Self::Nerd2,
            8 => Self::Teric,
            9 => Self::Peregrine,
            10 => Self::Petrel3,
            11 => Self::Perdix2,
            12 => Self::Tern,
            13 => Self::PeregrineTx,
            14 => Self::Perdix3,
            other => Self::Unknown(other),
        }
    }

    /// The product name, `None` for an unknown model code. Petrel and Petrel 2 share a
    /// code and are both named `Petrel`.
    #[must_use]
    pub const fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::Predator => "Predator",
            Self::Petrel => "Petrel",
            Self::Nerd => "NERD",
            Self::Perdix => "Perdix",
            Self::PerdixAi => "Perdix AI",
            Self::Nerd2 => "NERD 2",
            Self::Teric => "Teric",
            Self::Peregrine => "Peregrine",
            Self::Petrel3 => "Petrel 3",
            Self::Perdix2 => "Perdix 2",
            Self::Tern => "Tern",
            Self::PeregrineTx => "Peregrine TX",
            Self::Perdix3 => "Perdix 3",
            Self::Unknown(_) => return None,
        })
    }
}

/// What a device reports about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    serial: u32,
    firmware: String,
    model: Model,
    dive_base: u32,
}

impl Identity {
    /// Serial number; devices show it as 8 hex digits.
    #[must_use]
    pub const fn serial(&self) -> u32 {
        self.serial
    }

    /// Firmware version text as reported by the device.
    #[must_use]
    pub fn firmware(&self) -> &str {
        &self.firmware
    }

    /// The model.
    #[must_use]
    pub const fn model(&self) -> Model {
        self.model
    }
}

/// Whether an upload is requested compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    Raw,
    Compressed,
}

/// A connection to a Shearwater dive computer.
#[derive(Debug)]
pub struct Shearwater<S, C> {
    stream: S,
    clock: C,
    variant: Variant,
    decoder: SlipDecoder,
    received: Vec<u8>,
    position: usize,
}

impl<S: ByteStream, C: Clock> Shearwater<S, C> {
    /// Starts a session on a freshly opened link: waits for the device to settle and
    /// discards pending input.
    ///
    /// # Errors
    ///
    /// A transport error if the link cannot be purged.
    pub async fn connect(mut stream: S, clock: C, variant: Variant) -> Result<Self> {
        clock.sleep(CONNECT_DELAY).await;
        stream.purge().await?;
        Ok(Self {
            stream,
            clock,
            variant,
            decoder: SlipDecoder::new(variant.max_frame()),
            received: Vec::new(),
            position: 0,
        })
    }

    /// Reads the device's serial number, firmware, model and log-upload descriptor.
    ///
    /// # Errors
    ///
    /// Transport and protocol errors, and [`Error::Unsupported`] for an unknown log
    /// format.
    pub async fn identify(&mut self) -> Result<Identity> {
        let serial = self.read_item(ITEM_SERIAL).await?;
        let serial = core::str::from_utf8(&serial)
            .ok()
            .and_then(|text| u32::from_str_radix(text, 16).ok())
            .ok_or_else(|| {
                Error::Protocol(format!("serial number {serial:02X?} is not 8 hex digits"))
            })?;
        let firmware = self.read_item(ITEM_FIRMWARE).await?;
        let firmware = String::from_utf8_lossy(&firmware)
            .trim_end_matches('\0')
            .into();
        let model = match self.read_item(ITEM_MODEL).await?.as_slice() {
            &[code] => Model::from_code(code),
            other => {
                return Err(Error::Protocol(format!(
                    "model code {other:02X?} is not one byte"
                )));
            }
        };
        let descriptor = self.read_item(ITEM_LOG_UPLOAD).await?;
        let dive_base = dive_base(&descriptor)?;
        Ok(Identity {
            serial,
            firmware,
            model,
            dive_base,
        })
    }

    /// Downloads dives newest first and hands each to `on_dive`.
    ///
    /// Stops at the dive whose fingerprint equals `fingerprint` (not delivered), when
    /// `on_dive` returns [`ControlFlow::Break`], or after the oldest dive.
    ///
    /// # Errors
    ///
    /// Transport and protocol errors, and [`Error::Unsupported`] for the Predator.
    pub async fn download(
        &mut self,
        identity: &Identity,
        fingerprint: Option<&[u8]>,
        mut on_dive: impl FnMut(RawDive) -> ControlFlow<()>,
    ) -> Result<()> {
        if identity.model == Model::Predator {
            return Err(Error::Unsupported(
                "Shearwater Predator full-memory download",
            ));
        }
        for entry in self.read_manifest(fingerprint).await? {
            let address = identity
                .dive_base
                .checked_add(entry.offset)
                .ok_or_else(|| {
                    Error::Protocol(format!(
                        "dive offset {:#010x} overflows the base",
                        entry.offset
                    ))
                })?;
            let data = self
                .upload(address, SIZE_TO_END, Encoding::Compressed)
                .await?;
            if on_dive(RawDive::new(entry.fingerprint.to_vec(), data)).is_break() {
                break;
            }
        }
        Ok(())
    }

    /// Ends the session so the device leaves its communication screen, and returns the
    /// link.
    ///
    /// # Errors
    ///
    /// A transport error if the session-end command cannot be sent.
    pub async fn close(mut self) -> Result<S> {
        self.clock.sleep(CLOSE_DELAY).await;
        let [hi, lo] = ITEM_SESSION_END.to_be_bytes();
        // The device does not answer this command.
        self.send(&[CMD_WRITE_ITEM, hi, lo, 0x00]).await?;
        Ok(self.stream)
    }

    /// Reads the manifest pages and returns the entries newer than `fingerprint`.
    async fn read_manifest(&mut self, fingerprint: Option<&[u8]>) -> Result<Vec<ManifestEntry>> {
        let mut entries = Vec::new();
        let mut previous_page = Vec::new();
        loop {
            let page = self
                .upload(MANIFEST_ADDRESS, size_u32(MANIFEST_SIZE), Encoding::Raw)
                .await?;
            // shortcut: paging by re-reading the manifest address is unconfirmed (spec open
            // question 5); stop if the device returns the same page again.
            if page == previous_page {
                return Ok(entries);
            }
            let mut used: usize = 0;
            for raw in page.chunks_exact(MANIFEST_ENTRY_SIZE) {
                match raw.first_chunk::<2>() {
                    Some(&ENTRY_DELETED) => {}
                    Some(&ENTRY_VALID) => {
                        let entry = ManifestEntry::parse(raw)?;
                        if fingerprint == Some(entry.fingerprint.as_slice()) {
                            return Ok(entries);
                        }
                        entries.push(entry);
                    }
                    _ => return Ok(entries),
                }
                used = used.saturating_add(1);
            }
            if used < MANIFEST_ENTRIES {
                return Ok(entries);
            }
            previous_page = page;
        }
    }

    /// Reads one data item and returns its value.
    async fn read_item(&mut self, id: u16) -> Result<Vec<u8>> {
        let [hi, lo] = id.to_be_bytes();
        let response = self.transact(&[CMD_READ_ITEM, hi, lo]).await?;
        match response.split_first_chunk::<2>() {
            Some((&echo, value)) if echo == [hi, lo] => Ok(value.to_vec()),
            _ => Err(Error::Protocol(format!(
                "read of data item {id:#06x} answered with {response:02X?}"
            ))),
        }
    }

    /// Uploads `size` bytes from `address` (start, fetch blocks, finish).
    async fn upload(&mut self, address: u32, size: u32, encoding: Encoding) -> Result<Vec<u8>> {
        let flags = match encoding {
            Encoding::Raw => 0x00,
            Encoding::Compressed => 0x10,
        };
        let [_, size @ ..] = size.to_be_bytes();
        let mut request = Vec::with_capacity(10);
        request.extend_from_slice(&[CMD_UPLOAD_START, flags, UPLOAD_ADDRESS_FORMAT]);
        request.extend_from_slice(&address.to_be_bytes());
        request.extend_from_slice(&size);
        let response = self.transact(&request).await?;
        let max_block = max_block_len(&response)?;
        if max_block > self.variant.max_block() {
            return Err(Error::Protocol(format!(
                "upload block size {max_block} exceeds {}",
                self.variant.max_block()
            )));
        }

        let size = usize::try_from(u32::from_be_bytes([0, size[0], size[1], size[2]]))
            .map_err(|_| Error::Unsupported("upload size exceeds memory"))?;
        let mut data = Vec::new();
        let mut seq: u8 = 1;
        loop {
            let response = self.transact(&self.variant.block_request(seq)).await?;
            let block = match response.split_first() {
                Some((&echo, block)) if echo == seq && block.len() <= max_block => block,
                _ => {
                    return Err(Error::Protocol(format!(
                        "upload block {seq} answered with {} bytes",
                        response.len()
                    )));
                }
            };
            let done = match encoding {
                Encoding::Compressed => {
                    compression::decode_zero_runs(block, &mut data)? == Stream::Ended
                }
                Encoding::Raw if block.is_empty() => {
                    return Err(Error::Protocol(format!("upload block {seq} is empty")));
                }
                Encoding::Raw => {
                    data.extend_from_slice(block);
                    data.len() >= size
                }
            };
            if done {
                break;
            }
            seq = seq.wrapping_add(1);
        }
        if self.transact(&[CMD_UPLOAD_FINISH]).await? != [0x00] {
            return Err(Error::Protocol("upload finish not acknowledged".into()));
        }
        match encoding {
            Encoding::Raw => data.truncate(size),
            Encoding::Compressed => compression::undo_xor_delta(&mut data),
        }
        Ok(data)
    }

    /// Sends `request` and returns the positive response's payload after the response
    /// code.
    async fn transact(&mut self, request: &[u8]) -> Result<Vec<u8>> {
        self.send(request).await?;
        let frame = self.receive_frame().await?;
        let payload = packet::decode_response(self.variant, &frame)?;
        let command = request.first().copied().unwrap_or_default();
        match payload {
            [NEGATIVE_RESPONSE, rejected, code, ..] if *rejected == command => {
                Err(Error::Rejected {
                    command,
                    code: *code,
                })
            }
            [code, rest @ ..] if *code == command.wrapping_add(RESPONSE_OFFSET) => {
                Ok(rest.to_vec())
            }
            _ => Err(Error::Protocol(format!(
                "command {command:#04x} answered with {payload:02X?}"
            ))),
        }
    }

    async fn send(&mut self, payload: &[u8]) -> Result<()> {
        let mut frame = Vec::new();
        packet::encode_request(self.variant, payload, &mut frame)?;
        self.stream.write(&frame).await
    }

    /// Returns the next non-empty de-SLIPped frame.
    async fn receive_frame(&mut self) -> Result<Vec<u8>> {
        loop {
            while let Some(&byte) = self.received.get(self.position) {
                self.position = self.position.saturating_add(1);
                match self.decoder.push(byte)? {
                    Some(frame) if !frame.is_empty() => return Ok(frame),
                    _ => {}
                }
            }
            self.received.resize(256, 0);
            let count = self
                .stream
                .read(&mut self.received, RESPONSE_TIMEOUT)
                .await?;
            self.received.truncate(count);
            self.position = 0;
        }
    }
}

/// One valid manifest entry (spec: "Petrel family manifest").
struct ManifestEntry {
    fingerprint: [u8; 4],
    offset: u32,
}

impl ManifestEntry {
    fn parse(raw: &[u8]) -> Result<Self> {
        let field = |offset: usize| {
            raw.get(offset..)
                .and_then(<[u8]>::first_chunk::<4>)
                .copied()
                .ok_or_else(|| {
                    Error::Protocol(format!("manifest entry too short for offset {offset:#x}"))
                })
        };
        Ok(Self {
            fingerprint: field(0x04)?,
            offset: u32::from_be_bytes(field(0x14)?),
        })
    }
}

/// Maps the log-upload descriptor (data item `0x8021`) to the dive base address.
fn dive_base(descriptor: &[u8]) -> Result<u32> {
    let base = descriptor
        .get(1..)
        .and_then(<[u8]>::first_chunk::<4>)
        .map(|bytes| u32::from_be_bytes(*bytes))
        .ok_or_else(|| {
            Error::Protocol(format!("log-upload descriptor {descriptor:02X?} too short"))
        })?;
    match base {
        0x8000_0000 => Ok(base),
        0x9000_0000 | 0xC000_0000 | 0xDD00_0000 => Ok(0xC000_0000),
        _ => Err(Error::Unsupported("unknown Shearwater log format")),
    }
}

/// Parses the start-upload response `<n << 4> <max block: n bytes, big-endian>`.
fn max_block_len(response: &[u8]) -> Result<usize> {
    let invalid = || Error::Protocol(format!("start-upload response {response:02X?}"));
    let (&format, rest) = response.split_first().ok_or_else(invalid)?;
    let field = rest
        .get(..usize::from(format >> 4))
        .filter(|field| (1..=4).contains(&field.len()))
        .ok_or_else(invalid)?;
    let value = field
        .iter()
        .fold(0u32, |acc, &byte| (acc << 8) | u32::from(byte));
    usize::try_from(value).map_err(|_| invalid())
}

fn size_u32(size: usize) -> u32 {
    u32::try_from(size).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_log_upload_descriptor() {
        let descriptor = |base: u32| {
            let mut bytes = [0; 9];
            bytes[1..5].copy_from_slice(&base.to_be_bytes());
            bytes
        };
        assert_eq!(dive_base(&descriptor(0x8000_0000)), Ok(0x8000_0000));
        assert_eq!(dive_base(&descriptor(0x9000_0000)), Ok(0xC000_0000));
        assert_eq!(dive_base(&descriptor(0xDD00_0000)), Ok(0xC000_0000));
        assert!(dive_base(&descriptor(0x1234_5678)).is_err());
        assert!(dive_base(&[0x00, 0x80]).is_err());
    }

    #[test]
    fn parses_start_upload_response() {
        assert_eq!(max_block_len(&[0x10, 0x92]), Ok(146));
        assert_eq!(max_block_len(&[0x20, 0x10, 0x00]), Ok(4096));
        assert!(max_block_len(&[0x00]).is_err());
        assert!(max_block_len(&[0x20, 0x10]).is_err());
        assert!(max_block_len(&[]).is_err());
    }
}
