# Date and time encodings

Dive computers store dive start times in many ways. This document catalogues the encodings,
epochs and time-zone conventions, and defines how hosts convert them into the common model
([data-model.md](data-model.md)). Family documents state which encoding they use and at which
offsets; this document gives the shared rules.

## 1. Target representation

The common model stores a dive start time as:

- a **local wall-clock date-time** (year, month 1-12, day 1-31, hour 0-23, minute, second) as
  shown on the device, and
- an optional **UTC offset** in seconds, present only when the device records it.

Most devices keep only local time with no zone information. Hosts MUST NOT convert such a
value through the host's own time zone: a dive logged abroad would shift. Where a device
stores seconds since an epoch, those seconds are almost always **local time counted as if it
were UTC** ("floating" time); convert them with plain calendar arithmetic, no time-zone rules.

**Confidence:** likely.

## 2. Epochs

| Epoch (00:00:00) | Unix seconds of the epoch | Unit | Families |
|------------------|---------------------------|------|----------|
| 1970-01-01 | 0 | 1 s | Suunto EON Steel family (`u32le`), Shearwater (`u32be`), Oceans S1 (decimal text in the log), XDeep/Genz `SUMMARY.BIN` (`u32le`), Suunto EON Steel dive file names (hex) |
| 1989-12-31 | 631 065 600 | 1 s | Garmin FIT (`date_time`, UTC; `local_date_time` local) |
| 1992-01-01 05:00 UTC | 694 242 000 | 1 s | Cochran (newer models, `u32le`) |
| 2000-01-01 | 946 684 800 | 1 s | Divesoft Freedom/Liberty (`u32le`), McLean Extreme (`u32le`), Divesoft `.DLF` files |
| 2000-01-01 | 946 684 800 | 0.5 s | Uwatec Smart family (`u32le` half-seconds) |
| 2008-01-01 | 1 199 145 600 | 1 s | Ratio / DiveSystem iDive and iX3M (`u32le`) |
| 2021-01-01 | 1 609 459 200 | 1 s | Halcyon Symbios |

Conversion: `local_datetime = calendar(epoch_seconds + raw × unit)`, using a proleptic
Gregorian calendar with no leap seconds.

Notes:

- The Cochran constant is exactly 1992-01-01 00:00 in US Eastern Standard Time; one reference
  treats the result as a UTC instant and converts it to the host's zone. Whether the device
  counts local time or US Eastern time is an open question.
- Garmin FIT `timestamp` fields are UTC; FIT activity messages also carry a `local_timestamp`,
  whose difference from `timestamp` gives the UTC offset.
- Shearwater, Divesoft and McLean timestamps are local-as-UTC per section 1; Shearwater Teric
  and Divesoft v2 headers additionally carry an offset (section 5).

**Confidence:** likely (Garmin: verified, FIT SDK).

## 3. Calendar fields stored directly

Binary (not BCD) fields, one value each. "y8+2000" means a `u8` year offset from 2000;
"y16" a full `u16` year. Month and day are 1-based unless marked "0-based".

| Family | Layout (in storage order) |
|--------|---------------------------|
| Atomic Cobalt | y16le, month, day, hour, minute |
| Cressi Leonardo | y8+2000, month, day, hour, minute |
| Cressi Goa | y16le, month, day, hour, minute |
| Deepblu Cosmiq | y16le, **day, month, minute, hour** |
| Deep Six Excursion | y8+2000, month, day, hour, minute, second, then zone byte (section 5) |
| Dive Rite NiTek Q | y8+2000, month, day, hour, minute, second |
| HW OSTC (newer logs, version `0x23`/`0x24`) | y8+2000, month, day, hour, minute |
| HW OSTC (older logs) | **month, day, y8+2000**, hour, minute |
| Liquivision | `u16le` fields: second, minute, hour, (unused), day 0-based, month 0-based, year y16 (family document gives offsets) |
| Mares Darwin | y16**be**, month, day, hour, minute |
| Mares Icon HD (non-Genius) | `u16le` fields: hour, minute, day, month **0-based**, year − 1900 |
| Mares Nemo / Puck | y8+2000, month, day, hour, minute |
| Seac Screen | `u8` fields at fixed offsets: hour, minute, second, day, month, y8+2000, in UTC, plus a zone index (section 5) |
| Sporasub SP2 | day, month, y8+2000, second, minute, hour |
| Suunto D9 family (older) | hour, minute, second, y16le, month, day |
| Suunto D9 family (D4i, D6i, D9tx, DX, Zoop Novo, Vyper Novo, D4f) | y16le, month, day, hour, minute, second |
| Suunto Vyper family, Suunto Spyder | y8 with pivot: `< 90` → 2000 + y, else 1900 + y; month, day, hour, minute |
| Tecdiving | y8+2000, month, day, hour, minute, second |

When seconds are absent they are 0.

HW OSTC version `0x24` logs store the **end** of the dive; the start is obtained by
subtracting the dive duration.

**Confidence:** likely.

## 4. BCD and bit-packed fields

### 4.1 BCD

`bcd8` stores two decimal digits, one per nibble (`0x59` = 59).

| Family | Notes |
|--------|-------|
| Citizen Aqualand | century and year as two `bcd8` (`20`, `24` → 2024), then month, day, one byte not used for the date, hour, minute, second |
| Suunto Eon / Solution | `bcd8` year with pivot `< 85` → 2000s, else 1900s; month, day, hour, minute |
| Oceanic / Pelagic (Atom 2, VT Pro, Veo 250 generations) | minute and hour in `bcd8` with masks; **12-hour clock** with a PM flag bit (often bit 7 of the hour byte); day/month/year partly BCD and partly binary nibbles spread over several bytes, differing per model group. Hosts convert: `hour24 = (hour12 mod 12) + (PM ? 12 : 0)`. Some models need a fixed year correction (e.g. +2 or +3) because the year nibble wraps. Details per model in the Pelagic family document. |

### 4.2 Bit-packed

| Family | Encoding |
|--------|----------|
| Mares Genius-class (Genius, Sirius, Quad Ci, Puck 4, ...) | `u32le`: bits 0-4 hour, bits 5-10 minute, bits 11-15 day, bits 16-19 month, bits 20-31 year (full) |
| Oceanic Veo 250 family | year nibbles spread over two bytes plus a model-dependent offset; see Pelagic family document |

Worked example (Mares Genius): 2024-05-17 14:30 →
`2024<<20 | 5<<16 | 17<<11 | 30<<5 | 14` = `0x7E858BCE`, stored `CE 8B 85 7E`.

**Confidence:** likely.

## 5. Time zones

Most devices store no zone. Where they do:

| Family | Encoding | Meaning |
|--------|----------|---------|
| Uwatec Smart (models with a zone byte) | `i8` × 15 min | UTC offset; local time = epoch time + offset |
| Deep Six Excursion, header v0, firmware ≥ 5.B | `u8` − 12, hours | UTC offset |
| Deep Six Excursion, header v1 | `i8` × 15 min | UTC offset |
| Divesoft (header v2) | `i16le` minutes | UTC offset; local = stored + offset |
| Halcyon Symbios (log version ≥ 1.9) | `i8` hours | UTC offset; local = stored + offset |
| Shearwater Teric (log version ≥ 9) | `i32be` minutes + `u8` DST flag (adds 1 h) | UTC offset of the stored time |
| Ratio / DiveSystem (APOS 4 firmware) | zone **index** (even values; entry = index / 2) into the table below | UTC offset |
| Seac Screen | zone index into the table below; time fields are UTC | local = UTC + offset |

Zone table shared by Ratio and Seac (42 entries, index 0 first; the Ratio byte is twice the
index):

```
-12:00 -11:00 -10:00 -09:30 -09:00 -08:00 -07:00 -06:00 -05:00 -04:30
-04:00 -03:30 -03:00 -02:00 -01:00 +00:00 +01:00 +02:00 +03:00 +03:30
+04:00 +04:30 +05:00 +05:30 +05:45 +06:00 +06:30 +07:00 +08:00 +08:45
+09:00 +09:30 +09:45 +10:00 +10:30 +11:00 +11:30 +12:00 +12:45 +13:00
+13:45 +14:00
```

For the families where the stored value is "UTC" plus an offset, there are two conventions:
the stored seconds are true UTC and the offset gives local time (Seac, Uwatec, Divesoft,
Halcyon as interpreted by one reference), or the stored seconds are already local (Shearwater
Teric). The family documents say which.

**Confidence:** likely (offset sign conventions uncertain for Deep Six v0 and Halcyon).

## 6. Device-relative clocks

Some devices have no calendar clock, only a free-running counter. Dive times are counter
values; the host reads the counter's current value at download time and correlates:

```
dive_time_host = host_now + (dive_ticks − device_now) × tick
```

where `host_now` is the host's local time at the moment `device_now` was read, and the result
is local wall-clock time.

| Family | Counter | Tick |
|--------|---------|------|
| ReefNet Sensus, Sensus Pro, Sensus Ultra | `u32le` from the handshake | 1 s |
| Uwatec Aladin / Memomouse | `u32le` | 0.5 s |
| Uwatec Smart Pro, Smart Com, Smart Tec, Smart Z (model codes `0x10`, `0x14`, `0x18`, `0x1C`) | `u32le` from command `1A` | 0.5 s |

Hosts MUST record `(device_now, host_now)` together with the dump, so a dump can be re-parsed
later with the same result. Accuracy is limited by the device's clock drift since the dive.

For incremental download on these devices the "newer than" threshold is expressed in device
ticks: `threshold_ticks = device_now − (host_now − last_dive_host_time) / tick`. Devices with an
absolute half-second clock (other Uwatec Smart models) instead use
`threshold = (last_dive_local_seconds_since_2000) × 2`.

**Confidence:** likely.

## 7. Durations and sample times

Durations use the same diversity (seconds, minutes, `bcd8` minutes and seconds, minutes +
seconds byte); they are family-specific and are converted to seconds. Sample times are
almost always implicit (`index × sample interval`).

## Open questions

- Cochran epoch: local time or fixed US Eastern?
- Sign/offset conventions of Deep Six v0 zone byte (`− 12`) on devices west of UTC.
- Whether Seac's stored fields are truly UTC on all firmware versions.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Garmin FIT SDK, `date_time` base 1989-12-31T00:00:00Z, https://developer.garmin.com/fit/protocol/
