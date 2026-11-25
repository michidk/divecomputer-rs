//! The dive data model ([spec](https://michidk.github.io/divecomputer-rs/spec/common/data-model.html)).
//!
//! A download yields [`RawDive`]s; a family's parser turns one into a [`Dive`]. The model
//! mirrors one DiveJSON *recording*: header values, gas mixes, tanks, samples and events
//! from one device.

use alloc::{string::String, vec::Vec};
use core::time::Duration;

use crate::units::{Depth, GasFraction, Pressure, Temperature};

/// A dive as downloaded, before parsing: the bytes exactly as the device sent them, and
/// the fingerprint that identifies the dive for incremental downloads.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawDive {
    fingerprint: Vec<u8>,
    data: Vec<u8>,
}

impl RawDive {
    /// Creates a raw dive.
    #[must_use]
    pub const fn new(fingerprint: Vec<u8>, data: Vec<u8>) -> Self {
        Self { fingerprint, data }
    }

    /// The bytes that identify this dive on its device. Store the newest dive's
    /// fingerprint and pass it to the next download to fetch only newer dives.
    #[must_use]
    pub fn fingerprint(&self) -> &[u8] {
        &self.fingerprint
    }

    /// The dive's raw bytes, in the family's log format.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// A parsed dive: one device's recording of one dive.
///
/// Every field is optional or may be empty: a parser fills what the device recorded and
/// leaves the rest absent, never zero-filled. Values are in the units of
/// [`crate::units`]; times are elapsed from the dive start.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Dive {
    /// The dive start on the device's local wall clock. Combine with
    /// [`utc_offset`](Self::utc_offset) when that is known; never convert through the
    /// host's time zone.
    pub start: Option<jiff::civil::DateTime>,
    /// Offset of [`start`](Self::start) from UTC in seconds, only when the device
    /// recorded its time zone.
    pub utc_offset: Option<i32>,
    /// The device's own dive counter.
    pub number: Option<u32>,
    /// Dive time as stored by the device. May differ from the last sample time.
    #[cfg_attr(feature = "serde", serde(with = "millis_opt"))]
    pub duration: Option<Duration>,
    /// Maximum depth as stored by the device, not recomputed from samples.
    pub max_depth: Option<Depth>,
    /// Average depth as stored by the device.
    pub avg_depth: Option<Depth>,
    /// Lowest water temperature as stored by the device.
    pub min_temperature: Option<Temperature>,
    /// Highest water temperature as stored by the device.
    pub max_temperature: Option<Temperature>,
    /// The mode the device ran in.
    pub mode: Option<DiveMode>,
    /// The water type the device was set to.
    pub salinity: Option<Salinity>,
    /// The water density the device was set to, in kg/m³.
    pub water_density: Option<f64>,
    /// The surface (atmospheric) pressure the device measured or assumed.
    pub surface_pressure: Option<Pressure>,
    /// The decompression model the device ran, with its settings.
    pub deco_model: Option<DecoModel>,
    /// Surface position at the start of the dive.
    pub location: Option<Location>,
    /// The device that recorded the dive.
    pub device: DeviceInfo,
    /// Gas mixes; [`Sample`]s and [`Event`]s refer to them by index.
    pub gas_mixes: Vec<GasMix>,
    /// Tanks; [`TankPressure`]s refer to them by index.
    pub tanks: Vec<Tank>,
    /// Samples in time order.
    pub samples: Vec<Sample>,
    /// Events in time order. An event may fall after the last sample.
    pub events: Vec<Event>,
}

/// Identification of the device that recorded a dive. Strings are as the device
/// reports them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct DeviceInfo {
    /// The maker, for example `Shearwater`.
    pub brand: Option<String>,
    /// The product name, for example `Perdix 2`.
    pub model: Option<String>,
    /// The device's serial number in its own rendering.
    pub serial: Option<String>,
    /// The firmware version the device was running.
    pub firmware: Option<String>,
}

/// The mode a dive computer ran in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum DiveMode {
    /// Open circuit.
    OpenCircuit,
    /// Closed-circuit rebreather.
    ClosedCircuit,
    /// Semi-closed rebreather.
    SemiClosed,
    /// Depth and time only, no decompression calculation.
    Gauge,
    /// Breath-hold diving.
    Freedive,
}

/// The water type a device converts pressure to depth with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum Salinity {
    /// Fresh water.
    Fresh,
    /// The EN 13319 convention (1020 kg/m³).
    En13319,
    /// Salt water.
    Salt,
}

/// A decompression model and the settings it ran with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "algorithm", rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum DecoModel {
    /// Bühlmann with gradient factors.
    Buhlmann {
        /// Gradient factor low, percent.
        gf_low: u8,
        /// Gradient factor high, percent.
        gf_high: u8,
    },
    /// VPM-B, optionally combined with a Bühlmann gradient factor (VPM-B/GFS).
    Vpm {
        /// Conservatism on the device's own scale.
        conservatism: u8,
        /// The GFS gradient factor in percent, for VPM-B/GFS.
        gfs_gradient_factor: Option<u8>,
    },
    /// DCIEM.
    Dciem,
}

/// A position in decimal degrees (WGS 84).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Location {
    /// Latitude, −90 to 90.
    pub latitude: f64,
    /// Longitude, −180 to 180.
    pub longitude: f64,
}

impl Location {
    /// A position in decimal degrees.
    #[must_use]
    pub const fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

/// A breathing gas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct GasMix {
    /// Oxygen share.
    pub oxygen: GasFraction,
    /// Helium share; 0 for nitrox and air.
    pub helium: GasFraction,
    /// What the mix is used for, when the device says.
    pub usage: Option<GasUsage>,
}

impl GasMix {
    /// A mix of oxygen and helium, the rest nitrogen.
    #[must_use]
    pub const fn new(oxygen: GasFraction, helium: GasFraction, usage: Option<GasUsage>) -> Self {
        Self {
            oxygen,
            helium,
            usage,
        }
    }
}

/// What a gas or tank is used for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum GasUsage {
    /// Breathed open circuit.
    OpenCircuit,
    /// Rebreather diluent.
    Diluent,
    /// Rebreather oxygen.
    Oxygen,
}

/// A tank, usually one monitored by a wireless pressure transmitter.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Tank {
    /// The device's name for the tank or transmitter.
    pub name: Option<String>,
    /// The transmitter's serial number in the device's rendering.
    pub transmitter: Option<String>,
    /// What the tank is used for.
    pub usage: Option<GasUsage>,
    /// Index into [`Dive::gas_mixes`].
    pub gas_mix: Option<usize>,
    /// Pressure at the start of the dive.
    pub begin_pressure: Option<Pressure>,
    /// Pressure at the end of the dive.
    pub end_pressure: Option<Pressure>,
}

/// The values a device recorded at one instant. Absent values were not recorded.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Sample {
    /// Elapsed time since the dive start.
    #[cfg_attr(feature = "serde", serde(with = "millis"))]
    pub time: Duration,
    /// Depth.
    pub depth: Option<Depth>,
    /// Water temperature.
    pub temperature: Option<Temperature>,
    /// Tank pressures, one entry per tank that reported.
    pub pressures: Vec<TankPressure>,
    /// The ppO₂ the device computed or voted from its sensors.
    pub ppo2: Option<Pressure>,
    /// ppO₂ of each oxygen sensor, by sensor number (0 = first); `None` for a sensor
    /// without a usable reading.
    pub ppo2_sensors: Vec<Option<Pressure>>,
    /// Rebreather setpoint.
    pub setpoint: Option<Pressure>,
    /// CNS oxygen toxicity, in percent.
    pub cns: Option<f64>,
    /// No-decompression limit or the next decompression stop.
    pub deco: Option<Deco>,
    /// Time to surface.
    #[cfg_attr(feature = "serde", serde(with = "millis_opt"))]
    pub time_to_surface: Option<Duration>,
    /// Remaining gas time from air integration.
    #[cfg_attr(feature = "serde", serde(with = "millis_opt"))]
    pub gas_time_remaining: Option<Duration>,
    /// Gradient factor of the leading tissue (GF99), in percent.
    pub gradient_factor: Option<u8>,
}

/// A tank pressure reading.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct TankPressure {
    /// Index into [`Dive::tanks`].
    pub tank: usize,
    /// The pressure.
    pub pressure: Pressure,
}

impl TankPressure {
    /// A reading for tank `tank`.
    #[must_use]
    pub const fn new(tank: usize, pressure: Pressure) -> Self {
        Self { tank, pressure }
    }
}

/// Decompression state at a sample.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "kind", rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum Deco {
    /// No decompression obligation; `time` is the remaining no-decompression time.
    Ndl {
        /// Remaining no-decompression time.
        #[cfg_attr(feature = "serde", serde(with = "millis"))]
        time: Duration,
    },
    /// A decompression stop is required.
    Stop {
        /// Depth of the next stop.
        depth: Depth,
        /// Time at that stop.
        #[cfg_attr(feature = "serde", serde(with = "millis"))]
        time: Duration,
    },
}

/// Something that happened at one instant of the dive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Event {
    /// Elapsed time since the dive start.
    #[cfg_attr(feature = "serde", serde(with = "millis"))]
    pub time: Duration,
    /// What happened.
    pub kind: EventKind,
}

impl Event {
    /// An event at `time`.
    #[must_use]
    pub const fn new(time: Duration, kind: EventKind) -> Self {
        Self { time, kind }
    }
}

/// The kinds of [`Event`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", rename_all = "snake_case")
)]
#[non_exhaustive]
pub enum EventKind {
    /// The breathing gas changed.
    GasSwitch {
        /// Index into [`Dive::gas_mixes`].
        gas_mix: usize,
    },
    /// The diver marked this moment.
    Bookmark {
        /// Compass heading in degrees, when recorded.
        heading: Option<u32>,
    },
    /// A device event with no common meaning.
    Vendor {
        /// The device's event code.
        code: u32,
        /// The event's first parameter.
        value: u32,
    },
}

/// Serializes a [`Duration`] as whole milliseconds.
#[cfg(feature = "serde")]
mod millis {
    use core::time::Duration;

    pub(super) fn serialize<S: serde::Serializer>(
        duration: &Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
    }
}

/// Serializes an optional [`Duration`] as whole milliseconds.
#[cfg(feature = "serde")]
mod millis_opt {
    use core::time::Duration;

    #[allow(clippy::ref_option, reason = "signature required by serde(with)")]
    pub(super) fn serialize<S: serde::Serializer>(
        duration: &Option<Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match duration {
            Some(duration) => super::millis::serialize(duration, serializer),
            None => serializer.serialize_none(),
        }
    }
}
