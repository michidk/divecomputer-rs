# Shearwater dive log format

This document describes the dive records downloaded by the procedure in
[shearwater.md](shearwater.md). There are two encodings:

- **Block format** ("Predator-like"): a 128-byte opening block, fixed-size samples, a
  128-byte closing block, and (usually) a 128-byte final block. The Predator always
  uses this format. Petrel-family computers return it when dives are read through base
  address `0xC0000000`.
- **Native format** (PNF, "Petrel Native Format"): a sequence of 32-byte records,
  each tagged by its first byte. Petrel-family computers return it through base
  `0x80000000`. Shearwater's desktop software also stores logs in this format.

All multi-byte integers are big-endian unless stated otherwise. Offsets are zero-based.

## Telling the formats apart

| Test | Result |
|------|--------|
| Data came from the Predator full dump | block format, 16-byte samples |
| Petrel family, first two bytes are `FF FF` | block format, 32-byte samples |
| Petrel family, first two bytes are anything else | native format (first byte is a record type, normally `10`) |

Confidence: **likely**.

## Units

The units byte (opening offset 8, both formats) selects the units for depths and
temperatures:

| Value | Depth | Temperature |
|------:|-------|-------------|
| 0 | metres | °C |
| 1 | feet (× 0.3048 → m) | °F (`(F − 32) × 5/9` → °C) |

Tank pressures are always in units of 2 psi (× 2 × 0.0689475729 → bar), whatever the
units setting. Atmospheric pressure is always in mbar, density in kg/m³.

## Block format

A dive is `opening block | samples… | closing block | final block`.

- Opening block: 128 bytes, starts `FF FF`.
- Samples: 16 bytes each on the Predator, 32 bytes on Petrel-family computers. A
  sample that is all zero bytes is padding and is skipped.
- Closing block: 128 bytes, starts `FF FE`.
- Final block: 128 bytes, starts `FF FD`. Petrel-family downloads always include it.
  On Predator data it is present when the last block starts `FF FD`. Downloaders
  append the Predator's identity block as this final block.

The sample interval is fixed at 10 s.

### Opening block (128 bytes)

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 0x00 | `u16be` | — | `FFFF` marker |
| 0x02 | `u16be` | — | dive number (internal counter) |
| 0x04 | `u8` | % | gradient factor low |
| 0x05 | `u8` | % | gradient factor high |
| 0x06 | `u16be` | min (uncertain) | surface interval before the dive |
| 0x08 | `u8` | — | units, see [Units](#units) |
| 0x09 | `u8` | 0.1 V | battery voltage (uncertain) |
| 0x0C | `u32be` | s | dive start, seconds since 1970-01-01 on the local wall clock (fingerprint) |
| 0x11 | `u8` | 0.01 bar | low setpoint (Predator) |
| 0x12 | `u8` | 0.01 bar | high setpoint (Predator) |
| 0x13 | `bcd8` | — | firmware version (uncertain) |
| 0x14–0x1D | 10 × `u8` | % | O₂ of gases 1–10. Gases 1–5 are open circuit, 6–10 diluents |
| 0x1E–0x27 | 10 × `u8` | % | He of gases 1–10 |
| 0x2F | `u16be` | mbar | surface (atmospheric) pressure |
| 0x43 | `u8` | — | decompression model, see [Decompression model](#decompression-model) |
| 0x44 | `u8` | — | VPM-B conservatism |
| 0x53 | `u16be` | kg/m³ | water density (1000 = fresh) |
| 0x55 | `u8` | % | VPM-B/GFS gradient factor |
| 0x56 | `u8` | bit mask | O₂ sensors calibrated (bit 0 = cell 1) |
| 0x57, 0x59, 0x5B | 3 × `u16be` | 1/100000 bar/mV | calibration of cells 1–3 |
| 0x70 | `u8` | — | dive mode (log version ≥ 8), see [Dive mode](#dive-mode) |
| 0x78 | `u8` | — | battery type (log version ≥ 7), see [Battery type](#battery-type) |
| 0x7F | `u8` | — | log version |

In the block format, every gas is treated as enabled, and air integration is read
from the samples whenever the log version is ≥ 7.

### Closing block (128 bytes)

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 0x00 | `u16be` | — | `FFFE` marker |
| 0x02 | `u16be` | — | dive number, equal to the opening block's |
| 0x04 | `u16be` | m or ft, **whole units** | maximum depth |
| 0x06 | `u16be` | min | dive time |

### Final block (128 bytes)

| Offset | Type | Meaning |
|-------:|------|---------|
| 0x00 | `u16be` | `FFFD` marker |
| 0x02 | `u32be` | serial number |
| 0x0A | `bcd8` | firmware version |
| 0x0D | `u8` | model code (see [shearwater.md](shearwater.md#devices)) |

Confidence: **likely** for fields with no remark. Fields marked uncertain come from
one source and have no corroboration.

## Native format (PNF)

A dive is a sequence of 32-byte records. Byte 0 is the record type. An all-zero record
is padding. Record order is: opening records, then samples interleaved with events and
other records, then closing records, then the final record.

### Record types

| Type | Record | Decoded below |
|------|--------|---------------|
| `00` | padding (whole record zero) | — |
| `01` | dive sample | [Dive sample](#dive-sample) |
| `02` | freedive samples (4 per record) | [Freedive record](#freedive-record-02) |
| `03` | Avelo sample | [Dive sample](#dive-sample) |
| `10`–`19` | opening records 0–9 | [Opening and closing records](#opening-and-closing-records) |
| `20`–`29` | closing records 0–9 | same |
| `30` | info event | [Info event](#info-event-30) |
| `40` | DiveCAN event | layout unknown |
| `50` | button press | layout unknown |
| `60` | surface ppO₂ | layout unknown |
| `70`–`75` | tissue-loading snapshot | [Tissue snapshot](#tissue-snapshot-7075) |
| `80`–`87` | opening-record validity maps | layout unknown |
| `88` | dive-sample validity map | layout unknown |
| `E1` | extended dive sample | [Extended sample](#extended-sample-e1) |
| `FF` | final record | [Final record](#final-record-ff) |

Records 0–4 (opening and closing) are always present. A log without them is invalid.
Records 5 and up depend on the log version and the model. Petrel and Petrel 2 on
native-format firmware (from about v71) may omit record 5.

Decoders SHOULD keep unknown records verbatim.

Confidence: types `01`–`03`, `10`–`29`, `30`, `E1` and `FF` **likely** (two independent
sources). The names of `40`–`88` are **uncertain** (one source, which in turn cites
the vendor's desktop parser).

### Opening and closing records

Offsets include the type byte at 0. Fields not listed are unknown.

**Opening 0 (`10`)**

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 1 | `u8` | — | `FF` on observed logs (remnant of the block-format marker) |
| 2 | `u16be` | — | dive number |
| 4 | `u8` | % | gradient factor low |
| 5 | `u8` | % | gradient factor high |
| 6 | `u16be` | min (uncertain) | surface interval |
| 8 | `u8` | — | units |
| 9 | `u8` | 0.1 V | battery voltage (uncertain) |
| 12 | `u32be` | s | dive start, local wall clock, seconds since 1970 (fingerprint) |
| 20–29 | 10 × `u8` | % | O₂ of gases 1–10 (1–5 open circuit, 6–10 diluent) |
| 30–31 | 2 × `u8` | % | He of gases 1–2 |

**Opening 1 (`11`)**

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 1–8 | 8 × `u8` | % | He of gases 3–10 |
| 16 | `u16be` | mbar | surface (atmospheric) pressure |

**Opening 2 (`12`)**

| Offset | Type | Meaning |
|-------:|------|---------|
| 18 | `u8` | decompression model |
| 19 | `u8` | VPM-B conservatism |

**Opening 3 (`13`)**

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 3 | `u16be` | kg/m³ | water density |
| 5 | `u8` | % | VPM-B/GFS gradient factor |
| 6 | `u8` | bit mask | O₂ cells calibrated (bit 0 = cell 1) |
| 7, 9, 11 | 3 × `u16be` | 1/100000 bar/mV | calibration of cells 1–3 |

**Opening 4 (`14`)**

| Offset | Type | Version | Meaning |
|-------:|------|---------|---------|
| 1 | `u8` | ≥ 8 | dive mode |
| 9 | `u8` | ≥ 7 | battery type |
| 16 | `u8` | all | log version |
| 17 | `u16be` | all | gas enabled mask, bit *i* = gas *i*+1 |
| 28 | `u8` | ≥ 7 | air-integration mode, see [Air integration](#air-integration) |
| 29 | `u8` | ≥ 7 | gas-time-remaining transmitter mask, bit *i* = transmitter *i*+1. Two or more bits set means sidemount |

**Opening 5 (`15`)**, log version ≥ 9

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 1 | 3 × `bcd8` | — | transmitter 1 serial (6 decimal digits) |
| 6 | `u16be` | uncertain | transmitter 1 rated pressure |
| 8 | `u16be` | uncertain | transmitter 1 reserve pressure |
| 10 | 3 × `bcd8` | — | transmitter 2 serial |
| 15 | `u16be` | uncertain | transmitter 2 rated pressure |
| 17 | `u16be` | uncertain | transmitter 2 reserve pressure |
| 23 | `u16be` | ms | sample interval (default 10000 if the record is absent) |
| 26 | `i32be` | min | UTC offset |
| 30 | `u8` | h | daylight-saving offset |

**Opening 6 (`16`)**

| Offset | Type | Version | Meaning |
|-------:|------|---------|---------|
| 1 | `u16be` | ≥ 11 | configured scrubber (stack) time, s. 0 = feature off |
| 3 | `u16be` | ≥ 11 | remaining stack time at dive start, s |
| 5 | `u8` | ≥ 12 | recreational OC sub-mode (only when dive mode = 6): 0 three-gas nitrox, 1 air, 2 nitrox, 3 OC recreational |
| 19 | `u8` | ≥ 13 | transmitter 1 enabled |
| 20 | 2 × ASCII | ≥ 13 | transmitter 1 name (first character `O` = oxygen, `D` = diluent on CCR) |
| 22 | `u8` | ≥ 13 | transmitter 2 enabled |
| 23 | 2 × ASCII | ≥ 13 | transmitter 2 name |
| 25 | 3 × `bcd8` | ≥ 13 | transmitter 3 serial |
| 28 | `u16be` | ≥ 13 | transmitter 3 rated pressure |
| 30 | `u16be` | ≥ 13 | transmitter 3 reserve pressure |

**Opening 7 (`17`)**, log version ≥ 13

| Offset | Type | Meaning |
|-------:|------|---------|
| 1 | `u8` | transmitter 3 enabled |
| 2 | 2 × ASCII | transmitter 3 name |
| 4 | 3 × `bcd8` | transmitter 4 serial |
| 7 | `u16be` | transmitter 4 rated pressure |
| 9 | `u16be` | transmitter 4 reserve pressure |
| 11 | `u8` | transmitter 4 enabled |
| 12 | 2 × ASCII | transmitter 4 name |

**Opening 8 (`18`)**: layout unknown.

**Opening 9 (`19`)** and **closing 9 (`29`)**, log version ≥ 17: surface position at
dive start and end.

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 16 | `u8` | — | fix status: 0 no satellites, 1 no fix, 2 2D fix, 3 3D fix, `10` disabled, `FF` unknown |
| 21 | `i32be` | 1e-5 degree | latitude |
| 25 | `i32be` | 1e-5 degree | longitude |

Only use the position when the status is 2 or 3.

**Closing 0 (`20`)**

| Offset | Type | Raw unit | Meaning |
|-------:|------|----------|---------|
| 1 | `u8` | — | `FE` on observed logs |
| 2 | `u16be` | — | dive number |
| 4 | `u16be` | 0.1 m or 0.1 ft | maximum depth (note: tenths, unlike the block format) |
| 6 | `u24be` | s | dive time |
| 9 | `u8` | 0.1 V | battery voltage at end (uncertain) |
| 12 | `u32be` | s | end time, local wall clock (uncertain) |

Another implementation reads the dive time as a `u16be` at offset 7. That is the same
value whenever the dive is shorter than 65536 s.

**Closing 6 (`26`)**, log version ≥ 11: offset 3, `u16be`, remaining stack time at
dive end, in seconds.

Closing records 1–5, 7 and 8 mirror the opening records with end-of-dive values. Their
layouts have not been confirmed.

### Final record (`FF`)

| Offset | Type | Meaning |
|-------:|------|---------|
| 1 | `u8` | `FD` |
| 2 | `u32be` | serial number |
| 10 | `bcd8` | firmware version |
| 13 | `u8` | model code. Prefer this over a model code supplied by the caller |

Native logs read through base `0x80000000` end with this record. Logs from the early
native format (base `0x90000000`) lack it. The vendor's desktop software treats the
final record plus the three records after it (all zero) as one 128-byte unit.

Confidence: **likely**, except for fields marked uncertain.

## Dive sample

Both formats use the same sample layout. The table gives offsets relative to the start
of the sample fields:

- block format: sample start;
- native format: record start + 1 (after the type byte).

The Predator's 16-byte sample has only offsets 0–15. Petrel-family samples have 0–30
(block format: 32 bytes, last byte unknown; native: 31 bytes after the type byte).

| Off | Type | Raw unit | Meaning |
|----:|------|----------|---------|
| 0 | `u16be` | 0.1 m / 0.1 ft | depth |
| 2 | `u16be` | m / ft | next stop depth. 0 = no decompression obligation |
| 4 | `u16be` | min | time to surface |
| 6 | `u8` | 0.01 bar | ppO₂ (voted / average) |
| 7 | `u8` | % | current gas O₂ |
| 8 | `u8` | % | current gas He |
| 9 | `u8` | min | stop time when off 2 ≠ 0, else no-decompression limit |
| 10 | `u8` | % | battery remaining (uncertain) |
| 11 | `u8` | bits | status, see below |
| 12 | `u8` | mV | O₂ cell 1 |
| 13 | `i8` | °C / °F | water temperature (see below) |
| 14 | `u8` | mV | O₂ cell 2 |
| 15 | `u8` | mV | O₂ cell 3 |
| 16 | `u16be` | 0.01 V | battery voltage (uncertain) |
| 18 | `u8` | 0.01 bar | current setpoint (Petrel family) |
| 19 | `u16be` | see [Air integration](#air-integration) | transmitter 2 pressure (log version ≥ 7) |
| 21 | `u8` | min | gas time remaining (log version ≥ 7). `F0`–`FF` are codes: `FF` not paired, `FE` no communication, `FD` not available in this mode, `FC` not available during deco, `FB` tank size or rated pressure not set |
| 22 | `u8` | % | CNS (Petrel family) |
| 23 | `u8` | m | ceiling. Under DCIEM: safe-ascent depth, whole metres (uncertain) |
| 24 | `u8` | % | GF99, `FF` = not applicable. Under DCIEM: safe-ascent depth, hundredths (uncertain) |
| 25 | `u16be` | min | time to surface if the diver stayed 5 more minutes, `FFFF` = n/a (uncertain) |
| 27 | `u16be` | see [Air integration](#air-integration) | transmitter 1 pressure (log version ≥ 7) |
| 29 | `u16be` | 0.01 (unit unknown) | surface air consumption, `FFFF` = n/a (uncertain) |

**Status byte (offset 11):**

| Bit | Mask | Meaning |
|----:|-----:|---------|
| 0 | `01` | gas switch suggested |
| 1 | `02` | ppO₂ comes from an external source. Skip the ppO₂ values when set |
| 2 | `04` | high setpoint selected (Predator: choose opening 0x12 over 0x11) |
| 3 | `08` | semi-closed circuit (when bit 4 is clear) |
| 4 | `10` | open circuit. Clear = closed-circuit loop |
| 5 | `20` | bail-out from CCR. ppO₂ values are not meaningful |
| 6–7 | `C0` | number of O₂ solenoid firings in this interval (uncertain) |

Bits 3 and 5 have different names in different sources ("SC" vs "circuit switch
type", "bail-out" vs "CCR mode"). Bits 1, 2 and 4 agree across sources.

**Temperature.** A signed byte. Non-negative values are used as-is. Negative raw values
`r` are corrected to `min(r + 102, 0)`. That maps raw −128…−103 to −26…−1 and raw
−102…−1 to 0. Convert from °F when the units byte is 1. The correction only
reproduces what one reference does. It has not been checked on sub-zero dives.

**ppO₂ and cells** (closed circuit only, i.e. status bit 4 clear):

- Voted ppO₂ = offset 6 / 100 bar.
- Cell *n* ppO₂ = raw mV × calibration*n* / 100000 bar. Report a cell only if its bit is
  set in the calibration mask.
- **Predator only:** multiply the calibration by 2.2.
- If every masked cell has the factory default calibration 2100, the cells were
  probably not calibrated. This happens with DiveCAN controllers, which keep
  calibration in the sensor module. Either drop the cell values, or estimate one
  common factor: average voted ppO₂ divided by the average of the cells computed with
  2100. Apply it only if it falls outside 0.98–1.02.

**No-decompression limit.** Offset 9 is `0` with no stop (offset 2 = 0) on the first
sample before the computer has a value; read it as absent there and as 0 min elsewhere
(**likely**).

**Setpoint.** Petrel family: offset 18 / 100 bar. Predator: opening 0x11 (low) or 0x12
(high), chosen by status bit 2.

**Gas.** O₂ and He at offsets 7–8 give the current mix. A sample with both zero
carries no gas information. A change of (O₂, He, closed-circuit flag) from the
previous sample is a gas switch. In closed-circuit mode the mix is a diluent.
The first sample's mix is the starting gas, not a switch.

**Time.** Samples carry no timestamp. Sample *k* (counting from 1) is at
*k* × interval. The interval is 10 s, or opening 5 offset 23 (ms) when log version ≥ 9.
Another implementation puts the first sample at 0 instead. See
[Open questions](#open-questions).

**Avelo sample (`03`)**: same layout as `01`. The status byte is not used and the
sample is treated as open circuit. Only transmitter 1 (offset 27) is read.

Confidence: offsets 0–9, 11–15, 18, 19, 21, 22 and 27 **likely** (two independent
sources). Offsets marked uncertain come from one source.

### Air integration

The air-integration mode (opening 4 offset 28) decides which tank each pressure field
belongs to:

| Mode | Meaning |
|-----:|---------|
| 0 | off |
| 1 | transmitter 1 only |
| 2 | transmitter 2 only |
| 3 | transmitters 1 and 2 |
| 4 | high-pressure CCR: the two sample pressures are diluent (offset 27) and O₂ (offset 19) |
| 5 | on (log version ≥ 13, tanks enabled individually in opening 6/7) |
| 6 | on, with GNSS (uncertain) |

Each pressure field is a `u16be`:

- `FFF0`–`FFFF` are status codes: `FFFF` AI off, `FFFE` no communication for over 90 s,
  `FFFD` no communication for over 30 s, `FFFC` transmitter not paired.
- Otherwise bits 0–11 are pressure in units of 2 psi, and bits 12–15 are the
  transmitter battery state: 0 normal, 1 critical, 2 warning. A pressure of 0 means
  no reading.

The first and last non-zero readings give the begin and end pressure of a tank.

Teric: the 6-digit transmitter serial is stored with its three BCD bytes in reverse
order. Decode as BCD, then swap the first and last digit pairs.

### Extended sample (`E1`)

Follows a dive sample and adds tanks for the same instant. Offsets include the type
byte.

| Offset | Type | Version | Meaning |
|-------:|------|---------|---------|
| 1 | `u16be` | ≥ 13 | transmitter 3 pressure (same encoding as above) |
| 3 | `u16be` | ≥ 13 | transmitter 4 pressure |
| 5 | `u16be` | ≥ 14 | HP-CCR diluent pressure, 2 psi units, no status codes |
| 7 | `u16be` | ≥ 14 | HP-CCR O₂ pressure, 2 psi units, no status codes |

### Freedive record (`02`)

The record holds four 8-byte freedive samples at record offsets 0, 8, 16 and 24.
Byte 0 of the first sample is the record type. Unused trailing samples are all zero
and end the record. Any freedive record sets the dive mode to freedive.

| Offset in sub-sample | Type | Raw unit | Meaning |
|---------------------:|------|----------|---------|
| 1 | `u16be` | mbar | absolute pressure |
| 3 | `i16be` | 0.1 °C | temperature (always Celsius) |
| 0, 5–7 | — | — | unknown |

Depth in metres = (pressure − atmospheric) × 100 / (density × 9.80665). Atmospheric
pressure comes from opening 1 offset 16 (mbar), density from opening 3 offset 3. Each
sub-sample advances time by one sample interval.

Confidence: **likely** (one reference, internally consistent).

### Info event (`30`)

| Offset | Type | Meaning |
|-------:|------|---------|
| 1 | `u8` | event code |
| 4 | `u32be` | timestamp, same clock as the dive start |
| 8 | `u32be` | parameter 1 |
| 12 | `u32be` | parameter 2 |

Known codes:

| Code | Meaning | Parameter 1 | Parameter 2 |
|-----:|---------|-------------|-------------|
| 38 (`26`) | diver tag/bookmark | compass heading in degrees, `FFFFFFFF` = none | tag value |

Confidence: **likely**.

### Tissue snapshot (`70`–`75`)

Six consecutive records, `70` to `75`, make one snapshot. Bytes 1–3 of each record
are zero. Bytes 4–31 hold **little-endian** IEEE-754 `f32` values, seven per record.
Record `75` holds six values and then a big-endian `u32` timestamp at bytes 28–31.
Values 0–15 are N₂ loading of compartments 1–16 in bar. Values 16–31 are He loading
of compartments 1–16 in bar. Values 32–39 are unidentified, and value 40 has always
been zero. The first snapshot comes before the first sample and is stamped with the
dive start time. Later ones follow about every 4 minutes.

Confidence: **uncertain** (one source, Perdix 2 and Petrel 3 only).

## Header-level fields

### Dive mode

Taken from opening 4 offset 1 (native) or offset 0x70 (block) when the log version is
≥ 8. For older logs, derive it from the samples: any closed-circuit sample (status bit
4 clear) means CC, or SC if bit 3 is set. Any freedive record means freedive.
Otherwise the dive is OC.

| Value | Mode | Common model |
|------:|------|--------------|
| 0 | closed circuit / bail-out | CCR |
| 1 | open circuit, technical | OC |
| 2 | gauge | gauge |
| 3 | ppO₂ display | gauge |
| 4 | semi-closed / bail-out | SCR |
| 5 | closed circuit, second variant | CCR |
| 6 | open circuit, recreational (sub-mode in opening 6) | OC |
| 7 | freedive | freedive |
| 12 | Avelo | OC |

### Gas list

There are ten fixed slots. Slots 1–5 are open-circuit gases and 6–10 diluents. A slot
is reported if its O₂ and He are not both zero and it is either enabled (gas mask
bit, native format; always in the block format) or used in a sample. Diluents are
reported only in CC/SC dives. Freedives have no gases. Mixes first seen in samples
that match no slot are added (up to 20 mixes in total).

### Decompression model

| Value | Model | Parameters |
|------:|-------|------------|
| 0 | Bühlmann with gradient factors | GF low/high at opening offsets 4/5 |
| 1 | VPM-B | conservatism (opening 2 offset 19 / block 0x44) |
| 2 | VPM-B/GFS | conservatism as above, GFS gradient factor (opening 3 offset 5 / block 0x55) |
| 3 | DCIEM | — |

### Battery type

| Value | Battery |
|------:|---------|
| 1 | 1.5 V alkaline |
| 2 | 1.5 V lithium |
| 3 | 1.2 V NiMH |
| 4 | 3.6 V Saft |
| 5 | 3.7 V Li-ion |

Confidence: **uncertain** (one source).

### Salinity and atmospheric pressure

Density 1000 kg/m³ means fresh water. Any other value means salt water, with that
density. The atmospheric pressure is in mbar.

## Date and time

The dive start (opening offset 12) is seconds since 1970-01-01T00:00 counted on the
diver's **local wall clock**. Decode it as if it were UTC to get the local date and
time.

On the Teric, with log version ≥ 9 and opening 5 present, the UTC offset is
`offset_minutes × 60 + dst_hours × 3600` seconds, from opening 5 offsets 26 and 30.
Other models have left these bytes zero on observed logs, which should be read as "no
time zone".

## Log versions

Log versions below 6 are not reliably recorded. Treat them as 6.

| Version | Added |
|--------:|-------|
| 6 | baseline |
| 7 | air integration: sample pressures, gas time remaining, AI mode, GTR mask; battery type |
| 8 | dive mode in the header |
| 9 | opening 5: transmitter serials and pressures, sample interval, UTC offset and DST |
| 11 | scrubber stack time (opening 6, closing 6) |
| 12 | recreational OC sub-mode |
| 13 | transmitters 3 and 4 (opening 6/7, extended sample `E1`); per-tank enable and name; sidemount GTR mask |
| 14 | HP-CCR diluent and O₂ pressures in `E1` |
| 17 | GNSS position (opening 9, closing 9) |

The dive-sample layout is the same in versions 7 through 17.

## Example (native format, real Perdix 2, imperial, fresh water at altitude)

Excerpts from a published capture. Only the leading bytes of each record are shown.

```
opening 0  10 FF 00 02 28 55 0B 88 01 0F 00 00 6A 47 A0 8D 00 46 82 A2 15 ...
           dive 2, GF 40/85, units 1 (imperial), start 0x6A47A08D, gas 1 O2 21 %
opening 1  11 00 … (offset 16) 03 31      atmospheric 817 mbar
opening 3  13 0B 03 03 E8                 density 1000 (fresh)
opening 4  14 06 … (offset 16) 11 00 01   mode 6 (OC rec), log version 17, gas mask 0001
opening 5  15 … (offset 23) 27 10         interval 10000 ms
sample     01 00 37 00 00 00 01 14 15 00 00 58 12 00 41
           depth 5.5 ft, TTS 1 min, ppO2 0.20, O2 21 %, He 0, status 12 (OC + external), 65 °F
closing 0  20 FE 00 02 00 42 00 00 1C 0F 00 00 6A 47 A0 A4
           dive 2, max 6.6 ft, dive time 28 s
```

In this capture opening 0 offset 19 is `A2`, which is not valid BCD. That casts doubt
on reading a firmware version from that offset.

## Open questions

1. Is the first sample at time 0 or at one interval? Implementations differ.
2. Status bits 3 and 5: which name is correct ("semi-closed" or "circuit switch
   type"; "bail-out" or "CCR mode")?
3. Units of the transmitter rated and reserve pressures (opening 5–7), and of the SAC
   field at sample offset 29.
4. Meaning of opening offset 6 (surface interval?), offset 9 (battery voltage?) and
   offset 19 (firmware?). Meaning of closing 0 offset 12: it looks like an end time,
   but the capture shows 23 s after the start for a 28 s dive.
5. The temperature correction for negative raw values has not been checked on sub-zero
   dives.
6. Layouts of opening/closing 8, of records `40`, `50`, `60` and `80`–`88`, and of
   the unknown bytes in freedive sub-samples.
7. Is AI mode 6 "on with GNSS" or something else?
8. One reading treats native record type `01` as "first sample" and `02` as "further
   samples". The other sources treat `02` as a freedive record. Freedive data supports
   the latter. Logs mixing `01` and `02` in an OC dive would settle it.
9. Air-integration mode 4 (HP-CCR) puts diluent and O₂ pressures in the dive sample
   (offsets 27 and 19). Log version 14 adds them to the extended sample `E1` (offsets 5
   and 7) as well. Are these the same readings, and does mode 4 still use the dive-sample
   fields from version 14 on? The divecomputer-rs parser reads both into the same two
   tanks and lets `E1` override the dive sample at the same instant (**uncertain**).
10. Are transmitters 3 and 4 in `E1` used only under AI mode 5, or under other modes
    too? The parser reads them only under mode 5 (and 6) with the tank enabled in
    opening 6/7 (**uncertain**).
11. The ceiling at sample offset 23 is documented in whole metres. Is it in feet when the
    units byte is 1? Until that is known, the parser does not decode the ceiling.
12. A block-format log without a final block is read as a Predator log (16-byte
    samples, cell calibration × 2.2, setpoints from the opening block), because
    Petrel-family block logs always include one. A Petrel-family block log that lost its
    final block would be misread (**likely**).
13. What does a sample interval of 0 in opening 5 mean? The parser rejects such a log
    instead of guessing.
14. Can an info event (`30`) carry a timestamp before the dive start? Such an event has
    no place on the elapsed-time axis; the parser drops it.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>,
  used as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>,
  used as a factual reference; no code copied.
- *The Petrel Native Format (PNF), an unofficial specification* (MIT),
  <https://github.com/snehankekre/pnf/blob/main/SPEC.md>.
- shearwater-rs (MIT/Apache-2.0), Perdix 2 sample records,
  <https://github.com/kenthejr/shearwater-rs>.
