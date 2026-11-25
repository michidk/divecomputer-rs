//! The error type shared by transports and protocols.

use alloc::string::String;

/// A failure while talking to a dive computer or decoding what it sent.
///
/// The variants separate the cases a host reacts to differently: retry after a
/// [`Timeout`](Self::Timeout), reconnect after [`Disconnected`](Self::Disconnected), report
/// a [`Protocol`](Self::Protocol) violation or a [`Rejected`](Self::Rejected) command.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The device did not answer within the timeout.
    #[error("timed out waiting for the device")]
    Timeout,
    /// The connection to the device was closed.
    #[error("the connection to the device was closed")]
    Disconnected,
    /// The transport backend failed; the message comes from the backend.
    #[error("transport error: {0}")]
    Transport(String),
    /// The device sent data that does not follow the protocol. The message names the
    /// field or offset and what was expected.
    #[error("protocol violation: {0}")]
    Protocol(String),
    /// The device answered a command with a negative response.
    #[error("device rejected command {command:#04x} with code {code:#04x}")]
    Rejected {
        /// The command the device rejected.
        command: u8,
        /// The device's error code.
        code: u8,
    },
    /// The device or the requested operation is not supported.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
}

/// Result type with [`Error`] as the error.
pub type Result<T, E = Error> = core::result::Result<T, E>;
