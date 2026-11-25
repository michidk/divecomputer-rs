//! Downloads from a simulated Shearwater device that implements the protocol as the spec
//! describes it, over a byte stream (RFCOMM) and over BLE.
#![cfg(feature = "shearwater")]
// Test code: a simulated device may panic, index and do plain arithmetic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::unused_async_trait_impl
)]

use std::{
    collections::VecDeque,
    future::Future,
    ops::ControlFlow,
    pin::pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

use divecomputer_core::{
    Error, Result,
    dive::RawDive,
    slip::{self, SlipDecoder},
    transport::{ByteStream, Clock, PacketLink},
};
use divecomputer_protocol::shearwater::{BleStream, Model, Shearwater, Variant};

const DIVE_BASE: u32 = 0x8000_0000;
const DIVE_SPACING: u32 = 0x1000;
const MAX_BLOCK: usize = 146;

/// Runs a future whose transports never wait.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("simulated transports never wait"),
    }
}

struct NoDelay;

impl Clock for NoDelay {
    async fn sleep(&self, _duration: Duration) {}
}

/// A Shearwater device holding `dives` (newest first) and `deleted` manifest entries
/// placed after the first dive.
struct Device {
    variant: Variant,
    dives: Vec<Vec<u8>>,
    deleted: usize,
    reject_item: Option<u16>,
    decoder: SlipDecoder,
    output: VecDeque<u8>,
    blocks: VecDeque<Vec<u8>>,
    manifest_reads: usize,
    requests: Vec<Vec<u8>>,
    closed: bool,
}

impl Device {
    fn new(variant: Variant, dive_count: u32) -> Self {
        Self {
            variant,
            dives: (0..dive_count).rev().map(dive).collect(),
            deleted: 0,
            reject_item: None,
            decoder: SlipDecoder::new(64),
            output: VecDeque::new(),
            blocks: VecDeque::new(),
            manifest_reads: 0,
            requests: Vec::new(),
            closed: false,
        }
    }

    fn manifest(&self) -> Vec<Vec<u8>> {
        let mut entries: Vec<Vec<u8>> = self
            .dives
            .iter()
            .enumerate()
            .map(|(index, data)| {
                let mut entry = vec![0; 32];
                entry[..2].copy_from_slice(&[0xA5, 0xC4]);
                entry[4..8].copy_from_slice(&data[12..16]);
                let offset = DIVE_SPACING * u32::try_from(index).unwrap();
                entry[0x14..0x18].copy_from_slice(&offset.to_be_bytes());
                entry
            })
            .collect();
        for _ in 0..self.deleted {
            let mut entry = vec![0; 32];
            entry[..2].copy_from_slice(&[0x5A, 0x23]);
            entries.insert(1, entry);
        }
        entries
    }

    fn handle(&mut self, payload: &[u8]) {
        self.requests.push(payload.to_vec());
        let response = match payload {
            [0x22, hi, lo] if self.reject_item == Some(u16::from_be_bytes([*hi, *lo])) => {
                vec![0x7F, 0x22, 0x31]
            }
            [0x22, hi, lo] => {
                let value: &[u8] = match u16::from_be_bytes([*hi, *lo]) {
                    0x8010 => b"12AB34CD",
                    0x8011 => b"V92",
                    0x8060 => &[11],
                    0x8021 => &[0x00, 0x80, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04],
                    other => panic!("unexpected data item {other:#06x}"),
                };
                [&[0x62, *hi, *lo], value].concat()
            }
            [0x2E, 0x90, 0x20, 0x00] => {
                self.closed = true;
                return;
            }
            [0x35, flags, 0x34, a0, a1, a2, a3, 0xFF, 0xFF, 0xFF] => {
                assert_eq!(*flags, 0x10, "dives must be requested compressed");
                let offset = u32::from_be_bytes([*a0, *a1, *a2, *a3]) - DIVE_BASE;
                let index = usize::try_from(offset / DIVE_SPACING).unwrap();
                self.blocks = compress(&self.dives[index]).into();
                vec![0x75, 0x10, u8::try_from(MAX_BLOCK).unwrap()]
            }
            [0x35, 0x00, 0x34, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00] => {
                let mut page: Vec<u8> = self
                    .manifest()
                    .into_iter()
                    .skip(48 * self.manifest_reads)
                    .take(48)
                    .flatten()
                    .collect();
                page.resize(0x600, 0);
                self.manifest_reads += 1;
                self.blocks = page.chunks(MAX_BLOCK).map(<[u8]>::to_vec).collect();
                vec![0x75, 0x10, u8::try_from(MAX_BLOCK).unwrap()]
            }
            [0x36, seq, rest @ ..] => {
                let expected_rest: &[u8] = match self.variant {
                    Variant::V1 => &[],
                    Variant::V2 => &[0x00],
                };
                assert_eq!(rest, expected_rest);
                let block = self.blocks.pop_front().expect("no block left");
                [&[0x76, *seq], block.as_slice()].concat()
            }
            [0x37] => vec![0x77, 0x00],
            other => panic!("unexpected request {other:02X?}"),
        };
        let mut packet = match self.variant {
            Variant::V1 => vec![0x01, 0xFF, u8::try_from(response.len() + 1).unwrap(), 0x00],
            Variant::V2 => {
                let len = u16::try_from(response.len()).unwrap().to_be_bytes();
                vec![0x01, 0xFF, 0x00, len[0], len[1]]
            }
        };
        packet.extend(response);
        let mut encoded = Vec::new();
        slip::encode(&packet, &mut encoded);
        self.output.extend(encoded);
    }
}

impl ByteStream for Device {
    async fn read(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize> {
        if self.output.is_empty() {
            return Err(Error::Timeout);
        }
        let count = buf.len().min(self.output.len());
        for (slot, byte) in buf.iter_mut().zip(self.output.drain(..count)) {
            *slot = byte;
        }
        Ok(count)
    }

    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let header_len = match self.variant {
            Variant::V1 => 4,
            Variant::V2 => 5,
        };
        for &byte in data {
            if let Some(frame) = self.decoder.push(byte)? {
                assert_eq!(frame[..2], [0xFF, 0x01]);
                self.handle(&frame[header_len..]);
            }
        }
        Ok(())
    }

    async fn purge(&mut self) -> Result<()> {
        self.output.clear();
        Ok(())
    }
}

/// The device behind a BLE characteristic with the default 20-byte packets.
struct BleDevice {
    device: Device,
    packets: Vec<Vec<u8>>,
}

impl PacketLink for BleDevice {
    fn max_packet_len(&self) -> usize {
        20
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        assert!(packet.len() <= 20);
        self.packets.push(packet.to_vec());
        let data = match self.device.variant {
            Variant::V1 => &packet[2..],
            Variant::V2 => packet,
        };
        self.device.write(data).await
    }

    async fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let (prefix, room): (&[u8], usize) = match self.device.variant {
            Variant::V1 => (&[0x01, 0x01], 18),
            Variant::V2 => (&[], 20),
        };
        let mut data = vec![0; room];
        let count = self.device.read(&mut data, timeout).await?;
        Ok([prefix, &data[..count]].concat())
    }

    async fn purge(&mut self) -> Result<()> {
        self.device.purge().await
    }
}

/// Dive `number`: 300 bytes with runs of zeros and its start time at offset 12.
fn dive(number: u32) -> Vec<u8> {
    let mut data: Vec<u8> = (0..300u32)
        .map(|i| {
            if i % 7 < 3 {
                0
            } else {
                (i * 31 + number) as u8
            }
        })
        .collect();
    data[12..16].copy_from_slice(&(1_700_000_000 + number).to_be_bytes());
    data
}

/// Compresses `data` the way the device does: XOR delta, then 9-bit zero-run codes
/// packed into blocks of whole codes.
fn compress(data: &[u8]) -> Vec<Vec<u8>> {
    let delta: Vec<u8> = (0..data.len())
        .map(|i| {
            if i < 32 {
                data[i]
            } else {
                data[i] ^ data[i - 32]
            }
        })
        .collect();
    let mut codes = Vec::new();
    let mut zeros = 0u16;
    for &byte in &delta {
        if byte == 0 && zeros < 255 {
            zeros += 1;
            continue;
        }
        if zeros > 0 {
            codes.push(zeros);
            zeros = 0;
        }
        if byte == 0 {
            zeros = 1;
        } else {
            codes.push(0x100 | u16::from(byte));
        }
    }
    if zeros > 0 {
        codes.push(zeros);
    }
    codes.push(0);
    codes.resize(codes.len().div_ceil(8) * 8, 0);

    let packed: Vec<u8> = codes
        .chunks(8)
        .flat_map(|group| {
            let bits = group
                .iter()
                .fold(0u128, |acc, &code| (acc << 9) | u128::from(code));
            bits.to_be_bytes()[7..].to_vec()
        })
        .collect();
    packed.chunks(144).map(<[u8]>::to_vec).collect()
}

fn download(
    device: Device,
    fingerprint: Option<&[u8]>,
    limit: usize,
) -> (Result<Vec<RawDive>>, Device) {
    block_on(async {
        let variant = device.variant;
        let mut shearwater = Shearwater::connect(device, NoDelay, variant).await.unwrap();
        let mut dives = Vec::new();
        let result = async {
            let identity = shearwater.identify().await?;
            assert_eq!(identity.serial(), 0x12AB_34CD);
            assert_eq!(identity.firmware(), "V92");
            assert_eq!(identity.model(), Model::Perdix2);
            shearwater
                .download(&identity, fingerprint, |dive| {
                    dives.push(dive);
                    if dives.len() == limit {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                })
                .await
        }
        .await;
        let device = shearwater.close().await.unwrap();
        (result.map(|()| dives), device)
    })
}

fn fingerprints(dives: &[RawDive]) -> Vec<Vec<u8>> {
    dives
        .iter()
        .map(|dive| dive.fingerprint().to_vec())
        .collect()
}

#[test]
fn downloads_every_dive_newest_first() {
    let (dives, device) = download(Device::new(Variant::V1, 3), None, usize::MAX);
    let dives = dives.unwrap();
    assert_eq!(dives.len(), 3);
    for (raw, expected) in dives.iter().zip(&device.dives) {
        assert_eq!(raw.data(), expected);
        assert_eq!(raw.fingerprint(), &expected[12..16]);
    }
    assert!(device.closed);
}

#[test]
fn stops_at_known_fingerprint() {
    let known = dive(1)[12..16].to_vec();
    let (dives, _) = download(Device::new(Variant::V1, 4), Some(&known), usize::MAX);
    assert_eq!(
        fingerprints(&dives.unwrap()),
        [dive(3)[12..16].to_vec(), dive(2)[12..16].to_vec()]
    );
}

#[test]
fn stops_when_consumer_breaks() {
    let (dives, device) = download(Device::new(Variant::V1, 5), None, 2);
    assert_eq!(dives.unwrap().len(), 2);
    let dive_uploads = device
        .requests
        .iter()
        .filter(|request| request.starts_with(&[0x35, 0x10]))
        .count();
    assert_eq!(dive_uploads, 2);
}

#[test]
fn pages_through_a_full_manifest() {
    let mut device = Device::new(Variant::V1, 50);
    device.deleted = 1;
    let (dives, device) = download(device, None, usize::MAX);
    assert_eq!(dives.unwrap().len(), 50);
    assert_eq!(device.manifest_reads, 2);
}

#[test]
fn reports_rejected_commands() {
    let mut device = Device::new(Variant::V1, 1);
    device.reject_item = Some(0x8060);
    let (dives, _) = download(device, None, usize::MAX);
    assert_eq!(
        dives,
        Err(Error::Rejected {
            command: 0x22,
            code: 0x31
        })
    );
}

fn download_over_ble(variant: Variant) -> BleDevice {
    block_on(async {
        let link = BleDevice {
            device: Device::new(variant, 2),
            packets: Vec::new(),
        };
        let mut shearwater = Shearwater::connect(BleStream::new(link, variant), NoDelay, variant)
            .await
            .unwrap();
        let identity = shearwater.identify().await.unwrap();
        let mut count = 0;
        shearwater
            .download(&identity, None, |_| {
                count += 1;
                ControlFlow::Continue(())
            })
            .await
            .unwrap();
        assert_eq!(count, 2);
        shearwater.close().await.unwrap().into_inner()
    })
}

#[test]
fn downloads_over_ble_with_segmentation_prefix() {
    let link = download_over_ble(Variant::V1);
    // Worked examples from the spec: serial number read and session end.
    assert_eq!(
        link.packets.first().unwrap(),
        &[0x01, 0x00, 0xFF, 0x01, 0x04, 0x00, 0x22, 0x80, 0x10, 0xC0]
    );
    assert_eq!(
        link.packets.last().unwrap(),
        &[
            0x01, 0x00, 0xFF, 0x01, 0x05, 0x00, 0x2E, 0x90, 0x20, 0x00, 0xC0
        ]
    );
    assert!(link.device.closed);
}

#[test]
fn downloads_over_ble_with_variant_2() {
    let link = download_over_ble(Variant::V2);
    assert_eq!(
        link.packets.first().unwrap(),
        &[0xFF, 0x01, 0x00, 0x00, 0x03, 0x22, 0x80, 0x10, 0xC0]
    );
    assert!(link.device.closed);
}
