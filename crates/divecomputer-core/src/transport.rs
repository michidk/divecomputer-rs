//! Transport traits that protocols are written against.
//!
//! Protocols are `async` and generic over these traits, so the same protocol code runs on
//! every backend: native serial, USB and Bluetooth stacks, and the browser's Web Serial,
//! WebUSB, WebHID and Web Bluetooth APIs. Each trait describes one kind of link; a
//! protocol asks for exactly the kind it needs. See the
//! [transport documents](https://michidk.github.io/divecomputer-rs/spec/transports/README.html).

// The traits have no `Send` bound because browser futures are not `Send`; whether to add
// `Send` variants is an open decision in ARCHITECTURE.md.
#![allow(async_fn_in_trait)]

use alloc::vec::Vec;
use core::time::Duration;

use crate::Result;

/// A byte stream: serial ports, USB-serial chips, Bluetooth classic RFCOMM, or a BLE
/// GATT link that behaves like a serial line.
pub trait ByteStream {
    /// Reads at least one byte into `buf` and returns how many bytes were read.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`](crate::Error::Timeout) if no byte arrives within `timeout`, or
    /// a transport error.
    async fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize>;

    /// Writes all of `data`.
    ///
    /// # Errors
    ///
    /// A transport error if the write fails.
    async fn write(&mut self, data: &[u8]) -> Result<()>;

    /// Discards any received data that has not been read yet.
    ///
    /// # Errors
    ///
    /// A transport error if the backend cannot purge.
    async fn purge(&mut self) -> Result<()>;
}

/// A packet link that keeps message boundaries: USB HID reports, USB bulk transfers or
/// BLE characteristic writes and notifications.
pub trait PacketLink {
    /// The largest packet [`send`](Self::send) accepts, in bytes. For BLE this is the
    /// ATT MTU minus 3.
    fn max_packet_len(&self) -> usize;

    /// Sends one packet of at most [`max_packet_len`](Self::max_packet_len) bytes.
    ///
    /// # Errors
    ///
    /// A transport error if the send fails.
    async fn send(&mut self, packet: &[u8]) -> Result<()>;

    /// Receives the next packet.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`](crate::Error::Timeout) if no packet arrives within `timeout`,
    /// or a transport error.
    async fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>>;

    /// Discards any received packets that have not been read yet.
    ///
    /// # Errors
    ///
    /// A transport error if the backend cannot purge.
    async fn purge(&mut self) -> Result<()>;
}

/// Time for protocols: native timers, the browser's `setTimeout`, or a virtual clock in
/// tests.
pub trait Clock {
    /// Waits for `duration`.
    async fn sleep(&self, duration: Duration);
}
