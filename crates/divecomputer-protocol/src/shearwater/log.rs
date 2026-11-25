//! Shearwater dive log parser
//! ([spec](https://michidk.github.io/divecomputer-rs/spec/families/shearwater-log-format.html)).
//!
//! Decodes both encodings of a downloaded dive: the 128-byte block format (Predator and
//! base `0xC0000000`) and the 32-byte-record native format (PNF, base `0x80000000`).
//! Where the two differ only in where a field lives or how it is scaled, the difference
//! is a [`Layout`] table, not separate code.

use alloc::{format, string::String, vec, vec::Vec};
use core::time::Duration;

use divecomputer_core::{
    Error, Result,
    dive::{
        Deco, DecoModel, Dive, DiveMode, Event, EventKind, GasMix, GasUsage, Location, Salinity,
        Sample, Tank, TankPressure,
    },
    units::{Depth, GasFraction, Pressure, Temperature},
};

use super::Model;

/// Native record size (spec: "Native format").
const RECORD_SIZE: usize = 32;
/// Block size of the block format's opening, closing and final blocks (spec: "Block format").
const BLOCK_SIZE: usize = 128;
/// Block-format opening marker.
const MARKER_OPENING: [u8; 2] = [0xFF, 0xFF];
/// Block-format closing marker.
const MARKER_CLOSING: [u8; 2] = [0xFF, 0xFE];
/// Block-format final marker.
const MARKER_FINAL: [u8; 2] = [0xFF, 0xFD];
/// Predator sample size in the block format.
const PREDATOR_SAMPLE_SIZE: usize = 16;
/// Petrel-family sample size in the block format.
const PETREL_SAMPLE_SIZE: usize = 32;
/// Sample fields every sample has (the Predator's 16 bytes).
const MIN_SAMPLE_FIELDS: usize = 16;

/// Native record types (spec: "Record types").
const RECORD_DIVE_SAMPLE: u8 = 0x01;
/// Freedive record: four 8-byte sub-samples.
const RECORD_FREEDIVE: u8 = 0x02;
/// Avelo sample: dive-sample layout, open circuit, transmitter 1 only.
const RECORD_AVELO_SAMPLE: u8 = 0x03;
/// Opening records 0–9 are `10`–`19`.
const RECORD_OPENING: u8 = 0x10;
/// Closing records 0–9 are `20`–`29`.
const RECORD_CLOSING: u8 = 0x20;
/// Number of opening (and closing) record slots.
const HEADER_RECORDS: u8 = 10;
/// Opening and closing records 0–4 are mandatory.
const REQUIRED_RECORDS: u16 = 0b1_1111;
/// Info event.
const RECORD_INFO_EVENT: u8 = 0x30;
/// Extended dive sample: transmitters 3 and 4, HP-CCR pressures.
const RECORD_EXTENDED_SAMPLE: u8 = 0xE1;
/// Final record.
const RECORD_FINAL: u8 = 0xFF;
/// Freedive sub-sample size.
const FREEDIVE_SUBSAMPLE_SIZE: usize = 8;

/// Log versions below this are unreliable and treated as this (spec: "Log versions").
const MIN_LOG_VERSION: u8 = 6;
/// Sample interval when opening 5 is absent (spec: "Dive sample", Time).
const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);
/// Water density meaning fresh water, kg/m³.
const DENSITY_FRESH: u16 = 1000;
/// Standard gravity, m/s², for freedive depth from pressure.
const GRAVITY: f64 = 9.806_65;
/// Factory-default O₂ cell calibration; all masked cells at this value means uncalibrated.
const DEFAULT_CALIBRATION: u16 = 2100;
/// Predator cell calibration multiplier.
const PREDATOR_CALIBRATION_FACTOR: f64 = 2.2;
/// Upper bound of the gas list (spec: "Gas list").
const MAX_GAS_MIXES: usize = 20;
/// Number of fixed gas slots; slots 6–10 are diluents.
const GAS_SLOTS: usize = 10;
/// First diluent slot (zero-based).
const FIRST_DILUENT_SLOT: usize = 5;
/// Info-event code of a diver bookmark.
const EVENT_BOOKMARK: u8 = 38;
/// "No heading" in a bookmark event.
const NO_HEADING: u32 = 0xFFFF_FFFF;
/// Transmitter pressures at or above this are status codes (spec: "Air integration").
const PRESSURE_STATUS_CODES: u16 = 0xFFF0;
/// Pressure bits of a transmitter reading; the top four bits are battery state.
const PRESSURE_MASK: u16 = 0x0FFF;
/// Gas-time-remaining values at or above this are status codes.
const GTR_STATUS_CODES: u8 = 0xF0;
/// GF99 "not applicable".
const GF99_NONE: u8 = 0xFF;
/// GNSS fix states with a usable position (2D, 3D).
const GNSS_FIX: [u8; 2] = [2, 3];

/// Sample status byte bits (spec: "Status byte").
const STATUS_EXTERNAL_PPO2: u8 = 0x02;
/// High setpoint selected (Predator).
const STATUS_HIGH_SETPOINT: u8 = 0x04;
/// Semi-closed circuit, when open circuit is clear.
const STATUS_SEMI_CLOSED: u8 = 0x08;
/// Open circuit; clear means closed-circuit loop.
const STATUS_OPEN_CIRCUIT: u8 = 0x10;
/// Bail-out from CCR; ppO₂ values not meaningful.
const STATUS_BAILOUT: u8 = 0x20;

/// Absolute offset of `offset` in native opening/closing record `record`, in the
/// concatenated header buffer.
#[allow(
    clippy::arithmetic_side_effects,
    reason = "used with record numbers below 10 and offsets below 32"
)]
const fn at(record: usize, offset: usize) -> usize {
    record * RECORD_SIZE + offset
}

/// Where the fields shared by both encodings live (spec: "Opening block", "Opening and
/// closing records"). Opening offsets index the header buffer: the opening block, or the
/// native opening records concatenated by number. Closing offsets index the closing
/// block or closing record 0.
struct Layout {
    dive_number: usize,
    gf_low: usize,
    gf_high: usize,
    units: usize,
    start: usize,
    oxygen: [usize; GAS_SLOTS],
    helium: [usize; GAS_SLOTS],
    atmospheric: usize,
    deco_model: usize,
    vpm_conservatism: usize,
    density: usize,
    gfs_gradient_factor: usize,
    calibration_mask: usize,
    calibration: [usize; 3],
    dive_mode: usize,
    log_version: usize,
    /// Predator low and high setpoints.
    setpoints: Option<[usize; 2]>,
    max_depth: usize,
    /// Depth units per raw max-depth step.
    max_depth_scale: f64,
    dive_time: usize,
    /// `u16be` minutes (block) or `u24be` seconds (native).
    dive_time_seconds: bool,
}

#[allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "only evaluated at compile time for the layout tables"
)]
const fn consecutive<const N: usize>(first: usize, step: usize) -> [usize; N] {
    let mut offsets = [0; N];
    let mut i = 0;
    while i < N {
        offsets[i] = first + i * step;
        i += 1;
    }
    offsets
}

/// Block format layout.
const BLOCK: Layout = Layout {
    dive_number: 0x02,
    gf_low: 0x04,
    gf_high: 0x05,
    units: 0x08,
    start: 0x0C,
    oxygen: consecutive(0x14, 1),
    helium: consecutive(0x1E, 1),
    atmospheric: 0x2F,
    deco_model: 0x43,
    vpm_conservatism: 0x44,
    density: 0x53,
    gfs_gradient_factor: 0x55,
    calibration_mask: 0x56,
    calibration: consecutive(0x57, 2),
    dive_mode: 0x70,
    log_version: 0x7F,
    setpoints: Some([0x11, 0x12]),
    max_depth: 0x04,
    max_depth_scale: 1.0,
    dive_time: 0x06,
    dive_time_seconds: false,
};

/// Native format layout.
const NATIVE: Layout = Layout {
    dive_number: at(0, 2),
    gf_low: at(0, 4),
    gf_high: at(0, 5),
    units: at(0, 8),
    start: at(0, 12),
    oxygen: consecutive(at(0, 20), 1),
    helium: [
        at(0, 30),
        at(0, 31),
        at(1, 1),
        at(1, 2),
        at(1, 3),
        at(1, 4),
        at(1, 5),
        at(1, 6),
        at(1, 7),
        at(1, 8),
    ],
    atmospheric: at(1, 16),
    deco_model: at(2, 18),
    vpm_conservatism: at(2, 19),
    density: at(3, 3),
    gfs_gradient_factor: at(3, 5),
    calibration_mask: at(3, 6),
    calibration: consecutive(at(3, 7), 2),
    dive_mode: at(4, 1),
    log_version: at(4, 16),
    setpoints: None,
    max_depth: 4,
    max_depth_scale: 0.1,
    dive_time: 6,
    dive_time_seconds: true,
};

/// Native-only header fields (spec: "Opening 4" to "Opening 9").
const GAS_ENABLED_MASK: usize = at(4, 17);
/// Air-integration mode.
const AI_MODE: usize = at(4, 28);
/// Sample interval, ms.
const SAMPLE_INTERVAL: usize = at(5, 23);
/// UTC offset, minutes.
const UTC_OFFSET: usize = at(5, 26);
/// Daylight-saving offset, hours.
const DST_OFFSET: usize = at(5, 30);
/// GNSS fix status at dive start.
const GNSS_STATUS: usize = at(9, 16);
/// Latitude, 1e-5 degree.
const LATITUDE: usize = at(9, 21);
/// Longitude, 1e-5 degree.
const LONGITUDE: usize = at(9, 25);

/// Transmitter slots 1–4 and the HP-CCR pressures (spec: "Air integration").
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Transmitter(usize),
    HpDiluent,
    HpOxygen,
}

/// Per-transmitter header fields: (serial, enabled, name), log version for each.
struct TransmitterLayout {
    serial: usize,
    serial_version: u8,
    enabled: usize,
    name: usize,
}

/// Transmitters 1–4 (spec: "Opening 5" to "Opening 7").
const TRANSMITTERS: [TransmitterLayout; 4] = [
    TransmitterLayout {
        serial: at(5, 1),
        serial_version: 9,
        enabled: at(6, 19),
        name: at(6, 20),
    },
    TransmitterLayout {
        serial: at(5, 10),
        serial_version: 9,
        enabled: at(6, 22),
        name: at(6, 23),
    },
    TransmitterLayout {
        serial: at(6, 25),
        serial_version: 13,
        enabled: at(7, 1),
        name: at(7, 2),
    },
    TransmitterLayout {
        serial: at(7, 4),
        serial_version: 13,
        enabled: at(7, 11),
        name: at(7, 12),
    },
];
/// Log version that added per-tank enable flags and names.
const TANK_NAMES_VERSION: u8 = 13;

/// Sample offsets of transmitter 1 and 2 pressures.
const SAMPLE_T1_PRESSURE: usize = 27;
/// Transmitter 2 (or HP-CCR O₂) pressure.
const SAMPLE_T2_PRESSURE: usize = 19;

/// Big-endian field access that names the absolute offset on failure.
#[derive(Clone, Copy)]
struct Fields<'a> {
    bytes: &'a [u8],
    /// Offset of `bytes` in the whole log, for error messages.
    base: usize,
}

impl<'a> Fields<'a> {
    const fn new(bytes: &'a [u8], base: usize) -> Self {
        Self { bytes, base }
    }

    fn get<const N: usize>(self, offset: usize) -> Option<[u8; N]> {
        self.bytes.get(offset..)?.first_chunk::<N>().copied()
    }

    fn opt_u8(self, offset: usize) -> Option<u8> {
        self.bytes.get(offset).copied()
    }

    fn opt_u16(self, offset: usize) -> Option<u16> {
        self.get(offset).map(u16::from_be_bytes)
    }

    fn require<const N: usize>(self, offset: usize, field: &str) -> Result<[u8; N]> {
        self.get(offset).ok_or_else(|| {
            Error::Protocol(format!(
                "Shearwater log: {field} at byte {} is past the end",
                self.base.saturating_add(offset)
            ))
        })
    }

    fn u8(self, offset: usize, field: &str) -> Result<u8> {
        self.require::<1>(offset, field).map(|[b]| b)
    }

    fn u16(self, offset: usize, field: &str) -> Result<u16> {
        self.require(offset, field).map(u16::from_be_bytes)
    }

    fn u32(self, offset: usize, field: &str) -> Result<u32> {
        self.require(offset, field).map(u32::from_be_bytes)
    }

    fn i32(self, offset: usize, field: &str) -> Result<i32> {
        self.require(offset, field).map(i32::from_be_bytes)
    }
}

/// The two encodings' sample streams.
enum Body<'a> {
    /// Block format: fixed-size samples from byte 128.
    Block { samples: &'a [u8], size: usize },
    /// Native format: all records.
    Native(&'a [u8]),
}

/// A log split into its parts.
struct Log<'a> {
    layout: &'static Layout,
    /// Opening block, or native opening records 0–9 concatenated by number.
    header: Vec<u8>,
    /// Bit *n* set: native opening record *n* present (block: 0–4).
    present: u16,
    closing: Fields<'a>,
    final_record: Option<Fields<'a>>,
    body: Body<'a>,
}

/// Parses a downloaded Shearwater dive ([`RawDive::data`]) into a [`Dive`].
///
/// Handles the block format (opening block `FF FF`) and the native format (records,
/// normally starting `10`). Model-specific details (Predator sample size and cell
/// calibration, Teric time zone and transmitter serials) are taken from the log's final
/// record or block; a block-format log without one is a Predator log.
///
/// Sample *k* (counting from 1) is placed at *k* × the sample interval; see the spec's
/// open question 1.
///
/// # Errors
///
/// [`Error::Protocol`] naming the byte offset or record when the log is truncated, a
/// marker is wrong, or a mandatory record is missing.
///
/// [`RawDive::data`]: divecomputer_core::dive::RawDive::data
pub fn parse(data: &[u8]) -> Result<Dive> {
    let log = if data.starts_with(&MARKER_OPENING) {
        Log::block(data)?
    } else {
        Log::native(data)?
    };
    log.dive()
}

impl<'a> Log<'a> {
    fn block(data: &'a [u8]) -> Result<Self> {
        let (header, rest) = data.split_at_checked(BLOCK_SIZE).ok_or_else(|| {
            Error::Protocol(format!(
                "Shearwater log: {} bytes is shorter than an opening block",
                data.len()
            ))
        })?;
        let (rest, final_record) = match rest.split_last_chunk::<BLOCK_SIZE>() {
            Some((rest, last)) if last.starts_with(&MARKER_FINAL) => (rest, Some(&last[..])),
            _ => (rest, None),
        };
        let (samples, closing) = rest
            .split_last_chunk::<BLOCK_SIZE>()
            .filter(|(_, closing)| closing.starts_with(&MARKER_CLOSING))
            .ok_or_else(|| Error::Protocol("Shearwater log: no closing block (FF FE)".into()))?;
        let final_record =
            final_record.map(|bytes| Fields::new(bytes, data.len().saturating_sub(BLOCK_SIZE)));
        let model = final_record
            .and_then(|f| f.opt_u8(13))
            .map(Model::from_code);
        // A final block is always present in Petrel-family logs (spec: "Block format").
        let size = if matches!(model, None | Some(Model::Predator)) {
            PREDATOR_SAMPLE_SIZE
        } else {
            PETREL_SAMPLE_SIZE
        };
        if !samples.chunks_exact(size).remainder().is_empty() {
            return Err(Error::Protocol(format!(
                "Shearwater log: {} sample bytes are not a multiple of {size}",
                samples.len()
            )));
        }
        Ok(Self {
            layout: &BLOCK,
            header: header.to_vec(),
            present: REQUIRED_RECORDS,
            closing: Fields::new(closing, BLOCK_SIZE.saturating_add(samples.len())),
            final_record,
            body: Body::Block { samples, size },
        })
    }

    fn native(data: &'a [u8]) -> Result<Self> {
        let records = data.chunks_exact(RECORD_SIZE);
        if !records.remainder().is_empty() {
            return Err(Error::Protocol(format!(
                "Shearwater log: {} bytes is not a whole number of 32-byte records",
                data.len()
            )));
        }
        let mut header = vec![0; at(usize::from(HEADER_RECORDS), 0)];
        let mut present = 0_u16;
        let mut closing_present = 0_u16;
        let mut closing = None;
        let mut final_record = None;
        for (base, record) in (0..).step_by(RECORD_SIZE).zip(records) {
            let Some(&kind) = record.first() else {
                continue;
            };
            if let Some(n) = kind
                .checked_sub(RECORD_OPENING)
                .filter(|n| *n < HEADER_RECORDS)
            {
                let start = at(usize::from(n), 0);
                if let Some(slot) = header.get_mut(start..start.saturating_add(RECORD_SIZE)) {
                    slot.copy_from_slice(record);
                }
                present |= 1 << n;
            } else if let Some(n) = kind
                .checked_sub(RECORD_CLOSING)
                .filter(|n| *n < HEADER_RECORDS)
            {
                closing_present |= 1 << n;
                if n == 0 {
                    closing = Some(Fields::new(record, base));
                }
            } else if kind == RECORD_FINAL {
                final_record = Some(Fields::new(record, base));
            }
        }
        let missing = |which| {
            Error::Protocol(format!(
                "Shearwater log: {which} records 0-4 are not all present"
            ))
        };
        if present & REQUIRED_RECORDS != REQUIRED_RECORDS {
            return Err(missing("opening"));
        }
        let closing = closing
            .filter(|_| closing_present & REQUIRED_RECORDS == REQUIRED_RECORDS)
            .ok_or_else(|| missing("closing"))?;
        Ok(Self {
            layout: &NATIVE,
            header,
            present,
            closing,
            final_record,
            body: Body::Native(data),
        })
    }

    fn has(&self, record: u8) -> bool {
        self.present & (1 << record) != 0
    }

    const fn is_native(&self) -> bool {
        matches!(self.body, Body::Native(_))
    }

    fn header(&self) -> Fields<'_> {
        Fields::new(&self.header, 0)
    }

    fn dive(&self) -> Result<Dive> {
        let header = self.header();
        let layout = self.layout;
        let model = self
            .final_record
            .map(|f| f.u8(13, "model code"))
            .transpose()?
            .map(Model::from_code)
            // Petrel-family block logs always have a final block (spec: "Block format").
            .or_else(|| (!self.is_native()).then_some(Model::Predator));
        let version = header
            .u8(layout.log_version, "log version")?
            .max(MIN_LOG_VERSION);
        let units = Units {
            imperial: header.u8(layout.units, "units")? == 1,
        };
        let start = header.u32(layout.start, "dive start")?;

        let mut dive = Dive::default();
        dive.deco_model = self.deco_model()?;
        let context = self.sample_context(model, version, units, dive.deco_model)?;
        let mut walk = Walk::new(&context, self.interval(version)?);
        self.walk(&mut walk, start)?;
        dive.mode = self.mode(&walk, version)?;
        let rebreather = matches!(
            dive.mode,
            Some(DiveMode::ClosedCircuit | DiveMode::SemiClosed)
        );
        if dive.mode != Some(DiveMode::Freedive) {
            self.gases(&mut dive, &walk.switches, rebreather)?;
        }
        dive.tanks = self.tanks(&walk, model, version, rebreather);
        dive.events.extend(walk.events);
        dive.events.sort_by_key(|e| e.time);
        dive.samples = walk.samples;

        dive.start = jiff::Timestamp::from_second(i64::from(start))
            .ok()
            .map(|t| jiff::tz::Offset::UTC.to_datetime(t));
        if self.is_native() && model == Some(Model::Teric) && version >= 9 && self.has(5) {
            let offset_minutes = header.i32(UTC_OFFSET, "UTC offset")?;
            let dst_hours = header.u8(DST_OFFSET, "DST offset")?;
            dive.utc_offset = offset_minutes
                .checked_mul(60)
                .and_then(|s| s.checked_add(i32::from(dst_hours).checked_mul(3600)?));
        }
        dive.number = Some(u32::from(header.u16(layout.dive_number, "dive number")?));
        dive.duration = Some(if layout.dive_time_seconds {
            let [high, mid, low] = self.closing.require(layout.dive_time, "dive time")?;
            Duration::from_secs(u64::from(u32::from_be_bytes([0, high, mid, low])))
        } else {
            minutes(u32::from(self.closing.u16(layout.dive_time, "dive time")?))
        });
        let max_depth = self.closing.u16(layout.max_depth, "maximum depth")?;
        dive.max_depth = Some(units.depth(f64::from(max_depth) * layout.max_depth_scale));
        let density = header.u16(layout.density, "water density")?;
        if density != 0 {
            dive.water_density = Some(f64::from(density));
            dive.salinity = Some(if density == DENSITY_FRESH {
                Salinity::Fresh
            } else {
                Salinity::Salt
            });
        }
        let atmospheric = header.u16(layout.atmospheric, "atmospheric pressure")?;
        if atmospheric != 0 {
            dive.surface_pressure = Some(Pressure::from_millibar(f64::from(atmospheric)));
        }
        if self.is_native()
            && version >= 17
            && self.has(9)
            && GNSS_FIX.contains(&header.u8(GNSS_STATUS, "GNSS status")?)
        {
            dive.location = Some(Location::new(
                f64::from(header.i32(LATITUDE, "latitude")?) / 100_000.0,
                f64::from(header.i32(LONGITUDE, "longitude")?) / 100_000.0,
            ));
        }
        dive.device.brand = Some("Shearwater".into());
        dive.device.model = model.and_then(Model::name).map(Into::into);
        if let Some(f) = self.final_record {
            dive.device.serial = Some(format!("{:08X}", f.u32(2, "serial number")?));
            dive.device.firmware = bcd(f.u8(10, "firmware version")?);
        }
        Ok(dive)
    }

    /// The sample interval (spec: "Dive sample", Time).
    fn interval(&self, version: u8) -> Result<Duration> {
        if !(self.is_native() && version >= 9 && self.has(5)) {
            return Ok(DEFAULT_INTERVAL);
        }
        match self.header().u16(SAMPLE_INTERVAL, "sample interval")? {
            0 => Err(Error::Protocol(
                "Shearwater log: sample interval is 0".into(),
            )),
            ms => Ok(Duration::from_millis(u64::from(ms))),
        }
    }

    /// Spec: "Decompression model".
    fn deco_model(&self) -> Result<Option<DecoModel>> {
        let (header, layout) = (self.header(), self.layout);
        Ok(match header.u8(layout.deco_model, "deco model")? {
            0 => Some(DecoModel::Buhlmann {
                gf_low: header.u8(layout.gf_low, "GF low")?,
                gf_high: header.u8(layout.gf_high, "GF high")?,
            }),
            kind @ (1 | 2) => Some(DecoModel::Vpm {
                conservatism: header.u8(layout.vpm_conservatism, "VPM-B conservatism")?,
                gfs_gradient_factor: if kind == 2 {
                    Some(header.u8(layout.gfs_gradient_factor, "GFS gradient factor")?)
                } else {
                    None
                },
            }),
            3 => Some(DecoModel::Dciem),
            _ => None,
        })
    }

    /// Header values the sample decoders need.
    fn sample_context(
        &self,
        model: Option<Model>,
        version: u8,
        units: Units,
        deco_model: Option<DecoModel>,
    ) -> Result<SampleContext> {
        let (header, layout) = (self.header(), self.layout);
        let ai_mode = (self.is_native() && version >= 7)
            .then(|| header.u8(AI_MODE, "air-integration mode"))
            .transpose()?;
        let enabled = |t: usize| {
            version >= TANK_NAMES_VERSION
                && TRANSMITTERS
                    .get(t)
                    .and_then(|t| header.opt_u8(t.enabled))
                    .is_some_and(|e| e != 0)
        };
        let t1 = (Slot::Transmitter(0), SAMPLE_T1_PRESSURE);
        let t2 = (Slot::Transmitter(1), SAMPLE_T2_PRESSURE);
        // Pressure fields of a dive sample by air-integration mode (spec: "Air integration").
        let sample_pressures = match ai_mode {
            None if version >= 7 => vec![t1, t2],
            Some(1) => vec![t1],
            Some(2) => vec![t2],
            Some(3) => vec![t1, t2],
            Some(4) => vec![
                (Slot::HpDiluent, SAMPLE_T1_PRESSURE),
                (Slot::HpOxygen, SAMPLE_T2_PRESSURE),
            ],
            // shortcut: mode 6 ("with GNSS", uncertain) is read like mode 5.
            Some(5 | 6) => [(0, t1), (1, t2)]
                .into_iter()
                .filter(|&(t, _)| enabled(t))
                .map(|(_, field)| field)
                .collect(),
            _ => Vec::new(),
        };
        let extended_pressures = if matches!(ai_mode, Some(5 | 6)) {
            [(2, 1), (3, 3)]
                .into_iter()
                .filter(|&(t, _)| enabled(t))
                .collect()
        } else {
            Vec::new()
        };

        let mut cells = [None; 3];
        let mask = header.u8(layout.calibration_mask, "calibration mask")?;
        let mut all_default = true;
        let factor = if model == Some(Model::Predator) {
            PREDATOR_CALIBRATION_FACTOR
        } else {
            1.0
        };
        for (bit, (cell, &offset)) in cells.iter_mut().zip(&layout.calibration).enumerate() {
            if mask & (1 << bit) != 0 {
                let raw = header.u16(offset, "cell calibration")?;
                all_default &= raw == DEFAULT_CALIBRATION;
                *cell = Some(f64::from(raw) * factor / 100_000.0);
            }
        }
        if all_default {
            // Probably not calibrated (DiveCAN); the spec allows dropping the cells.
            cells = [None; 3];
        }

        let setpoints = match (layout.setpoints, model) {
            (Some([low, high]), Some(Model::Predator)) => Some([
                header.u8(low, "low setpoint")?,
                header.u8(high, "high setpoint")?,
            ]),
            _ => None,
        };
        let atmospheric = header.u16(layout.atmospheric, "atmospheric pressure")?;
        let density = header.u16(layout.density, "water density")?;
        Ok(SampleContext {
            units,
            version,
            cells,
            setpoints,
            dciem: deco_model == Some(DecoModel::Dciem),
            sample_pressures,
            extended_pressures,
            freedive_surface: (atmospheric != 0 && density != 0)
                .then_some((f64::from(atmospheric), f64::from(density))),
        })
    }

    /// Decodes the samples and events in order.
    fn walk(&self, walk: &mut Walk<'_>, start: u32) -> Result<()> {
        match self.body {
            Body::Block { samples, size } => {
                for (base, sample) in (BLOCK_SIZE..).step_by(size).zip(samples.chunks_exact(size)) {
                    if sample.iter().any(|&b| b != 0) {
                        walk.dive_sample(Fields::new(sample, base), false)?;
                    }
                }
            }
            Body::Native(records) => {
                for (base, record) in (0..)
                    .step_by(RECORD_SIZE)
                    .zip(records.chunks_exact(RECORD_SIZE))
                {
                    let fields = Fields::new(record, base);
                    // Sample fields start after the type byte.
                    let sample =
                        Fields::new(record.get(1..).unwrap_or_default(), base.saturating_add(1));
                    match record.first().copied() {
                        Some(RECORD_DIVE_SAMPLE) => walk.dive_sample(sample, false)?,
                        Some(RECORD_AVELO_SAMPLE) => walk.dive_sample(sample, true)?,
                        Some(RECORD_FREEDIVE) => walk.freedive(record, base)?,
                        Some(RECORD_EXTENDED_SAMPLE) => walk.extended(fields)?,
                        Some(RECORD_INFO_EVENT) => walk.info_event(fields, start)?,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Spec: "Dive mode".
    fn mode(&self, walk: &Walk<'_>, version: u8) -> Result<Option<DiveMode>> {
        Ok(if walk.freedive {
            Some(DiveMode::Freedive)
        } else if version >= 8 {
            match self.header().u8(self.layout.dive_mode, "dive mode")? {
                0 | 5 => Some(DiveMode::ClosedCircuit),
                1 | 6 | 12 => Some(DiveMode::OpenCircuit),
                2 | 3 => Some(DiveMode::Gauge),
                4 => Some(DiveMode::SemiClosed),
                7 => Some(DiveMode::Freedive),
                _ => None,
            }
        } else if walk.closed {
            Some(DiveMode::ClosedCircuit)
        } else if walk.semi_closed {
            Some(DiveMode::SemiClosed)
        } else {
            Some(DiveMode::OpenCircuit)
        })
    }

    /// Fills the gas list and gas-switch events (spec: "Gas list").
    fn gases(
        &self,
        dive: &mut Dive,
        switches: &[(Duration, GasKey)],
        rebreather: bool,
    ) -> Result<()> {
        let (header, layout) = (self.header(), self.layout);
        let gas_mask = if self.is_native() {
            header.u16(GAS_ENABLED_MASK, "gas enabled mask")?
        } else {
            u16::MAX
        };
        let mut keys = Vec::new();
        for (slot, (&oxygen, &helium)) in layout.oxygen.iter().zip(&layout.helium).enumerate() {
            let key = GasKey {
                oxygen: header.u8(oxygen, "gas O2")?,
                helium: header.u8(helium, "gas He")?,
                diluent: slot >= FIRST_DILUENT_SLOT,
            };
            if (key.oxygen, key.helium) == (0, 0) || (key.diluent && !rebreather) {
                continue;
            }
            let used = switches.iter().any(|(_, k)| *k == key);
            if gas_mask & (1 << slot) != 0 || used {
                keys.push(key);
            }
        }
        for (n, (time, key)) in switches.iter().enumerate() {
            let index = if let Some(index) = keys.iter().position(|k| k == key) {
                index
            } else if keys.len() < MAX_GAS_MIXES {
                keys.push(*key);
                keys.len().saturating_sub(1)
            } else {
                continue;
            };
            // The first sample's gas is the starting gas, not a switch.
            if n > 0 {
                dive.events
                    .push(Event::new(*time, EventKind::GasSwitch { gas_mix: index }));
            }
        }
        dive.gas_mixes = keys.into_iter().map(GasKey::mix).collect();
        Ok(())
    }

    /// One tank per transmitter slot that reported, in order of first reading.
    fn tanks(
        &self,
        walk: &Walk<'_>,
        model: Option<Model>,
        version: u8,
        rebreather: bool,
    ) -> Vec<Tank> {
        let header = self.header();
        let teric = model == Some(Model::Teric);
        let mut tanks = Vec::new();
        for (index, slot) in walk.slots.iter().enumerate() {
            let mut tank = Tank::default();
            match *slot {
                Slot::Transmitter(t) => {
                    if let Some(layout) = TRANSMITTERS.get(t).filter(|_| self.is_native()) {
                        let record = u8::try_from(layout.serial / RECORD_SIZE).unwrap_or(u8::MAX);
                        if version >= layout.serial_version && self.has(record) {
                            tank.transmitter = header
                                .get::<3>(layout.serial)
                                .and_then(|s| serial(s, teric));
                        }
                        if version >= TANK_NAMES_VERSION {
                            tank.name = header.get::<2>(layout.name).and_then(name);
                        }
                    }
                    if rebreather {
                        tank.usage = match tank.name.as_deref().and_then(|n| n.chars().next()) {
                            Some('O') => Some(GasUsage::Oxygen),
                            Some('D') => Some(GasUsage::Diluent),
                            _ => None,
                        };
                    }
                }
                Slot::HpDiluent => tank.usage = Some(GasUsage::Diluent),
                Slot::HpOxygen => tank.usage = Some(GasUsage::Oxygen),
            }
            let mut readings = walk
                .samples
                .iter()
                .flat_map(|s| &s.pressures)
                .filter(|p| p.tank == index)
                .map(|p| p.pressure);
            tank.begin_pressure = readings.next();
            tank.end_pressure = readings.next_back().or(tank.begin_pressure);
            tanks.push(tank);
        }
        tanks
    }
}

/// Depth and temperature units of a log (spec: "Units").
#[derive(Clone, Copy)]
struct Units {
    imperial: bool,
}

impl Units {
    fn depth(self, value: f64) -> Depth {
        if self.imperial {
            Depth::from_feet(value)
        } else {
            Depth::from_metres(value)
        }
    }

    fn temperature(self, value: f64) -> Temperature {
        if self.imperial {
            Temperature::from_fahrenheit(value)
        } else {
            Temperature::from_celsius(value)
        }
    }
}

/// A breathing gas as seen in a sample: O₂ %, He %, diluent (closed circuit).
#[derive(Clone, Copy, PartialEq, Eq)]
struct GasKey {
    oxygen: u8,
    helium: u8,
    diluent: bool,
}

impl GasKey {
    const fn usage(self) -> GasUsage {
        if self.diluent {
            GasUsage::Diluent
        } else {
            GasUsage::OpenCircuit
        }
    }

    fn mix(self) -> GasMix {
        GasMix::new(
            GasFraction::from_percent(f64::from(self.oxygen)),
            GasFraction::from_percent(f64::from(self.helium)),
            Some(self.usage()),
        )
    }
}

/// Header values a sample decoder needs.
struct SampleContext {
    units: Units,
    version: u8,
    /// Per-cell calibration in bar/mV, for calibrated cells.
    cells: [Option<f64>; 3],
    /// Predator low and high setpoints, 0.01 bar.
    setpoints: Option<[u8; 2]>,
    dciem: bool,
    sample_pressures: Vec<(Slot, usize)>,
    /// (transmitter, record offset) in an extended sample.
    extended_pressures: Vec<(usize, usize)>,
    /// Atmospheric pressure (mbar) and density (kg/m³) for freedive depths.
    freedive_surface: Option<(f64, f64)>,
}

/// State while walking the samples in order.
struct Walk<'c> {
    context: &'c SampleContext,
    interval: Duration,
    /// Samples decoded so far; also the time base.
    count: u32,
    samples: Vec<Sample>,
    events: Vec<Event>,
    /// Gas changes: (time, new gas).
    switches: Vec<(Duration, GasKey)>,
    gas: Option<GasKey>,
    /// Tank slot of each tank index, in order of first reading.
    slots: Vec<Slot>,
    closed: bool,
    semi_closed: bool,
    freedive: bool,
}

impl<'c> Walk<'c> {
    const fn new(context: &'c SampleContext, interval: Duration) -> Self {
        Self {
            context,
            interval,
            count: 0,
            samples: Vec::new(),
            events: Vec::new(),
            switches: Vec::new(),
            gas: None,
            slots: Vec::new(),
            closed: false,
            semi_closed: false,
            freedive: false,
        }
    }

    /// The time of the next sample: *k* × interval for sample *k* counting from 1.
    fn next_time(&mut self, base: usize) -> Result<Duration> {
        self.count = self.count.checked_add(1).ok_or_else(|| {
            Error::Protocol(format!("Shearwater log: too many samples at byte {base}"))
        })?;
        self.interval.checked_mul(self.count).ok_or_else(|| {
            Error::Protocol(format!(
                "Shearwater log: sample time overflows at byte {base}"
            ))
        })
    }

    /// Adds a tank reading, replacing an earlier one for the same tank at this instant.
    fn add_pressure(&mut self, sample: &mut Sample, slot: Slot, pressure: Option<Pressure>) {
        let Some(pressure) = pressure else { return };
        let tank = if let Some(tank) = self.slots.iter().position(|s| *s == slot) {
            tank
        } else {
            self.slots.push(slot);
            self.slots.len().saturating_sub(1)
        };
        sample.pressures.retain(|p| p.tank != tank);
        sample.pressures.push(TankPressure::new(tank, pressure));
    }

    /// Decodes a dive sample (spec: "Dive sample"). `f` starts at the sample fields.
    fn dive_sample(&mut self, f: Fields<'_>, avelo: bool) -> Result<()> {
        let c = self.context;
        f.require::<MIN_SAMPLE_FIELDS>(0, "dive sample")?;
        let mut sample = Sample::default();
        sample.time = self.next_time(f.base)?;
        sample.depth = Some(c.units.depth(f64::from(f.u16(0, "depth")?) / 10.0));
        let next_stop = f.u16(2, "next stop depth")?;
        let stop_time = minutes(u32::from(f.u8(9, "stop time / NDL")?));
        // A 0 NDL with no stop on the first sample means not computed yet.
        sample.deco = if next_stop == 0 && stop_time.is_zero() && self.samples.is_empty() {
            None
        } else if next_stop == 0 {
            Some(Deco::Ndl { time: stop_time })
        } else {
            Some(Deco::Stop {
                depth: c.units.depth(f64::from(next_stop)),
                time: stop_time,
            })
        };
        sample.time_to_surface = Some(minutes(u32::from(f.u16(4, "time to surface")?)));
        let status = f.u8(11, "status")?;
        let open = avelo || status & STATUS_OPEN_CIRCUIT != 0;
        if !open {
            if status & STATUS_SEMI_CLOSED != 0 {
                self.semi_closed = true;
            } else {
                self.closed = true;
            }
            if status & (STATUS_EXTERNAL_PPO2 | STATUS_BAILOUT) == 0 {
                sample.ppo2 = Some(centibar(f.u8(6, "ppO2")?));
                if c.cells.iter().any(Option::is_some) {
                    for (calibration, offset) in c.cells.iter().zip([12, 14, 15]) {
                        let mv = f64::from(f.u8(offset, "O2 cell")?);
                        sample
                            .ppo2_sensors
                            .push(calibration.map(|cal| Pressure::from_bar(mv * cal)));
                    }
                }
            }
            sample.setpoint = match c.setpoints {
                Some([low, high]) => Some(centibar(if status & STATUS_HIGH_SETPOINT != 0 {
                    high
                } else {
                    low
                })),
                None => f.opt_u8(18).map(centibar),
            };
        }
        let raw = i16::from(i8::from_ne_bytes([f.u8(13, "temperature")?]));
        let corrected = if raw < 0 {
            raw.saturating_add(102).min(0)
        } else {
            raw
        };
        sample.temperature = Some(c.units.temperature(f64::from(corrected)));
        sample.cns = f.opt_u8(22).map(f64::from);
        if !c.dciem {
            sample.gradient_factor = f.opt_u8(24).filter(|&gf| gf != GF99_NONE);
        }
        if c.version >= 7 {
            sample.gas_time_remaining = f
                .opt_u8(21)
                .filter(|&m| m < GTR_STATUS_CODES)
                .map(|m| minutes(u32::from(m)));
            let pressures: &[(Slot, usize)] = if avelo {
                &[(Slot::Transmitter(0), SAMPLE_T1_PRESSURE)]
            } else {
                &c.sample_pressures
            };
            for &(slot, offset) in pressures {
                self.add_pressure(
                    &mut sample,
                    slot,
                    f.opt_u16(offset).and_then(transmitter_pressure),
                );
            }
        }

        let oxygen = f.u8(7, "gas O2")?;
        let helium = f.u8(8, "gas He")?;
        if (oxygen, helium) != (0, 0) {
            let key = GasKey {
                oxygen,
                helium,
                diluent: !open,
            };
            if self.gas != Some(key) {
                self.switches.push((sample.time, key));
                self.gas = Some(key);
            }
        }
        self.samples.push(sample);
        Ok(())
    }

    /// Adds an extended sample's tanks to the preceding sample (spec: "Extended sample").
    fn extended(&mut self, f: Fields<'_>) -> Result<()> {
        let version = self.context.version;
        let Some(mut sample) = self.samples.pop() else {
            return Ok(());
        };
        if version >= 13 {
            for &(t, offset) in &self.context.extended_pressures {
                let raw = f.u16(offset, "extended transmitter pressure")?;
                self.add_pressure(&mut sample, Slot::Transmitter(t), transmitter_pressure(raw));
            }
        }
        if version >= 14 {
            for (slot, offset) in [(Slot::HpDiluent, 5), (Slot::HpOxygen, 7)] {
                let raw = f.u16(offset, "HP-CCR pressure")?;
                let pressure = (raw != 0).then(|| Pressure::from_psi(2.0 * f64::from(raw)));
                self.add_pressure(&mut sample, slot, pressure);
            }
        }
        self.samples.push(sample);
        Ok(())
    }

    /// Decodes a freedive record's sub-samples (spec: "Freedive record").
    fn freedive(&mut self, record: &[u8], base: usize) -> Result<()> {
        self.freedive = true;
        for (index, sub) in record.chunks_exact(FREEDIVE_SUBSAMPLE_SIZE).enumerate() {
            if index > 0 && sub.iter().all(|&b| b == 0) {
                break;
            }
            let f = Fields::new(
                sub,
                base.saturating_add(index.saturating_mul(FREEDIVE_SUBSAMPLE_SIZE)),
            );
            let pressure = f64::from(f.u16(1, "freedive pressure")?);
            let temperature = f64::from(i16::from_be_bytes(f.require(3, "freedive temperature")?));
            let mut sample = Sample::default();
            sample.time = self.next_time(f.base)?;
            sample.depth = self.context.freedive_surface.map(|(atmospheric, density)| {
                // Pressure at the surface can read just below atmospheric; that is depth 0.
                Depth::from_metres(
                    ((pressure - atmospheric) * 100.0 / (density * GRAVITY)).max(0.0),
                )
            });
            sample.temperature = Some(Temperature::from_celsius(temperature / 10.0));
            self.samples.push(sample);
        }
        Ok(())
    }

    /// Decodes an info event (spec: "Info event").
    fn info_event(&mut self, f: Fields<'_>, start: u32) -> Result<()> {
        let code = f.u8(1, "event code")?;
        let timestamp = f.u32(4, "event timestamp")?;
        let value = f.u32(8, "event parameter")?;
        // shortcut: an event stamped before the dive start has no place on the
        // elapsed-time axis and is dropped.
        let Some(elapsed) = timestamp.checked_sub(start) else {
            return Ok(());
        };
        let kind = if code == EVENT_BOOKMARK {
            EventKind::Bookmark {
                heading: (value != NO_HEADING).then_some(value),
            }
        } else {
            EventKind::Vendor {
                code: u32::from(code),
                value,
            }
        };
        self.events
            .push(Event::new(Duration::from_secs(u64::from(elapsed)), kind));
        Ok(())
    }
}

/// Minutes as a [`Duration`].
fn minutes(minutes: u32) -> Duration {
    Duration::from_secs(60).saturating_mul(minutes)
}

/// A pressure in units of 0.01 bar.
fn centibar(raw: u8) -> Pressure {
    Pressure::from_bar(f64::from(raw) / 100.0)
}

/// A transmitter reading (spec: "Air integration"): status codes and 0 are no reading;
/// bits 0–11 are 2 psi units.
fn transmitter_pressure(raw: u16) -> Option<Pressure> {
    let psi2 = raw & PRESSURE_MASK;
    (raw < PRESSURE_STATUS_CODES && psi2 != 0).then(|| Pressure::from_psi(2.0 * f64::from(psi2)))
}

/// Renders a `bcd8` byte as its two decimal digits, or `None` if it is not BCD.
fn bcd(byte: u8) -> Option<String> {
    (byte >> 4 < 10 && byte & 0x0F < 10).then(|| format!("{byte:02X}"))
}

/// A 6-digit BCD transmitter serial; all zero means no transmitter. The Teric stores
/// the bytes in reverse order (spec: "Air integration").
fn serial(mut bytes: [u8; 3], teric: bool) -> Option<String> {
    if bytes == [0; 3] {
        return None;
    }
    if teric {
        bytes.reverse();
    }
    let [a, b, c] = bytes;
    Some(format!("{}{}{}", bcd(a)?, bcd(b)?, bcd(c)?))
}

/// A 2-character ASCII tank name, `None` if empty or not printable.
fn name(bytes: [u8; 2]) -> Option<String> {
    let text = core::str::from_utf8(&bytes)
        .ok()?
        .trim_matches(|c: char| c == '\0' || c == ' ');
    (!text.is_empty() && text.chars().all(|c| c.is_ascii_graphic() || c == ' '))
        .then(|| text.into())
}

#[cfg(test)]
#[allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    clippy::assert_is_empty
)]
mod tests {
    use super::*;

    /// A 32-byte native record of type `kind` with `fields` at their record offsets.
    fn record(kind: u8, fields: &[(usize, &[u8])]) -> Vec<u8> {
        let mut record = vec![0; RECORD_SIZE];
        record[0] = kind;
        for (offset, bytes) in fields {
            record[*offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        record
    }

    /// A dive-sample record; `fields` offsets are relative to the sample fields.
    fn sample(kind: u8, fields: &[(usize, &[u8])]) -> Vec<u8> {
        let shifted: Vec<_> = fields.iter().map(|&(o, b)| (o + 1, b)).collect();
        record(kind, &shifted)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    /// The native-format example from the spec (Perdix 2, imperial, fresh water).
    fn spec_example() -> Vec<u8> {
        [
            record(
                0x10,
                &[(
                    1,
                    &[
                        0xFF, 0x00, 0x02, 0x28, 0x55, 0x0B, 0x88, 0x01, 0x0F, 0x00, 0x00, 0x6A,
                        0x47, 0xA0, 0x8D, 0x00, 0x46, 0x82, 0xA2, 0x15,
                    ],
                )],
            ),
            record(0x11, &[(16, &[0x03, 0x31])]),
            record(0x12, &[]),
            record(0x13, &[(1, &[0x0B, 0x03, 0x03, 0xE8])]),
            record(0x14, &[(1, &[0x06]), (16, &[0x11, 0x00, 0x01])]),
            record(0x15, &[(23, &[0x27, 0x10])]),
            sample(
                0x01,
                &[(
                    0,
                    &[
                        0x00, 0x37, 0x00, 0x00, 0x00, 0x01, 0x14, 0x15, 0x00, 0x00, 0x58, 0x12,
                        0x00, 0x41,
                    ],
                )],
            ),
            record(
                0x20,
                &[(
                    1,
                    &[
                        0xFE, 0x00, 0x02, 0x00, 0x42, 0x00, 0x00, 0x1C, 0x0F, 0x00, 0x00, 0x6A,
                        0x47, 0xA0, 0xA4,
                    ],
                )],
            ),
            record(0x21, &[]),
            record(0x22, &[]),
            record(0x23, &[]),
            record(0x24, &[]),
            record(
                0xFF,
                &[
                    (1, &[0xFD, 0x12, 0x34, 0xAB, 0xCD]),
                    (10, &[0x92]),
                    (13, &[11]),
                ],
            ),
        ]
        .concat()
    }

    #[test]
    fn decodes_spec_example() {
        let dive = parse(&spec_example()).unwrap();
        assert_eq!(dive.number, Some(2));
        assert_eq!(
            dive.start,
            Some(jiff::civil::date(2026, 7, 3).at(11, 44, 13, 0))
        );
        assert_eq!(dive.utc_offset, None);
        assert_eq!(dive.duration, Some(Duration::from_secs(28)));
        assert!(close(dive.max_depth.unwrap().metres(), 6.6 * 0.3048));
        assert_eq!(dive.mode, Some(DiveMode::OpenCircuit));
        assert_eq!(dive.salinity, Some(Salinity::Fresh));
        assert_eq!(dive.water_density, Some(1000.0));
        assert!(close(dive.surface_pressure.unwrap().bar(), 0.817));
        assert_eq!(
            dive.deco_model,
            Some(DecoModel::Buhlmann {
                gf_low: 40,
                gf_high: 85
            })
        );
        assert_eq!(dive.gas_mixes.len(), 1);
        assert_eq!(dive.gas_mixes[0].oxygen.percent(), 21.0);
        assert_eq!(dive.gas_mixes[0].helium.percent(), 0.0);
        assert_eq!(dive.gas_mixes[0].usage, Some(GasUsage::OpenCircuit));
        // The only sample's gas is the starting gas, not a switch.
        assert!(dive.events.is_empty());
        assert_eq!(dive.device.brand.as_deref(), Some("Shearwater"));
        assert_eq!(dive.device.model.as_deref(), Some("Perdix 2"));
        assert_eq!(dive.device.serial.as_deref(), Some("1234ABCD"));
        assert_eq!(dive.device.firmware.as_deref(), Some("92"));
        assert!(dive.tanks.is_empty());

        let [s] = dive.samples.as_slice() else {
            panic!("one sample expected");
        };
        assert_eq!(s.time, Duration::from_secs(10));
        assert!(close(s.depth.unwrap().metres(), 5.5 * 0.3048));
        assert!(close(
            s.temperature.unwrap().celsius(),
            (65.0 - 32.0) * 5.0 / 9.0
        ));
        assert_eq!(s.time_to_surface, Some(Duration::from_secs(60)));
        // An NDL of 0 with no stop on the first sample is not computed yet.
        assert_eq!(s.deco, None);
        // Open circuit: no ppO2 or setpoint.
        assert_eq!(s.ppo2, None);
        assert_eq!(s.setpoint, None);
    }

    #[test]
    fn decodes_native_features() {
        let mut log = spec_example();
        let final_record = log.split_off(log.len() - RECORD_SIZE);
        // Metric Teric dive, log version 14, mode 5 with transmitters 1 and 3 enabled.
        log[8] = 0; // units
        let opening4 = 4 * RECORD_SIZE;
        log[opening4 + 16] = 14;
        log[opening4 + 28] = 5;
        let opening5 = 5 * RECORD_SIZE;
        log[opening5 + 1..opening5 + 4].copy_from_slice(&[0x56, 0x34, 0x12]);
        log[opening5 + 26..opening5 + 30].copy_from_slice(&(-120_i32).to_be_bytes());
        log[opening5 + 30] = 1;
        log.extend(record(0x16, &[(19, &[1, b'O', b'2'])]));
        log.extend(record(0x17, &[(1, &[1])]));
        // A second sample: 10.0 m, deco stop at 6 m for 2 min, T1 3000 psi, cold water.
        log.extend(sample(
            0x01,
            &[
                (0, &[0x00, 0x64, 0x00, 0x06, 0x00, 0x05]),
                (7, &[32, 0, 2]),
                (11, &[0x10]),
                (13, &[(-110_i8).to_be_bytes()[0]]),
                (21, &[0x2A, 50, 12, 0xFF]),
                (27, &(0x2000_u16 | 0x05DC).to_be_bytes()),
            ],
        ));
        log.extend(record(0xE1, &[(1, &700_u16.to_be_bytes())]));
        log.extend(record(
            0x30,
            &[
                (1, &[38]),
                (4, &0x6A47_A097_u32.to_be_bytes()),
                (8, &[0, 0, 0, 90]),
            ],
        ));
        log.extend(record(0x02, &[(1, &[0x07, 0xD0, 0x00, 0x96])]));
        let mut final_record = final_record;
        final_record[13] = 8;
        log.extend(final_record);

        let dive = parse(&log).unwrap();
        assert_eq!(dive.utc_offset, Some(-120 * 60 + 3600));
        assert_eq!(dive.device.model.as_deref(), Some("Teric"));
        assert_eq!(dive.mode, Some(DiveMode::Freedive));
        assert!(dive.gas_mixes.is_empty());
        let [t1, t3] = dive.tanks.as_slice() else {
            panic!("two tanks expected: {:?}", dive.tanks);
        };
        assert_eq!(t1.transmitter.as_deref(), Some("123456"));
        assert_eq!(t1.name.as_deref(), Some("O2"));
        assert!(close(
            t1.begin_pressure.unwrap().bar(),
            3000.0 * 0.068_947_572_9
        ));
        assert_eq!(t1.end_pressure, t1.begin_pressure);
        assert!(close(
            t3.begin_pressure.unwrap().bar(),
            1400.0 * 0.068_947_572_9
        ));

        let s = &dive.samples[1];
        assert_eq!(s.time, Duration::from_secs(20));
        assert!(close(s.depth.unwrap().metres(), 10.0));
        assert_eq!(
            s.deco,
            Some(Deco::Stop {
                depth: Depth::from_metres(6.0),
                time: Duration::from_secs(120)
            })
        );
        assert_eq!(s.time_to_surface, Some(Duration::from_secs(300)));
        assert!(close(s.temperature.unwrap().celsius(), -8.0));
        assert_eq!(s.gas_time_remaining, Some(Duration::from_secs(42 * 60)));
        assert_eq!(s.cns, Some(50.0));
        assert_eq!(s.gradient_factor, None);
        assert_eq!(s.pressures.len(), 2);
        // The bookmark 10 s after the start.
        assert!(dive.events.contains(&Event::new(
            Duration::from_secs(10),
            EventKind::Bookmark { heading: Some(90) }
        )));
        // Freedive sub-sample: 2000 mbar absolute, 15.0 °C.
        let f = &dive.samples[2];
        assert_eq!(f.time, Duration::from_secs(30));
        let expected = (2000.0 - 817.0) * 100.0 / (1000.0 * GRAVITY);
        assert!(close(f.depth.unwrap().metres(), expected));
        assert!(close(f.temperature.unwrap().celsius(), 15.0));
    }

    /// A Petrel block-format closed-circuit log, version 7 (mode derived from samples).
    #[test]
    fn decodes_block_format() {
        let mut opening = vec![0; BLOCK_SIZE];
        opening[..2].copy_from_slice(&MARKER_OPENING);
        opening[0x02..0x04].copy_from_slice(&7_u16.to_be_bytes());
        opening[0x0C..0x10].copy_from_slice(&0x6000_0000_u32.to_be_bytes());
        opening[0x14] = 21;
        opening[0x19] = 10; // diluent 1: 10/50
        opening[0x23] = 50;
        opening[0x2F..0x31].copy_from_slice(&1013_u16.to_be_bytes());
        opening[0x43] = 2;
        opening[0x44] = 3;
        opening[0x53..0x55].copy_from_slice(&1025_u16.to_be_bytes());
        opening[0x55] = 90;
        opening[0x56] = 0b011;
        opening[0x57..0x59].copy_from_slice(&2000_u16.to_be_bytes());
        opening[0x59..0x5B].copy_from_slice(&2200_u16.to_be_bytes());
        opening[0x7F] = 7;
        let mut sample = vec![0; PETREL_SAMPLE_SIZE];
        sample[0..2].copy_from_slice(&300_u16.to_be_bytes());
        sample[6] = 130;
        sample[7] = 10;
        sample[8] = 50;
        sample[11] = 0x00; // closed circuit
        sample[12] = 60;
        sample[13] = 12;
        sample[14] = 61;
        sample[15] = 62;
        sample[18] = 130;
        sample[24] = 0xFF;
        sample[27..29].copy_from_slice(&(0xFFFF_u16).to_be_bytes());
        let padding = vec![0; PETREL_SAMPLE_SIZE];
        let mut closing = vec![0; BLOCK_SIZE];
        closing[..2].copy_from_slice(&MARKER_CLOSING);
        closing[4..6].copy_from_slice(&31_u16.to_be_bytes());
        closing[6..8].copy_from_slice(&45_u16.to_be_bytes());
        let mut final_block = vec![0; BLOCK_SIZE];
        final_block[..2].copy_from_slice(&MARKER_FINAL);
        final_block[13] = 3;
        let log = [opening, sample, padding, closing, final_block].concat();

        let dive = parse(&log).unwrap();
        assert_eq!(dive.number, Some(7));
        assert_eq!(dive.mode, Some(DiveMode::ClosedCircuit));
        assert_eq!(dive.duration, Some(Duration::from_secs(45 * 60)));
        assert_eq!(dive.max_depth, Some(Depth::from_metres(31.0)));
        assert_eq!(dive.salinity, Some(Salinity::Salt));
        assert_eq!(
            dive.deco_model,
            Some(DecoModel::Vpm {
                conservatism: 3,
                gfs_gradient_factor: Some(90)
            })
        );
        assert_eq!(dive.device.model.as_deref(), Some("Petrel"));
        // Both slots enabled in the block format; the diluent is listed on a CC dive.
        assert_eq!(dive.gas_mixes.len(), 2);
        assert_eq!(dive.gas_mixes[1].usage, Some(GasUsage::Diluent));
        assert!(dive.tanks.is_empty());
        let [s] = dive.samples.as_slice() else {
            panic!("padding is skipped");
        };
        assert!(close(s.ppo2.unwrap().bar(), 1.3));
        assert!(close(s.setpoint.unwrap().bar(), 1.3));
        assert_eq!(s.ppo2_sensors.len(), 3);
        assert!(close(s.ppo2_sensors[0].unwrap().bar(), 60.0 * 0.02));
        assert!(close(s.ppo2_sensors[1].unwrap().bar(), 61.0 * 0.022));
        assert_eq!(s.ppo2_sensors[2], None);
        assert_eq!(s.gradient_factor, None);
        // The only sample's gas is the starting gas, not a switch.
        assert!(dive.events.is_empty());
    }

    /// A Predator log: no final block, 16-byte samples, header setpoints.
    #[test]
    fn decodes_predator_samples() {
        let mut opening = vec![0; BLOCK_SIZE];
        opening[..2].copy_from_slice(&MARKER_OPENING);
        opening[0x08] = 1;
        opening[0x11] = 70;
        opening[0x12] = 120;
        opening[0x56] = 0b001;
        opening[0x57..0x59].copy_from_slice(&1000_u16.to_be_bytes());
        let mut sample = vec![0; PREDATOR_SAMPLE_SIZE];
        sample[11] = STATUS_HIGH_SETPOINT;
        sample[12] = 50;
        let mut closing = vec![0; BLOCK_SIZE];
        closing[..2].copy_from_slice(&MARKER_CLOSING);
        closing[4..6].copy_from_slice(&100_u16.to_be_bytes());
        let log = [opening, sample.clone(), sample, closing].concat();

        let dive = parse(&log).unwrap();
        assert_eq!(dive.device.model.as_deref(), Some("Predator"));
        assert_eq!(dive.device.serial, None);
        assert!(close(dive.max_depth.unwrap().metres(), 30.48));
        assert_eq!(dive.samples.len(), 2);
        let s = &dive.samples[1];
        assert_eq!(s.time, Duration::from_secs(20));
        assert!(close(s.setpoint.unwrap().bar(), 1.2));
        assert!(close(s.ppo2_sensors[0].unwrap().bar(), 50.0 * 0.01 * 2.2));
        assert_eq!(s.cns, None);
        assert_eq!(s.pressures, []);
    }

    #[test]
    fn rejects_malformed_logs() {
        let log = spec_example();
        // Missing opening record 2.
        let mut missing = log.clone();
        missing.drain(2 * RECORD_SIZE..3 * RECORD_SIZE);
        assert!(matches!(parse(&missing), Err(Error::Protocol(m)) if m.contains("opening")));
        // Not a whole number of records.
        assert!(parse(&log[..log.len() - 1]).is_err());
        // Block format without a closing block.
        assert!(parse(&[0xFF; BLOCK_SIZE * 2]).is_err());
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn decodes_helpers() {
        assert_eq!(bcd(0x92).as_deref(), Some("92"));
        assert_eq!(bcd(0xA2), None);
        assert_eq!(serial([0x56, 0x34, 0x12], true).as_deref(), Some("123456"));
        assert_eq!(serial([0; 3], false), None);
        assert_eq!(transmitter_pressure(0xFFFE), None);
        assert_eq!(transmitter_pressure(0x1000), None);
        assert!(close(
            transmitter_pressure(0x2001).unwrap().bar(),
            2.0 * 0.068_947_572_9
        ));
        assert_eq!(name([b'D', 0]).as_deref(), Some("D"));
        assert_eq!(name([0, 0]), None);
    }

    /// Arbitrary, truncated and mutated input never panics.
    #[test]
    fn never_panics() {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let valid = spec_example();
        for len in 0..=valid.len() {
            let _ = parse(&valid[..len]);
        }
        for _ in 0..5000 {
            let mut log = valid.clone();
            for _ in 0..(next() % 8) {
                let i = (next() as usize) % log.len();
                log[i] = next() as u8;
            }
            let _ = parse(&log);
            let len = (next() % 600) as usize;
            let mut noise: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            if next() % 2 == 0 && noise.len() >= 2 {
                noise[..2].copy_from_slice(&MARKER_OPENING);
            }
            let _ = parse(&noise);
        }
    }
}
