//! Dive computer download protocols, one module per protocol family.
//!
//! Each module implements one family document of the
//! [Dive Computer Protocols](https://michidk.github.io/divecomputer-rs/) specification and
//! is behind a Cargo feature of the same name. Protocols are generic over the transport
//! traits in [`divecomputer_core::transport`].

#![no_std]

extern crate alloc;

#[cfg(feature = "shearwater")]
pub mod shearwater;
