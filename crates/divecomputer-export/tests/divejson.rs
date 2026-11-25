//! DiveJSON export of parsed and hand-built dives.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    missing_docs
)]

use core::time::Duration;

use divecomputer_core::dive::{
    Deco, DecoModel, Dive, DiveMode, Event, EventKind, GasMix, GasUsage, Sample, Tank, TankPressure,
};
use divecomputer_core::units::{Depth, GasFraction, Pressure, Temperature};
use divecomputer_export::to_divejson;
use divejson::{CylinderRole, DateTime, EventType, Generator, RecordingMode, Salinity};

/// A 32-byte native record of type `kind` with `fields` at their record offsets.
fn record(kind: u8, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut record = vec![0; 32];
    record[0] = kind;
    for (offset, bytes) in fields {
        record[*offset..offset + bytes.len()].copy_from_slice(bytes);
    }
    record
}

/// The native-format worked example from the Shearwater spec (Perdix 2, imperial, fresh
/// water), as the protocol crate's parser tests build it.
fn shearwater_example() -> Vec<u8> {
    let opening0 = [
        0xFF, 0x00, 0x02, 0x28, 0x55, 0x0B, 0x88, 0x01, 0x0F, 0x00, 0x00, 0x6A, 0x47, 0xA0, 0x8D,
        0x00, 0x46, 0x82, 0xA2, 0x15,
    ];
    let sample = [
        0x00, 0x37, 0x00, 0x00, 0x00, 0x01, 0x14, 0x15, 0x00, 0x00, 0x58, 0x12, 0x00, 0x41,
    ];
    let closing0 = [
        0xFE, 0x00, 0x02, 0x00, 0x42, 0x00, 0x00, 0x1C, 0x0F, 0x00, 0x00, 0x6A, 0x47, 0xA0, 0xA4,
    ];
    [
        record(0x10, &[(1, &opening0)]),
        record(0x11, &[(16, &[0x03, 0x31])]),
        record(0x12, &[]),
        record(0x13, &[(1, &[0x0B, 0x03, 0x03, 0xE8])]),
        record(0x14, &[(1, &[0x06]), (16, &[0x11, 0x00, 0x01])]),
        record(0x15, &[(23, &[0x27, 0x10])]),
        record(0x01, &[(1, &sample)]),
        record(0x20, &[(1, &closing0)]),
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

fn exported_at() -> DateTime {
    "2026-11-25T12:00:00+01:00".parse().unwrap()
}

fn generator() -> Generator {
    let mut generator = Generator::new("divecomputer-rs");
    generator.version = Some("0.0.0".into());
    generator
}

#[test]
fn exports_the_shearwater_worked_example() {
    let dive = divecomputer_protocol::shearwater::parse(&shearwater_example()).unwrap();
    let document = to_divejson(&[dive], exported_at(), generator()).unwrap();
    let text = divejson::to_string_pretty(&document).unwrap();

    // The written text reads back as a valid document.
    assert_eq!(divejson::from_str(&text).unwrap(), document);
    let dive = &document.dives[0];
    assert_eq!(dive.started_at.to_string(), "2026-07-03T11:44:13");
    assert_eq!(dive.submerged_time, Some(28));
    let recording = &dive.recordings[0];
    let device = recording.device.as_ref().unwrap();
    assert_eq!(device.brand.as_deref(), Some("Shearwater"));
    assert_eq!(device.dive_number, Some(2));
    assert_eq!(recording.mode, Some(RecordingMode::OpenCircuit));
    assert_eq!(recording.salinity, Some(Salinity::Fresh));
    let profile = recording.profile.as_ref().unwrap();
    // 5.5 ft is 1.6764 m.
    assert_eq!(profile.depth.as_ref().unwrap().values, [168]);
    // The first sample's gas is the starting gas, not a switch.
    assert!(profile.events.is_empty());
    assert_eq!(dive.cylinders[0].oxygen, Some(21.0));

    insta::assert_snapshot!(text, @r#"
    {
      "format": "divejson",
      "version": "1.0",
      "exported_at": "2026-11-25T12:00:00+01:00",
      "generator": {
        "name": "divecomputer-rs",
        "version": "0.0.0"
      },
      "dives": [
        {
          "uuid": "e80811f4-77c4-5665-8fbf-f7939f2e193f",
          "started_at": "2026-07-03T11:44:13",
          "submerged_time": 28,
          "max_depth": 2.01168,
          "cylinders": [
            {
              "oxygen": 21.0,
              "helium": 0.0
            }
          ],
          "recordings": [
            {
              "device": {
                "brand": "Shearwater",
                "model": "Perdix 2",
                "serial": "1234ABCD",
                "firmware": "92",
                "dive_number": 2
              },
              "mode": "open_circuit",
              "deco_model": {
                "algorithm": "buhlmann",
                "gf_low": 40,
                "gf_high": 85
              },
              "salinity": "fresh",
              "surface_pressure": 0.817,
              "profile": {
                "duration": 10000,
                "depth": {
                  "times": [
                    10000
                  ],
                  "values": [
                    168
                  ]
                },
                "temperature": {
                  "times": [
                    10000
                  ],
                  "values": [
                    1833
                  ]
                },
                "tts": {
                  "times": [
                    10000
                  ],
                  "values": [
                    60
                  ]
                },
                "cns": {
                  "times": [
                    10000
                  ],
                  "values": [
                    0
                  ]
                },
                "gradient_factor": {
                  "times": [
                    10000
                  ],
                  "values": [
                    0
                  ]
                }
              }
            }
          ]
        }
      ]
    }
    "#);
}

/// A closed-circuit dive with a known time zone, two tanks and a bookmark.
fn rebreather_dive() -> Dive {
    let mut dive = Dive::default();
    dive.start = Some(jiff_date_time());
    dive.utc_offset = Some(2 * 3600);
    dive.number = Some(7);
    dive.duration = Some(Duration::from_secs(1800));
    dive.max_depth = Some(Depth::from_metres(30.25));
    dive.avg_depth = Some(Depth::from_metres(18.0));
    dive.min_temperature = Some(Temperature::from_celsius(11.5));
    dive.mode = Some(DiveMode::ClosedCircuit);
    dive.surface_pressure = Some(Pressure::from_millibar(1013.0));
    dive.deco_model = Some(DecoModel::Vpm {
        conservatism: 2,
        gfs_gradient_factor: Some(90),
    });
    dive.device.brand = Some("Shearwater".into());
    dive.device.serial = Some("1234ABCD".into());
    dive.gas_mixes = vec![
        GasMix::new(
            GasFraction::from_percent(21.0),
            GasFraction::from_percent(35.0),
            Some(GasUsage::Diluent),
        ),
        GasMix::new(
            GasFraction::from_percent(100.0),
            GasFraction::from_percent(0.0),
            Some(GasUsage::Oxygen),
        ),
        GasMix::new(
            GasFraction::from_percent(50.0),
            GasFraction::from_percent(0.0),
            None,
        ),
    ];
    let mut oxygen = Tank::default();
    oxygen.gas_mix = Some(1);
    oxygen.begin_pressure = Some(Pressure::from_bar(200.0));
    oxygen.end_pressure = Some(Pressure::from_bar(150.0));
    let mut unknown = Tank::default();
    unknown.begin_pressure = Some(Pressure::from_bar(0.0));
    dive.tanks = vec![oxygen, unknown];
    dive.samples = (0..4u32)
        .map(|i| {
            let mut sample = Sample::default();
            sample.time = Duration::from_secs(u64::from(i) * 10);
            sample.depth = Some(Depth::from_metres(f64::from(i) * 2.6));
            sample.pressures = vec![TankPressure::new(
                0,
                Pressure::from_bar(200.0 - f64::from(i)),
            )];
            if i == 2 {
                // A second tank reading, and a reading of zero that is no reading.
                sample
                    .pressures
                    .push(TankPressure::new(1, Pressure::from_bar(180.0)));
                sample
                    .pressures
                    .push(TankPressure::new(0, Pressure::from_bar(0.0)));
            }
            sample.deco = Some(if i < 3 {
                Deco::Ndl {
                    time: Duration::from_secs(600),
                }
            } else {
                Deco::Stop {
                    depth: Depth::from_metres(6.0),
                    time: Duration::from_secs(60),
                }
            });
            sample.gradient_factor = Some(40);
            sample
        })
        .collect();
    dive.events = vec![
        Event::new(Duration::from_secs(10), EventKind::GasSwitch { gas_mix: 0 }),
        Event::new(
            Duration::from_secs(20),
            EventKind::Bookmark { heading: None },
        ),
        Event::new(
            Duration::from_secs(25),
            EventKind::Vendor { code: 1, value: 2 },
        ),
    ];
    dive
}

fn jiff_date_time() -> divejson::jiff::civil::DateTime {
    divejson::jiff::civil::date(2026, 8, 1).at(9, 30, 0, 0)
}

#[test]
fn exports_a_hand_built_rebreather_dive() {
    let dive = rebreather_dive();
    let document = to_divejson(core::slice::from_ref(&dive), exported_at(), generator()).unwrap();
    let out = &document.dives[0];
    assert_eq!(out.started_at.to_string(), "2026-08-01T09:30:00+02:00");
    assert_eq!(out.max_depth, Some(30.25));
    assert_eq!(out.bottom_temperature, Some(11.5));

    let recording = &out.recordings[0];
    assert_eq!(recording.mode, Some(RecordingMode::ClosedCircuit));
    let deco = recording.deco_model.as_ref().unwrap();
    assert_eq!(deco.name.as_deref(), Some("VPM-B/GFS"));
    assert_eq!(deco.algorithm, None);
    assert_eq!(recording.surface_pressure, Some(1.013));

    let profile = recording.profile.as_ref().unwrap();
    assert_eq!(profile.duration, 30_000);
    let depth = profile.depth.as_ref().unwrap();
    assert_eq!(depth.times, [0, 10_000, 20_000, 30_000]);
    assert_eq!(depth.values, [0, 260, 520, 780]);
    // The stop at 30 s is a gap in `ndl`, not a reading.
    assert_eq!(profile.ndl.as_ref().unwrap().values, [600, 600, 600]);
    assert_eq!(profile.pressures.len(), 2);
    assert_eq!(profile.pressures[0].gas_number, 1, "the oxygen tank's mix");
    assert_eq!(profile.pressures[0].values, [2000, 1990, 1980, 1970]);
    assert_eq!(profile.pressures[1].gas_number, 3, "a tank with no mix");
    let kinds: Vec<_> = profile.events.iter().map(|e| e.event_type).collect();
    assert_eq!(
        kinds,
        [Some(EventType::GasSwitch), Some(EventType::Bookmark)]
    );

    // Mixes 0 and 1 are referenced, mix 2 is not; the unknown tank is a cylinder of its own.
    let numbers: Vec<_> = out.cylinders.iter().map(|c| c.gas_number).collect();
    assert_eq!(numbers, [Some(0), Some(1), None, Some(3)]);
    assert_eq!(out.cylinders[0].role, Some(CylinderRole::Diluent));
    assert_eq!(out.cylinders[1].start_pressure, Some(200.0));
    assert_eq!(out.cylinders[1].end_pressure, Some(150.0));
    assert_eq!(out.cylinders[3].start_pressure, None, "0 bar is no reading");
    assert_eq!(out.cylinders[3].oxygen, None, "nothing invented");

    // Exporting again yields the same identity; another dive gets another.
    let again = to_divejson(core::slice::from_ref(&dive), exported_at(), generator()).unwrap();
    assert_eq!(again.dives[0].uuid, out.uuid);
    let mut other = dive;
    other.number = Some(8);
    let both = to_divejson(&[rebreather_dive(), other], exported_at(), generator()).unwrap();
    assert_ne!(both.dives[0].uuid, both.dives[1].uuid);
}

#[test]
fn reports_what_divejson_cannot_hold() {
    let error = to_divejson(&[Dive::default()], exported_at(), generator()).unwrap_err();
    assert!(matches!(
        error,
        divecomputer_export::Error::MissingStart { index: 0 }
    ));

    let local = "2026-11-25T12:00:00".parse().unwrap();
    let error = to_divejson(&[], local, generator()).unwrap_err();
    assert!(matches!(error, divecomputer_export::Error::DiveJson(_)));

    // Two exports of one dive in one document collide on purpose.
    let dive = rebreather_dive();
    let error = to_divejson(&[dive.clone(), dive], exported_at(), generator()).unwrap_err();
    assert!(matches!(error, divecomputer_export::Error::DiveJson(_)));
}

#[test]
fn a_dive_without_samples_keeps_its_device() {
    let mut dive = Dive::default();
    dive.start = Some(jiff_date_time());
    let document = to_divejson(core::slice::from_ref(&dive), exported_at(), generator()).unwrap();
    assert!(document.dives[0].recordings.is_empty());

    dive.device.model = Some("Teric".into());
    let document = to_divejson(&[dive], exported_at(), generator()).unwrap();
    let recording = &document.dives[0].recordings[0];
    assert!(recording.profile.is_none());
    assert_eq!(
        recording.device.as_ref().unwrap().model.as_deref(),
        Some("Teric")
    );
}
