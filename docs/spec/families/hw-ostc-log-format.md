# Heinrichs Weikamp OSTC logbook and profile format

This document describes the dive record that all Heinrichs Weikamp computers return: a dive
header followed by a variable-length profile. One family of formats covers every model. The
**profile version** byte selects the header layout. The profile is a stream of samples: depth,
then a flag byte that gives the number of extra bytes, optional event bytes, then optional
"extended information" fields whose presence is governed by per-field **divisors**. The
transport side is in [hw-ostc-legacy.md](hw-ostc-legacy.md) (OSTC, Mk2, 2N, 2C, Frog) and
[hw-ostc3.md](hw-ostc3.md) (hwOS and OSTC 4/5).

All multi-byte integers are little-endian unless stated otherwise.

## Profile versions

| Version | Devices | Header size | Where the version byte sits | Time stored | Divisor table |
|---------|---------|-------------|-----------------------------|-------------|---------------|
| `20` | OSTC, Mk2 (early firmware) | 47 | header offset 2 | end of dive | 6 nibble bytes in header |
| `21` | OSTC Mk2, 2N, 2C | 57 | header offset 2 | end of dive | 6 nibble bytes in header |
| `22` | Frog | 256 | header offset 8 | end of dive | 6 nibble bytes in header |
| `23` | hwOS (OSTC 3, Plus, 2, Sport, cR, …) older firmware | 256 | header offset 8 | end of dive | profile small header |
| `24` | hwOS newer firmware, all OSTC 4/5 dives | 256 | header offset 8 | **start** of dive | profile small header |

A parser MUST select the version byte position from the device family: offset 2 for the
OSTC/Mk2/2N/2C dump format, offset 8 for Frog and hwOS. Any other version value is unknown
and MUST be rejected. The OSTC 4/5 always reports `24` over the wire. Its internal version is
`30`. Entries of any other internal version are reported as `FF`.

Confidence: **verified** for `21`, `23`, `24` (vendor firmware sources and interface
documents). **likely** for `20` and `22` (one open-source implementation).

## Header layouts

### Versions `20` and `21` (OSTC, Mk2, 2N, 2C)

Offsets are relative to the first `FA` of the header.

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0 | `FA FA` | | header start marker |
| 2 | u8 | | profile version (`20` or `21`) |
| 3 | u8 ×5 | | month, day, year−2000, hour, minute: **end** of dive, local time. The 5 bytes are the fingerprint |
| 8 | u16 | mbar (relative, salinity-adjusted) | maximum depth. Implementations use value/100 → m |
| 10 | u16 + u8 | min, s | dive time (excludes very shallow parts) |
| 13 | i16 | 0.1 °C | minimum temperature |
| 15 | u16 | mbar | surface pressure 30 min before the dive |
| 17 | u16 | min | desaturation time after the dive |
| 19 | 5 × (u8, u8) | % | gas 1-5: O2, He. In CCR mode these are diluents 1-5 |
| 29 | u8, u8 | % | gas 6 (manually set mix): O2, He |
| 31 | u8 | | initial gas number, 1-5 (`FF` = none) |
| 32 | u8, u8 | | firmware major, minor |
| 34 | u16 | mV (likely) | battery voltage |
| 36 | u8 | s | sample interval |
| 37 | u8 ×6 | | divisor bytes for types 0-5 (see [Divisors](#divisors-and-extended-information)) |
| 43 | u8 | | salinity: 100-104 meaning 1.00-1.04 kg/l |
| 44 | u8 | % | CNS at end of dive |
| 45 | u16 | mbar (relative) | average depth, value/100 → m (version `21`. Implementations ignore it for `20`) |
| *end of version `20` header (47 bytes)* | | | |
| 47 | u16 | s | total dive time including shallow parts |
| 49 | u8, u8 | % | GF low, GF high (GF modes). Otherwise saturation and desaturation multipliers |
| 51 | u8 | | dive mode / deco model, see below |
| 52 | u8 | m | last deco stop depth setting |
| 53 | u8 | bit set | gases enabled: bit *i* = gas *i*+1 active |
| 54 | u8 | h | on-time since last full charge |
| 55 | `FB FB` | | header end marker |

Dive mode byte (offset 51, version `21`):

| Value | Mode | Deco model |
|-------|------|------------|
| 0 | open circuit | ZH-L16 (GF 100/100) |
| 1 | gauge | none |
| 2 | closed circuit | ZH-L16 |
| 3 | apnea (freedive) | none |
| 4 | open circuit | ZH-L16-GF (GF from 49/50) |
| 5 | closed circuit | ZH-L16-GF |
| 6 | pSCR | ZH-L16-GF |

For version `20` all five gases count as enabled and there is no mode byte.

Confidence: **verified** for version `21` field order (vendor Mk2 firmware source). **likely** for
the mode table, the 47-byte version `20` header, battery units and salinity encoding (one
open-source implementation).

### Version `22` (Frog)

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0 | `FA FA` (likely) | | header start |
| 2 | u24 | address | profile begin pointer into the profile ring |
| 5 | u24 | address | profile end pointer |
| 8 | u8 | | profile version `22` |
| 9 | u8 ×5 | | month, day, year−2000, hour, minute (end of dive). Fingerprint |
| 14 | u16 | mbar | max depth, value/100 → m |
| 16 | u16 + u8 | min, s | dive time |
| 19 | i16 | 0.1 °C | minimum temperature |
| 21 | u16 | mbar | surface pressure |
| 25 | u8 at 25, 27, 29 | % | O2 of gases 1-3 (no helium on the Frog) |
| 31 | u8 | | initial gas number 1-3 (`FF` = none) |
| 32 | u8, u8 | | firmware major, minor |
| 36 | u8 | s | sample interval |
| 37 | u8 ×6 | | divisor bytes (same encoding as versions `20`/`21`) |
| 43 | u8 | | salinity 100-104 |
| 45 | u16 | mbar | average depth |
| 47 | u16 | s | total dive time |
| 49 | u8, u8 | % | GF low, GF high |
| 51 | u8 | | mode: 0 ZH-L16 (OC), 1 ZH-L16-GF (OC), 2 apnea |
| 52 | u16 | | internal dive counter |

Confidence: **likely** (one open-source implementation, no vendor documentation for the Frog).

### Versions `23` and `24` (hwOS, OSTC 4/5)

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0 | `FA FA` | | header start |
| 2 | u24 | address | profile begin pointer (internal) |
| 5 | u24 | address | profile end pointer (internal) |
| 8 | u8 | | profile version `23` / `24` |
| 9 | u24 | bytes | profile length (see [hw-ostc3.md](hw-ostc3.md#download-procedure) for the transfer arithmetic) |
| 12 | u8 ×5 | | year−2000, month, day, hour, minute. `23` = end, `24` = **start** of dive. Fingerprint |
| 17 | u16 | mbar / cm (salinity included) | max depth, value/100 → m |
| 19 | u16 + u8 | min, s | dive time (excludes parts shallower than 1 m) |
| 22 | i16 | 0.1 °C | minimum temperature |
| 24 | u16 | mbar | surface pressure 30 min before the dive |
| 26 | u16 | min | desaturation time after the dive |
| 28 | 5 × 4 bytes | | gas or diluent 1-5: O2 %, He %, change depth m, type |
| 48 | 2 bytes | | firmware: hwOS major, minor. OSTC 4/5 packed `u16le` (bits 15-11 major, 10-6 minor, 5-1 patch, 0 beta) |
| 50 | u16 | mV | battery voltage after the dive |
| 52 | u8 | s | sample interval (also in the profile small header) |
| 53 | u16 | % | CNS at start of dive |
| 55 | u8 | % | GF at start of dive |
| 56 | u8 | % | GF at end of dive (OSTC 4/5 firmware ≥ 1.7.5: surface GF at end) |
| 57 | u16 | | configured logbook offset (OSTC 4/5: personal dive count) |
| 59 | u8 | | hwOS: battery information (undefined). OSTC 4/5: battery charge % (`FF` = unknown) |
| 60 | 5 × (u8, u8) | cbar, m | setpoint 1-5 and its change depth |
| 70 | u8 | % | salinity 0-5. Density = (100 + value) × 10 kg/m³ |
| 71 | u16 | % | maximum CNS |
| 73 | u16 | mbar | average depth, value/100 → m |
| 75 | u16 | s | total dive time including shallow parts |
| 77 | u8 | % | GF low (GF model). Otherwise saturation multiplier (OSTC 4/5 VPM: conservatism) |
| 78 | u8 | % | GF high (GF model). Otherwise desaturation multiplier |
| 79 | u8 | | deco model: 0 ZH-L16, 1 ZH-L16-GF, 2 VPM (OSTC 4/5) |
| 80 | u16 | | internal total dive counter at the time of the dive |
| 82 | u8 | | dive mode: 0 OC, 1 CC, 2 gauge, 3 apnea, 4 pSCR |
| 83 | u8 ×16 | min | N2 compartment desaturation times |
| 99 | f32 ×16 | mbar | N2 compartment pressures at end of dive |
| 163 | u8 ×16 | min | He compartment desaturation times |
| 179 | f32 ×16 | mbar | He compartment pressures at end of dive |
| 243 | u8 | m | last deco stop setting (3, 4, 5 or 6) |
| 244 | u8 | 10 cm | assumed distance to the shown stop |
| 245 | u16 | mV | hwHUD battery (0 = no HUD) |
| 247 | u8 | bit set | hwHUD status: bit 0 link ok, 1 calibrated, 2 HUD battery low, 3-5 sensor 1-3 active, 6 calibrated externally, 7 data from DR5/DRx |
| 248 | 6 bytes | 10⁻⁹ As | battery gauge registers (3.6 V battery only. Zero on OSTC 4/5) |
| 254 | `FB FB` | | header end |

Gas type byte: open circuit 0 disabled, 1 first, 2 travel, 3 deco. Diluent 0 disabled,
1 first, 2 normal. The first gas always has change depth 0. In CC and pSCR mode the list
holds diluents. The initial gas is the first entry whose type is 1. In CC mode, setpoint 1
(offset 60) is the initial setpoint.

Depth units: the vendor documents describe the max depth as "centimetres" (2019) and as "mbar"
(2025). Both say salinity is included. The average depth is "mbar". Implementations divide
both by 100 to get metres. See open questions.

Confidence: **verified** (vendor interface documents and OSTC 4 firmware source). **likely**
for pSCR mode `4` (one implementation) and the OSTC 4/5 meaning of offsets 56 and 59
(firmware source plus one implementation).

### Compact header (16 bytes, hwOS command `6D`)

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u24 | profile length (same value as full header offset 9) |
| 3 | u8 ×5 | year−2000, month, day, hour, minute (fingerprint) |
| 8 | u16 | max depth (as full header offset 17) |
| 10 | u16 + u8 | dive time min, s |
| 13 | u16 | internal dive counter |
| 15 | u8 | profile version (`FF` on firmware older than 1.92) |

An unused entry is 16 × `FF`.

Confidence: **verified** (vendor documents, OSTC 4 source).

## Profile

### hwOS small header (versions `23`, `24`)

The profile that follows the 256-byte header begins with:

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u24 | profile length |
| 3 | u8 | sample interval, s |
| 4 | u8 | N = number of divisor entries |
| 5 + 3k | u8, u8, u8 | entry k: type, size in bytes, divisor |

Samples start at offset 5 + 3N. Firmware so far writes N = 7, one entry per type 0-6 in type
order. The vendor reserves later entries for future types, so a parser SHOULD accept any N
and identify entries by their **type byte**, not their position. Existing implementations cap
N at 7.

The vendor tells hosts to use the interval "from the dive header". Implementations use the
small-header interval at offset 3. Both values are written from the same setting.

### Older layouts (versions `20`, `21`, `22`)

There is no small header. The interval is header offset 36. Offsets 37-42 hold one byte per
type 0-5 in type order. Bits 0-3 are the divisor (0 = never), bits 4-7 the field size. Samples
start right after the header.

### Divisors and extended information

Samples are numbered from 1. Field *t* is present in sample *n* exactly when its divisor
*d* ≠ 0 and *n* mod *d* = 0.

| Type | Field | Size | Encoding | Vendor default divisor |
|------|-------|------|----------|------------------------|
| 0 | temperature | 2 | i16, 0.1 °C | 6 |
| 1 | deco / NDL | 2 | byte 0: 0 = within no-deco limit, else first stop depth in m. Byte 1: NDL minutes, or length of first stop in minutes | 6 |
| 2 | gradient factor | 1 | u8, current GF % | 12 (OSTC 4/5: 0) |
| 3 | ppO2 sensors | 3 or 9 | size 3 (Mk2): one u8 per sensor, 0.01 bar (the Mk2 writes zeros). Size 9 (hwOS): per sensor u8 ppO2 in 0.01 bar + u16 cell voltage in 0.1 mV | 2 (0 in OC dives) |
| 4 | deco plan | 15 | u8 × 15: stop lengths in minutes for 15 stop levels. The Mk2 orders them 3 m … 45 m in 3 m steps. On hwOS the vendor points to the last-stop setting (header offset 243: 3-6 m). The exact level mapping is uncertain | 12 |
| 5 | CNS | 1 or 2 | size 1 (Mk2): u8 %. Size 2 (hwOS): u16 % | 12 |
| 6 | tank pressure | 2 | u16, active gas tank pressure, see below | 15 (OSTC 4/5: 30 or 0) |

Implementations check the sizes: temperature, deco and tank must be 2. ppO2 must be 3 or 9.
CNS must be 1 or 2.

Tank pressure unit: the 2019 vendor document says 0.1 bar. The OSTC 4/5 writes the sensor
value in bar, and its document says "not used yet". The reference implementation treats the
value as **bar**, except hwOS Sport firmware 10.40-10.50, which it treats as 0.1 bar. The
value 0 means "no reading". The value belongs to the tank of the currently active gas.

### Sample layout

```
u16    depth            (relative pressure, mbar, salinity NOT applied; value/100 → m)
u8     flag             bits 0-6: number of bytes that follow in this sample (0-127)
                        bit 7: at least one event byte follows
[event bytes]           only if flag bit 7
[event payloads]        in the fixed order below
[extended fields]       types in table order, each only when its divisor fires
[setpoint, bailout]     versions 20/21/22 only, see below
[unknown trailing bytes]   skip: the count in the flag byte is authoritative
```

The profile ends with `FD FD`, which must follow the last sample exactly. A profile of exactly
`FD FD`, or the 5-byte form `08 00 00 FD FD` (length field 8, end marker), contains no
samples.

Sample *n* (1-based) is at time *n* × interval seconds after dive start.

### Event bytes

**Event byte 1:**

| Bits | Meaning |
|------|---------|
| 0-3 | alarm: 0 none, 1 ascent too fast ("SLOW"), 2 deco stop missed, 3 deep stop missed (Mk2. hwOS: unused), 4 ppO2 low, 5 ppO2 high, 6 manual marker (bookmark), 7 low battery |
| 4 | manual gas set ("gas 6"): 2 payload bytes |
| 5 | gas change: 1 payload byte |
| 6 | setpoint change: 1 payload byte |
| 7 | versions `23`/`24`: another event byte follows. Versions `20`/`21`: bailout (2 payload bytes) |

**Event byte 2** (versions `23`/`24`, present when bit 7 of byte 1 is set):

| Bit | Meaning | Payload |
|-----|---------|---------|
| 0 | switched to bailout | 2 bytes |
| 1 | compass heading (OSTC 4/5) | 2 bytes |
| 2 | GNSS position (OSTC 4/5) | 8 bytes |
| 3 | scrubber state (OSTC 4/5) | 2 bytes |
| 4-6 | unused | |
| 7 | another event byte follows | |

Later event bytes follow the same chaining rule. Treat event byte *k* (0-based) as bits
8k..8k+7 of one event word.

**Payload order** (versions `23`/`24`). Payloads follow all event bytes:

1. manual gas set (byte 1 bit 4): O2 %, He %. A new mix not in the header list.
2. gas change (byte 1 bit 5): u8 gas number, 1-based into the header gas list. On the OSTC
   4/5 in CC mode, numbers 6-10 mean diluent 1-5 (see variations).
3. setpoint change (byte 1 bit 6): u8, cbar.
4. bailout (byte 2 bit 0): O2 %, He % of the open-circuit bailout gas.
5. compass heading (byte 2 bit 1): u16. Bits 0-8 heading in degrees, bit 14 "set as new
   reference heading", bit 15 "heading cleared" (bits 0-8 then meaningless).
6. GNSS position (byte 2 bit 2): f32 longitude, then f32 latitude (IEEE 754, little-endian).
7. scrubber state (byte 2 bit 3): u16. Bits 0-11 remaining scrubber time in minutes as a signed
   12-bit value (negative = overdue), bit 13 warning (< 30 min), bit 14 error (< 0 min).

Then the extended fields follow.

**Versions `20`, `21`, `22`:** a single event byte. Its payloads are: manual gas (bit 4,
2 bytes) and gas change (bit 5, 1 byte). Then the extended fields. **After** the extended
fields come setpoint change (bit 6, 1 byte) and bailout (bit 7, O2 %, He %). The vendor
firmware comment says the setpoint was appended at the end for compatibility.

Confidence: **verified** for sample layout, event bits, payload order and extended field
encodings (vendor documents and Mk2/OSTC 4 firmware sources). **likely** for alarm value 3 on
the Mk2, scrubber sign extension and the tank pressure unit handling (one implementation).

### Worked example

Vendor example. The divisors are temperature 3, deco 1, all others 0, so N = 7 with only
these two non-zero.

```
64 00 02 00 A0             depth 100 → 1.00 m; 2 bytes follow; deco: NDL 160 min
C8 00 02 00 A0             depth 2.00 m; NDL 160 min
2C 01 04 32 00 00 A0       depth 3.00 m; 4 bytes; temperature 0x0032 = 5.0 °C; NDL 160 min
5E 01 84 21 02 00 9F       depth 3.50 m; 4 bytes + event; event 0x21 = alarm 1 (ascent)
                           + gas change; gas 2; NDL 159 min
FD FD                      end of profile
```

In sample 3, 3 mod 3 = 0, so the temperature is present. In sample 4 the order is: event byte,
gas change payload, then the deco field. These values were re-parsed with a fresh script and
the flag-byte counts match.

## Date and time

Local wall-clock time without time zone. Year = 2000 + byte. Seconds are not stored.

- Version `24`: the header time is the dive **start**. Use it directly.
- Versions `20`-`23`: the header time is the dive **end**. Start = end − duration. Variants:
  - reference implementation: for versions `21`-`23` the duration is the total dive time
    (`21`/`22`: offset 47, `23`: offset 75), rounded down to whole minutes, to match the
    device display. For `20` it is the dive time field (min × 60 + s).
  - another implementation: duration = dive time field (min × 60 + s) for every version older
    than `24`.

Example (version `23`): end 2019-06-01 10:47, total time 2843 s → rounded 2820 s → start
10:00. The unrounded variant gives 09:59:37.

## Model-specific variations

- **OSTC 4/5 diluent numbering.** In CC dives the gas-change byte uses 6-10 for diluents 1-5.
  One implementation subtracts the number of fixed gases (5) only for OSTC 4/5 CC dives.
  Another subtracts 5 for any value in 6-10. The same second implementation treats manually
  set mixes on the OSTC 4/5 as open-circuit gases, even in CC mode.
- **OSTC 4/5 firmware below 1.0.8:** deco/NDL samples are wrong and should be ignored.
- **hwOS Tech 3.03-3.08 and hwOS Sport 10.57-10.63:** the ppO2 divisor is sometimes left
  non-zero while no ppO2 data is written. If a ppO2 field is due but fewer bytes than its size
  remain, treat its divisor as 0 for the rest of the dive.
- **hwOS Sport 10.40-10.50:** tank pressure in 0.1 bar.
- **GNSS:** the OSTC 4/5 may log a placeholder position (latitude 8.99, longitude 47.77 as
  stored). One implementation suppresses exactly that pair.
- **OSTC 4/5 dummy samples:** if the stored profile no longer matches its header, the firmware
  synthesises a sample stream from header data instead of sending the original profile.

Confidence: **likely** (one or two open-source implementations. The firmware bug
workarounds are not vendor-documented).

## Open questions

1. Depth unit. Samples are "mbar, not including salinity". The header max depth is
   "centimetres" or "mbar", "including salinity". The vendor note "depth in metres = depth in
   mbar × g0 / 1000" is dimensionally odd. Implementations use raw / 100 (one applies × 9.80665
   / 1000 to the average depth, following that note). Need dumps compared against the device
   display.
2. Tank pressure unit (0.1 bar per vendor 2019, bar per OSTC 4/5 source and reference
   implementation).
3. Alarm value 3: "deep stop missed" (Mk2 era, implementation) or "unused" (hwOS vendor doc).
4. Event-byte assembly. One implementation shifts earlier bytes up (`ev << 8 | next`) and then
   tests `0x100` for bailout, which conflicts with the vendor bit assignment. The vendor
   assignment (byte *k* = bits 8k..) is specified here.
5. Divisor entries: implementations either use the type byte (as specified here) or the
   table position. They agree while firmware writes types 0-6 in order.
6. Version `20` and `22` layouts lack vendor documentation.

## References

- Heinrichs Weikamp, *External Interface for Heinrichs Weikamp Dive-Computers with hwOS*,
  2019-04-10 (hwOS sources, `doc/hwos_interface.pdf`),
  <https://code.heinrichsweikamp.com/public/hwos_code>.
- Heinrichs Weikamp, *External Interface … hwOS and OSTC4/5*, v16, 2025-07-13 (OSTC 4 sources,
  `Documentations/hwos4_interface_v16.pdf`), <https://code.heinrichsweikamp.com/public/ostc4>.
- Heinrichs Weikamp OSTC Mk.2/2N/2C firmware source (GPL-3.0), `divemode.asm`, used as a
  factual reference for the version `21` header and sample writer.
- Heinrichs Weikamp OSTC 4/5 firmware source (GPL-3.0), `logbook.c`, `logbook.h`, used as a
  factual reference.
- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), used as a factual reference for OSTC 4/5
  variants; no code copied.
