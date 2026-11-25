//! Bluetooth Low Energy link ([spec](https://michidk.github.io/divecomputer-rs/spec/families/shearwater.html#bluetooth-low-energy)).

use alloc::{collections::VecDeque, format, vec::Vec};
use core::time::Duration;

use divecomputer_core::{
    Error, Result,
    transport::{ByteStream, PacketLink},
};

use super::Variant;

/// Service UUID of the Shearwater BLE profile.
pub const SERVICE_UUID: &str = "fe25c237-0ece-443c-b0aa-e02033e7029d";
/// The one characteristic used for writes and notifications.
pub const CHARACTERISTIC_UUID: &str = "27b7570b-359e-45a3-91bb-cf7e70049bd2";

/// Hosts write at most 20 bytes per packet (the default ATT MTU minus 3).
const MAX_WRITE_LEN: usize = 20;
/// Length of the variant 1 segmentation prefix: packet count and packet index.
const PREFIX_LEN: usize = 2;

/// Turns the Shearwater BLE characteristic into the byte stream the protocol runs on.
///
/// In [`Variant::V1`] every write and notification carries a 2-byte segmentation prefix,
/// which this adapter adds and strips. In [`Variant::V2`] the bytes pass through.
#[derive(Debug)]
pub struct BleStream<P> {
    link: P,
    variant: Variant,
    received: VecDeque<u8>,
}

impl<P: PacketLink> BleStream<P> {
    /// Wraps a link to the Shearwater characteristic with notifications enabled.
    pub const fn new(link: P, variant: Variant) -> Self {
        Self {
            link,
            variant,
            received: VecDeque::new(),
        }
    }

    /// Returns the wrapped link.
    pub fn into_inner(self) -> P {
        self.link
    }

    fn write_len(&self) -> usize {
        self.link.max_packet_len().min(MAX_WRITE_LEN)
    }
}

impl<P: PacketLink> ByteStream for BleStream<P> {
    async fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize> {
        while self.received.is_empty() {
            let packet = self.link.receive(timeout).await?;
            let data = match self.variant {
                Variant::V1 => packet.get(PREFIX_LEN..).ok_or_else(|| {
                    Error::Protocol(format!(
                        "notification of {} bytes has no prefix",
                        packet.len()
                    ))
                })?,
                Variant::V2 => &packet,
            };
            self.received.extend(data);
        }
        let count = buf.len().min(self.received.len());
        for (slot, byte) in buf.iter_mut().zip(self.received.drain(..count)) {
            *slot = byte;
        }
        Ok(count)
    }

    async fn write(&mut self, data: &[u8]) -> Result<()> {
        match self.variant {
            Variant::V1 => {
                let chunk_len = self
                    .write_len()
                    .checked_sub(PREFIX_LEN)
                    .filter(|&len| len > 0)
                    .ok_or(Error::Unsupported("BLE packets too small for the prefix"))?;
                // shortcut: the count byte for multi-packet frames is an open question in
                // the spec; every command a downloader sends fits in one packet.
                let count = u8::try_from(data.len().div_ceil(chunk_len)).map_err(|_| {
                    Error::Protocol(format!("{} bytes need too many packets", data.len()))
                })?;
                for (index, chunk) in (0..=u8::MAX).zip(data.chunks(chunk_len)) {
                    let mut packet = Vec::with_capacity(chunk.len().saturating_add(PREFIX_LEN));
                    packet.extend_from_slice(&[count, index]);
                    packet.extend_from_slice(chunk);
                    self.link.send(&packet).await?;
                }
            }
            Variant::V2 => {
                let chunk_len = self.write_len().max(1);
                for chunk in data.chunks(chunk_len) {
                    self.link.send(chunk).await?;
                }
            }
        }
        Ok(())
    }

    async fn purge(&mut self) -> Result<()> {
        self.received.clear();
        self.link.purge().await
    }
}
