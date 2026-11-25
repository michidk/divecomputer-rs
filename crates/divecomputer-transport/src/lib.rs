//! Transport backends for dive computer protocols.
//!
//! Each backend implements a transport trait from `divecomputer_core::transport`. The
//! browser backends live in `web` and exist only on `wasm32-unknown-unknown`.

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod web;
