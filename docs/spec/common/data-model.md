# Common dive data model

Every family document ends by mapping its raw records into the model defined here. The model
is deliberately small: it holds what divers and logbook software use, in SI units, and keeps
everything else as raw bytes so nothing is lost. Field names below are descriptive, not an
API; implementations choose their own identifiers.

Units (see [../conventions.md](../conventions.md)): depth m, temperature °C, pressure bar,
time s, gas fractions 0..1, volume litres (water capacity), ppO2 bar, CNS as a fraction 0..1
(100 % = 1.0).

## 1. Download result

A download produces one **device record** and zero or more **dive records**, ordered newest
first (the order devices naturally deliver them).

### 1.1 Device record

| Field | Type | Notes |
|-------|------|-------|
| vendor, product | text | from the identification step |
| family | identifier | the family document that applies |
| model code | integer | as reported by the device (see family document) |
| serial number | text | devices report numbers, BCD or ASCII; keep the device's rendering |
| firmware version | text | |
| hardware id | integer, optional | where a family distinguishes hardware revisions |
| clock correlation | `(device_ticks, host_local_time)`, optional | required for device-relative clocks ([datetime.md](datetime.md) section 6) |
| raw identification bytes | bytes | the unparsed identity/version reply |

### 1.2 Dive record

| Field | Type | Notes |
|-------|------|-------|
| fingerprint | bytes | section 4 |
| raw | bytes | the dive's raw header and profile exactly as read (enables re-parsing) |
| header | section 2 | |
| samples | list of section 3 | in time order |

**Confidence:** likely (design).

## 2. Dive header

All fields are optional; a family fills what its data provides and leaves the rest absent
(never zero-filled).

| Field | Type / unit | Notes |
|-------|-------------|-------|
| start | local date-time + optional UTC offset (s) | [datetime.md](datetime.md) |
| number | integer | the device's own dive counter, if any |
| duration | s | as stored by the device; may differ from the last sample time (devices often exclude surface time at the end) |
| max depth | m | as stored in the header, not recomputed |
| average depth | m | |
| min / max / surface temperature | °C | |
| surface (atmospheric) pressure | bar | as measured or assumed by the device |
| water type | `fresh` / `salt` / `en13319` / custom density (kg/m³) | the device's depth-conversion setting |
| dive mode | `open circuit`, `closed circuit (CCR)`, `semi-closed (SCR)`, `gauge`, `freedive`, `unknown` | |
| deco model | `bühlmann` (+ gradient factors low/high, 0..1), `vpm` (+ conservatism), `rgbm` (+ conservatism), `dciem`, `none/unknown` | conservatism kept as the device's own integer setting |
| personal adjustment / altitude level | integer, device-defined | |
| surface interval before the dive | s | |
| desaturation / no-fly time after the dive | s | |
| CNS at end, OTU | fraction, units | |
| gas mixes | list of gas mix (2.1) | index used by samples |
| tanks | list of tank (2.2) | index used by samples |
| location | latitude, longitude (degrees, WGS 84), optional | some GPS-capable devices |
| vendor fields | key → value | anything else worth keeping |

### 2.1 Gas mix

| Field | Unit | Notes |
|-------|------|-------|
| O2 | fraction 0..1 | |
| He | fraction 0..1 | 0 for nitrox |
| N2 | fraction | derived: 1 − O2 − He |
| usage | `open circuit`, `diluent`, `oxygen`, `bailout`, `unknown` | |
| enabled | boolean | some devices store disabled mixes |

Devices store fractions as percent integers or in other scaled units; a value of 0 % O2 often
means "air" (21 %) and the family document says so.

### 2.2 Tank

| Field | Unit | Notes |
|-------|------|-------|
| gas mix index | index into gas mixes | optional |
| water volume | l | optional; imperial devices store "nominal gas volume at working pressure" in cubic feet; convert with the working pressure: `V_water_l = V_gas_cuft × 28.3168 × 1.01325 / P_work_bar` (ideal-gas approximation) |
| working pressure | bar | optional |
| begin / end pressure | bar | from the header or the first/last pressure samples |
| sensor / transmitter id | integer | wireless air-integration transmitter |
| usage | `open circuit`, `diluent`, `oxygen`, `sidemount left/right`, `unknown` | |

**Confidence:** likely (design).

## 3. Samples

A sample is a time point with any subset of the following values. Families deliver samples at
a fixed interval with optional extra values interleaved; hosts SHOULD merge everything that
belongs to the same time into one sample.

| Field | Unit | Notes |
|-------|------|-------|
| time | s since dive start | strictly increasing |
| depth | m | |
| temperature | °C | |
| tank pressure | bar, per tank index | several tanks possible |
| ppO2 | bar, per sensor index, plus optional "computed/voted" value | CCR |
| setpoint | bar | CCR |
| CNS | fraction | |
| deco state | kind (`ndl`, `safety stop`, `deco stop`, `deep stop`) + stop depth (m) + time (s; NDL remaining or stop time) | |
| time to surface | s | |
| remaining bottom / air time | s | air-integrated devices |
| current gas mix | index | set when it changes |
| heart rate | 1/min | |
| bearing | degrees 0..359 | compass |
| ascent rate | m/s | if stored, not derived |
| events | list (3.1) | |
| vendor data | (type, bytes) | unparsed extra records |

### 3.1 Events

| Event | Value | Notes |
|-------|-------|-------|
| gas switch | gas mix index | also reflected in "current gas mix" |
| ascent rate warning | severity / rate | |
| deco ceiling violation | — | |
| missed / violated deco stop | — | |
| safety stop (voluntary / mandatory) begin/end | — | |
| deep stop | — | |
| ppO2 high / low | ppO2 (bar) | |
| CNS / OTU warning | value | |
| max depth / max time alarm | — | |
| low tank pressure / reserve | tank index | |
| transmitter lost / low battery | tank index | |
| battery low (device) | — | |
| bookmark / user mark | — | |
| surface / dive end detected | — | |
| setpoint switch | bar | CCR |
| bailout | gas mix index | |
| heading set | degrees | |
| workload / tissue saturation indicator | device-defined | |
| vendor event | (code, value) | anything not mapped |

Events carry the time of the sample they belong to. A family document lists, for every raw
event or alarm bit, which event it maps to; unknown bits become vendor events.

**Confidence:** likely (design).

## 4. Fingerprints and incremental download

A **fingerprint** is a short byte string that uniquely identifies a dive on a given device,
and is stable across downloads. Most families use the raw bytes of the dive's timestamp (4-8
bytes); some use a dive number or a header CRC. The family document defines it exactly.

Incremental download: the host stores the fingerprint of the newest dive it has; on the next
download it reads dives newest first and stops at the first dive whose fingerprint equals the
stored one. Devices that transfer only on request per dive (most modern ones) save time this
way; devices that dump their whole memory still dump it, and the fingerprint only limits
parsing.

Some devices filter on the device side instead: the host sends a timestamp and the device
returns only newer dives (Uwatec Smart). The threshold is derived from the newest stored dive
time as described in [datetime.md](datetime.md) section 6.

**Confidence:** likely.

## 5. Ring buffers

Many devices store profiles in a circular memory region `[begin, end)`. Shared definitions,
used by the family documents:

- **Normalise** an address `a`: `begin + ((a − begin) mod (end − begin))`, taking a
  non-negative modulo.
- **Distance** from `a` to `b` (moving forward): `(b − a) mod size`. When `a == b` the
  distance is ambiguous: 0 if the buffer is defined empty, `size` if it is full. Each family
  states which applies (typically "full" when walking from the newest dive's end back to its
  own start).
- **Increment / decrement** with wrap-around.
- A profile that crosses `end` continues at `begin`.

Reading a ring buffer **backwards** from the newest data (the common case) is done in chunks:
start at the end pointer, read the chunk ending there, move the pointer back, wrapping from
`begin` to `end`. Hosts SHOULD align chunk boundaries to the device's page size so that each
read request is a whole number of pages, and SHOULD cache the most recent chunk because
consecutive dives share pages.

Logbook (header) areas are often a separate ring of fixed-size entries with their own begin/end
and an index of the newest entry. An entry or profile consisting only of `FF` (erased flash) or
`00` marks unused space.

**Confidence:** likely.

## 6. Depth from pressure

Several devices record absolute or relative pressure instead of depth. Families MUST document
the device's own conversion. The common form is

```
depth_m = (p_abs − p_surface) / (ρ × g)
```

with `p` in pascals, `g = 9.80665 m/s²`, and the device's water density `ρ` (commonly
1000 kg/m³ fresh, 1025 or 1030 kg/m³ salt; EN 13319 uses 1020 kg/m³). Hosts that let the user
override density MUST still default to the device's convention so that depths match what the
diver saw.

**Confidence:** likely.

## 7. Raw preservation

Hosts SHOULD keep, per download, the device record's raw identification bytes and each dive's
raw bytes. Parsers improve over time; raw data lets old downloads be re-parsed. A full memory
dump, where the family supports it, is the most complete record and SHOULD be kept when the
user asks for diagnostics.

## References

- libdivecomputer (LGPL-2.1), consulted for the range of fields devices provide; no code or
  structure copied. https://www.libdivecomputer.org/
- EN 13319:2000, Diving accessories — depth gauges (reference density for depth display)
