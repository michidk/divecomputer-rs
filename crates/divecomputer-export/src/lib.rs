//! Export downloaded dives to open dive-log formats.
//!
//! Today this is [DiveJSON](https://divejson.org): [`to_divejson`] turns parsed
//! [`divecomputer_core::dive::Dive`]s into a validated [`divejson::Document`], which
//! `divejson` writes as text (`divejson::to_string_pretty`). The crate is `no_std` with
//! `alloc` and reads only the core dive model.

#![no_std]

extern crate alloc;

mod divejson_writer;

pub use divejson_writer::{DIVE_NAMESPACE, to_divejson};

/// Why an export failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The dive at `index` has no start date-time, which every DiveJSON dive needs.
    #[error("dive {index}: no start date-time")]
    MissingStart {
        /// Position of the dive in the input.
        index: usize,
    },
    /// The dive at `index` has a start DiveJSON cannot write (a year outside 0000–9999).
    #[error("dive {index}: {source}")]
    Start {
        /// Position of the dive in the input.
        index: usize,
        /// What was wrong with it.
        source: divejson::ValueError,
    },
    /// The document breaks a DiveJSON rule, for example `exported_at` without a UTC
    /// offset or two dives with the same identity.
    #[error(transparent)]
    DiveJson(#[from] divejson::Error),
}

/// The result type of this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;

// Compile and run the examples in the crate README as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
