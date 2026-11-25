//! Shared building blocks for dive computer protocols.
//!
//! This crate holds everything that more than one protocol family needs, so that it
//! exists exactly once: checksums, framing codecs, the error type, the transport traits
//! protocols are written against, and the dive data model. It is `no_std` and has no
//! platform or runtime dependencies, so it runs natively, on embedded targets and in
//! WebAssembly.
//!
//! The behaviour implemented here is specified in the
//! [Dive Computer Protocols](https://michidk.github.io/divecomputer-rs/) documentation.

#![no_std]

extern crate alloc;

pub mod checksum;
pub mod dive;
mod error;
pub mod slip;
pub mod transport;
pub mod units;

pub use error::{Error, Result};

// Compile and run the examples in the crate README as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
