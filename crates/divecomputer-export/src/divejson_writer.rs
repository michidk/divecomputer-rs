//! The DiveJSON writer: the core dive model to a [`divejson::Document`].
//!
//! Follows the [DiveJSON specification](https://divejson.org) and its converter guide
//! (`docs/converting.md` in the specification repository): nothing is invented, a value the
//! device did not record stays absent, and identities are version 5 UUIDs so that exporting the
//! same dive again yields the same uuid.

use alloc::vec::Vec;
use core::time::Duration;

use divecomputer_core::dive::{self as core_dive, Deco, EventKind, GasUsage};
use divecomputer_core::units::{Depth, Pressure, Temperature};
use divejson::jiff::tz::Offset;
use divejson::{
    Cylinder, CylinderRole, DateTime, DecoAlgorithm, Device, Document, Event, EventType, Generator,
    Position, PressureSeries, Profile, Recording, RecordingMode, Salinity, Series,
};

use crate::{Error, Result};

/// The version 5 UUID namespace of dives exported by this crate:
/// `uuid5(NAMESPACE_URL, "https://michidk.github.io/divecomputer-rs/ns/divecomputer")`.
///
/// A dive's uuid is `uuid5(DIVE_NAMESPACE, "dive:" + key)`, where `key` is the JSON array
/// `[brand, serial, device dive number, start]` of the core dive (`null` for an absent
/// member, the start as the DiveJSON `started_at` text). Fixed forever: changing it or the
/// key renumbers every document this crate has written.
pub const DIVE_NAMESPACE: uuid::Uuid = uuid::uuid!("997c2614-74f0-51ad-a673-e78a7e6fa08c");

/// Exports dives to a DiveJSON document.
///
/// Each core dive becomes one DiveJSON dive with one recording, in input order.
/// `exported_at` is the moment of export and must carry a UTC offset (the crate has no
/// clock); `generator` names the software writing the document.
///
/// # Errors
///
/// [`Error::MissingStart`] or [`Error::Start`] when a dive has no writable start, and
/// [`Error::DiveJson`] when the document breaks a DiveJSON rule, for example
/// `exported_at` without an offset or two dives with the same identity.
pub fn to_divejson(
    dives: &[core_dive::Dive],
    exported_at: DateTime,
    generator: Generator,
) -> Result<Document> {
    let mut document = Document::new(exported_at);
    document.generator = Some(generator);
    document.dives = dives
        .iter()
        .enumerate()
        .map(|(index, dive)| self::dive(index, dive))
        .collect::<Result<_>>()?;
    document
        .validate()
        .map_err(|violations| Error::DiveJson(divejson::Error::Invalid(violations)))?;
    Ok(document)
}

fn dive(index: usize, dive: &core_dive::Dive) -> Result<divejson::Dive> {
    let local = dive.start.ok_or(Error::MissingStart { index })?;
    // An offset DiveJSON cannot write (not whole minutes, out of range) is read as not
    // recorded rather than failing the dive.
    let started_at = dive
        .utc_offset
        .and_then(|seconds| Offset::from_seconds(seconds).ok())
        .and_then(|offset| DateTime::with_offset(local, offset).ok())
        .map_or_else(|| DateTime::from_local(local), Ok)
        .map_err(|source| Error::Start { index, source })?;

    let mut out = divejson::Dive::new(uuid(dive, &started_at).into(), started_at);
    // The device's dive figures seed the logbook entry (spec §6.2).
    out.submerged_time = dive
        .duration
        .map(|d| d.as_millis().saturating_add(500) / 1000)
        .and_then(|s| u64::try_from(s).ok())
        .filter(|&s| s > 0);
    out.max_depth = dive.max_depth.map(Depth::metres).filter(|&m| m > 0.0);
    out.avg_depth = dive
        .avg_depth
        .map(Depth::metres)
        .filter(|&m| m > 0.0 && out.max_depth.is_none_or(|max| m <= max));
    out.bottom_temperature = dive.min_temperature.map(Temperature::celsius);
    out.entry_position = dive
        .location
        .map(|l| Position::new(l.latitude, l.longitude));

    let gas = Gases::new(dive);
    let mut recording = Recording::default();
    recording.device = device(&dive.device, dive.number);
    recording.mode = dive.mode.map(|mode| match mode {
        core_dive::DiveMode::ClosedCircuit => RecordingMode::ClosedCircuit,
        core_dive::DiveMode::SemiClosed => RecordingMode::SemiClosed,
        core_dive::DiveMode::Gauge => RecordingMode::Gauge,
        core_dive::DiveMode::Freedive => RecordingMode::Freedive,
        _ => RecordingMode::OpenCircuit,
    });
    recording.deco_model = dive.deco_model.map(deco_model);
    recording.salinity = dive.salinity.map(|salinity| match salinity {
        core_dive::Salinity::Fresh => Salinity::Fresh,
        core_dive::Salinity::En13319 => Salinity::En13319,
        _ => Salinity::Salt,
    });
    recording.surface_pressure = dive
        .surface_pressure
        .map(Pressure::bar)
        .filter(|p| (0.4..=1.2).contains(p));
    recording.profile = profile(dive, &gas);
    out.cylinders = gas.into_cylinders(recording.profile.as_ref());
    if recording != Recording::default() {
        out.recordings.push(recording);
    }
    Ok(out)
}

fn uuid(dive: &core_dive::Dive, started_at: &DateTime) -> uuid::Uuid {
    let key = divejson::serde_json::json!([
        dive.device.brand,
        dive.device.serial,
        dive.number,
        started_at.as_str(),
    ]);
    uuid::Uuid::new_v5(&DIVE_NAMESPACE, alloc::format!("dive:{key}").as_bytes())
}

fn device(info: &core_dive::DeviceInfo, dive_number: Option<u32>) -> Option<Device> {
    let mut device = Device::default();
    device.brand.clone_from(&info.brand);
    device.model.clone_from(&info.model);
    device.serial.clone_from(&info.serial);
    device.firmware.clone_from(&info.firmware);
    device.dive_number = dive_number.map(u64::from);
    (device != Device::default()).then_some(device)
}

/// Names as the specification's Shearwater mappings write them; VPM has no `algorithm`.
fn deco_model(model: core_dive::DecoModel) -> divejson::DecoModel {
    let mut out = divejson::DecoModel::default();
    match model {
        core_dive::DecoModel::Buhlmann { gf_low, gf_high } => {
            out.algorithm = Some(DecoAlgorithm::Buhlmann);
            out.gf_low = Some(gf_low);
            out.gf_high = Some(gf_high);
        }
        core_dive::DecoModel::Vpm {
            conservatism,
            gfs_gradient_factor,
        } => {
            out.name = Some(
                if gfs_gradient_factor.is_some() {
                    "VPM-B/GFS"
                } else {
                    "VPM-B"
                }
                .into(),
            );
            out.conservatism = Some(i64::from(conservatism));
        }
        _ => out.name = Some("DCIEM".into()),
    }
    out
}

/// The dive's cylinders: one per gas mix (gas number = mix index), and one more for each
/// tank whose mix already has a tank or that names no mix.
struct Gases {
    cylinders: Vec<Cylinder>,
    /// For each core tank, the gas number of its cylinder.
    tanks: Vec<u32>,
    /// For each core gas mix, its gas number.
    mixes: Vec<u32>,
}

impl Gases {
    fn new(dive: &core_dive::Dive) -> Self {
        let mut cylinders: Vec<Cylinder> = dive
            .gas_mixes
            .iter()
            .map(|mix| {
                let mut cylinder = Cylinder::default();
                cylinder.oxygen = Some(mix.oxygen.percent());
                cylinder.helium = Some(mix.helium.percent());
                cylinder.role = role(mix.usage);
                cylinder
            })
            .collect();
        let mixes: Vec<u32> = (0..cylinders.len())
            .map_while(|i| u32::try_from(i).ok())
            .collect();
        let mut has_tank = alloc::vec![false; cylinders.len()];
        let mut tanks = Vec::new();
        for tank in &dive.tanks {
            let own = tank
                .gas_mix
                .filter(|&mix| has_tank.get(mix) == Some(&false));
            let index = if let Some(mix) = own {
                if let Some(flag) = has_tank.get_mut(mix) {
                    *flag = true;
                }
                mix
            } else {
                let mut cylinder = tank
                    .gas_mix
                    .and_then(|mix| cylinders.get(mix))
                    .cloned()
                    .unwrap_or_default();
                if cylinder.role.is_none() {
                    cylinder.role = role(tank.usage);
                }
                cylinders.push(cylinder);
                cylinders.len().saturating_sub(1)
            };
            if let Some(cylinder) = cylinders.get_mut(index) {
                // 0 is a device's absent-marker (converting.md, "Cylinders").
                cylinder.start_pressure = tank
                    .begin_pressure
                    .map(Pressure::bar)
                    .filter(|p| *p > 0.0 && *p <= 350.0);
                cylinder.end_pressure = tank
                    .end_pressure
                    .map(Pressure::bar)
                    .filter(|p| *p >= 0.0 && *p <= cylinder.start_pressure.unwrap_or(350.0));
            }
            tanks.push(u32::try_from(index).unwrap_or(u32::MAX));
        }
        Self {
            cylinders,
            tanks,
            mixes,
        }
    }

    /// The cylinders, numbered only where the profile refers to them (converting.md).
    fn into_cylinders(self, profile: Option<&Profile>) -> Vec<Cylinder> {
        let used = |number: u32| {
            profile.is_some_and(|p| {
                p.pressures.iter().any(|s| s.gas_number == number)
                    || p.events.iter().any(|e| e.gas_number == Some(number))
            })
        };
        self.cylinders
            .into_iter()
            .zip(0..)
            .map(|(mut cylinder, number)| {
                cylinder.gas_number = used(number).then_some(number);
                cylinder
            })
            .collect()
    }
}

fn role(usage: Option<GasUsage>) -> Option<CylinderRole> {
    match usage {
        Some(GasUsage::Diluent) => Some(CylinderRole::Diluent),
        Some(GasUsage::Oxygen) => Some(CylinderRole::Oxygen),
        _ => None,
    }
}

/// One channel being built: readings at strictly increasing times, the first reading of
/// a millisecond kept.
#[derive(Default)]
struct Channel {
    times: Vec<u64>,
    values: Vec<i64>,
}

impl Channel {
    fn push(&mut self, time: u64, value: Option<i64>) {
        if let Some(value) = value {
            if self.times.last().is_none_or(|&last| time > last) {
                self.times.push(time);
                self.values.push(value);
            }
        }
    }

    fn series(self) -> Option<Series> {
        (!self.times.is_empty()).then(|| Series::new(self.times, self.values))
    }
}

fn profile(dive: &core_dive::Dive, gas: &Gases) -> Option<Profile> {
    let mut depth = Channel::default();
    let mut temperature = Channel::default();
    let mut ppo2 = Channel::default();
    let mut cns = Channel::default();
    let mut ndl = Channel::default();
    let mut tts = Channel::default();
    let mut gradient_factor = Channel::default();
    let mut pressures: Vec<(u32, Channel)> = Vec::new();
    let floor = |v: Option<i64>| v.filter(|&v| v >= 0);
    for sample in &dive.samples {
        let t = millis(sample.time);
        depth.push(t, sample.depth.and_then(|d| scaled(d.metres(), 100.0)));
        temperature.push(
            t,
            sample.temperature.and_then(|c| scaled(c.celsius(), 100.0)),
        );
        ppo2.push(t, floor(sample.ppo2.and_then(|p| scaled(p.bar(), 100.0))));
        cns.push(t, floor(sample.cns.and_then(|c| scaled(c, 10.0))));
        // A stop's time is not a no-decompression time and has no member.
        if let Some(Deco::Ndl { time }) = sample.deco {
            ndl.push(t, i64::try_from(seconds(time)).ok());
        }
        tts.push(
            t,
            sample
                .time_to_surface
                .and_then(|d| i64::try_from(seconds(d)).ok()),
        );
        gradient_factor.push(t, sample.gradient_factor.map(i64::from));
        for reading in &sample.pressures {
            let Some(&number) = gas.tanks.get(reading.tank) else {
                continue;
            };
            let bar = reading.pressure.bar();
            // Zero is no reading; past 350 bar is out of the format's range.
            let value = scaled(bar, 10.0).filter(|_| bar > 0.0 && bar <= 350.0);
            if let Some((_, channel)) = pressures.iter_mut().find(|(n, _)| *n == number) {
                channel.push(t, value);
            } else {
                let mut channel = Channel::default();
                channel.push(t, value);
                pressures.push((number, channel));
            }
        }
    }

    let mut profile = Profile::new(dive.samples.iter().map(|s| millis(s.time)).max()?);
    profile.depth = depth.series();
    profile.temperature = temperature.series();
    profile.ppo2 = ppo2.series();
    profile.cns = cns.series();
    profile.ndl = ndl.series();
    profile.tts = tts.series();
    profile.gradient_factor = gradient_factor.series();
    profile.pressures = pressures
        .into_iter()
        .filter_map(|(number, channel)| {
            channel
                .series()
                .map(|s| PressureSeries::new(s.times, s.values, number))
        })
        .collect();
    profile.events = dive
        .events
        .iter()
        .filter_map(|event| {
            let time = millis(event.time);
            match event.kind {
                EventKind::GasSwitch { gas_mix } => {
                    let mut out = Event::new(time, EventType::GasSwitch);
                    out.gas_number = gas.mixes.get(gas_mix).copied();
                    Some(out)
                }
                EventKind::Bookmark { .. } => Some(Event::new(time, EventType::Bookmark)),
                // No common meaning and no device wording to carry as a label.
                _ => None,
            }
        })
        .collect();
    let has_channel = profile.channels().next().is_some() || !profile.pressures.is_empty();
    has_channel.then_some(profile)
}

/// Whole milliseconds, rounded half up.
fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos().saturating_add(500_000) / 1_000_000).unwrap_or(u64::MAX)
}

/// Whole seconds, rounded half up.
fn seconds(duration: Duration) -> u64 {
    millis(duration).saturating_add(500) / 1000
}

/// `value × scale` rounded half away from zero, `None` when not finite or out of range.
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "range checked before the cast; no_std has no f64::round"
)]
fn scaled(value: f64, scale: f64) -> Option<i64> {
    let x = value * scale;
    if !x.is_finite() || x.abs() >= 9.0e15 {
        return None;
    }
    let magnitude = (x.abs() + 0.5) as i64;
    Some(if x < 0.0 {
        magnitude.saturating_neg()
    } else {
        magnitude
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_is_derived_from_the_project_url() {
        assert_eq!(
            DIVE_NAMESPACE,
            uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_URL,
                b"https://michidk.github.io/divecomputer-rs/ns/divecomputer"
            )
        );
    }

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(scaled(2.6, 100.0), Some(260));
        assert_eq!(scaled(0.125, 100.0), Some(13));
        assert_eq!(scaled(-1.5, 1.0), Some(-2));
        assert_eq!(scaled(f64::NAN, 1.0), None);
        assert_eq!(millis(Duration::from_micros(2500)), 3);
        assert_eq!(seconds(Duration::from_millis(1499)), 1);
    }
}
