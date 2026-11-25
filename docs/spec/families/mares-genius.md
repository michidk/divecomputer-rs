# Mares Genius family (object protocol)

Covers the Mares Genius (2019) and its successors: Horizon, Sirius, Sirius L, Quad Ci,
Quad 2, Puck Air 2, Puck 4 and the Puck 4 derivatives (Puck Lite, Puck Pro EZ, Puck Pro
Ultra). These computers keep the two-byte command framing of the
[Icon HD family](mares-iconhd.md) (`cmd`, `cmd XOR A5`; `AA` … `EA`), but instead of raw
memory reads they expose numbered **objects** that are read with a segmented transfer
modelled on the CANopen SDO upload service. Each dive is two objects: a fixed-layout
header and a profile made of tagged, CRC-protected records. The usual transport is
Bluetooth Low Energy; Genius and Horizon also have a USB cable.

## Devices

| Vendor | Model | Product name in version reply | Model code | Transports | BLE mode | Notes |
|--------|-------|-------------------------------|------------|------------|----------|-------|
| Mares | Genius | `Genius` | 0x1C (28) | serial (USB cable), BLE | fixed | |
| Mares | Horizon | `Horizon` | 0x2C (44) | serial | — | semi-closed rebreather; header log format 1 |
| Mares | Puck Air 2 | `Puck Air 2` | 0x2D (45) | BLE | variable | |
| Mares | Sirius | `Sirius` | 0x2F (47) | BLE | variable | |
| Mares | Quad Ci | `Quad Ci` | 0x31 (49) | BLE | variable | |
| Mares | Quad 2 | `Quad2` | 0x32 (50) | BLE | variable | |
| Mares | Sirius L | `Sirius L` | 0x33 (51) | BLE | variable | |
| Mares | Puck 4, Puck Lite, Puck Pro EZ, Puck Pro Ultra | `Puck4`, `Puck Lite`, `Puck`, `Puck Pro U` | 0x35 (53) | BLE | variable | several names share one code |

The product name is the 16-byte NUL-padded field at offset `0x46` of the version reply
(see [mares-iconhd.md](mares-iconhd.md#session)). The model object (`0x2000`/`02`)
returns the model code as u32le.

BLE advertised-name prefixes: `Mares Genius`, `Sirius`, `Sirius L`, `Quad Ci`, `Quad2`,
`Puck4`, `Puck Lite`, `Puck`, `Puck Pro U`. (Prefix matching: `Puck` also matches the
others starting with `Puck`.)

Confidence: **likely** (names and codes from the reference implementation; the
Puck 4 sub-models from one source; one other source lists only `Mares Genius`,
`Sirius`, `Quad Ci` and `Puck4`).

## Transport parameters

- **Serial (Genius, Horizon):** 115200 baud, 8E1, DTR and RTS cleared, 3000 ms read
  timeout — as for the [Icon HD family](mares-iconhd.md#serial--usb).
- **BLE:** same GATT service and characteristics as the BlueLink Pro adapter, see
  [mares-iconhd.md](mares-iconhd.md#ble). Host writes are split into chunks of at most
  20 bytes; notifications of up to 244 bytes.

The protocol has two BLE behaviours:

- **Fixed mode** (Genius): byte-stream semantics exactly as on serial, except that the
  command payload follows the header immediately (see
  [mares-iconhd.md](mares-iconhd.md#ble-order-bluelink-pro-genius)).
- **Variable mode** (Puck Air 2, Sirius, Sirius L, Quad Ci, Quad 2, Puck 4): one
  notification carries one complete reply; see below.

Object segment payload size (bytes of object data per segment):

| Transport | Segment size |
|-----------|--------------|
| serial | 504 |
| BLE fixed mode (Genius) | 124 |
| BLE variable mode | 241 (244 − `AA` − segment header − `EA`) |

Confidence: **likely**.

## Session

1. Version command `C2 67` → `AA <140 bytes> EA`; identify the model by product name.
2. Read object `0x2000`/`02` (model, u32le) — optional cross-check.
3. Read object `0x2000`/`04` (serial number).
4. Read object `0x2008`/`01` (number of dives, u16le).
5. Read dives (see [Download procedure](#download-procedure)).

One other host reads the dive count from object `0x2003`/`01` first and then from
`0x2008`/`01` (the latter wins), and obtains the serial number with a separate "factory
data" command (below) instead of object `0x2000`/`04`.

Confidence: **likely**.

## Commands

| Command | Header | Payload (H→D) | Response payload | Notes |
|---------|--------|---------------|------------------|-------|
| Version | `C2 67` | — | 140 bytes | as Icon HD |
| Object init | `BF 1A` | 16 bytes | 16 bytes | starts an object upload |
| Object segment, toggle 0 | `AC 09` | — | 1 + k bytes | 1st, 3rd, 5th … segment |
| Object segment, toggle 1 | `FE 5B` | — | 1 + k bytes | 2nd, 4th … segment |
| Set time | `B0 15` | u32le local seconds since 1970 | — | see Icon HD |
| Factory data | `FD 58` | 18 bytes: `00 04` + 16 × `00` | 18 bytes | one source only |

### Object init

Request payload (16 bytes):

| Byte | Value |
|------|-------|
| 0 | `40` (upload request) |
| 1 | object index, low byte |
| 2 | object index, high byte |
| 3 | sub-index |
| 4–15 | `00` |

Response payload (16 bytes):

| Byte 0 | Meaning | Bytes 1–3 | Bytes 4–15 |
|--------|---------|-----------|------------|
| `41` | segmented upload follows | echo of index/sub-index; MUST match | 4–7: u32le total object size; rest unused |
| `42` | expedited: data is inline | echo | 4–15: the object data (12 bytes) |
| `80` | abort | echo | 4–7: u32le abort code |

These are the CANopen SDO command specifiers (initiate upload request, initiate upload
response with size indicated, expedited response, abort transfer). Observed abort code:
`0x08000020` ("data cannot be transferred", in CANopen terms), returned when a dive's
profile object is not available.

### Object segment

Request: just the header `AC 09` or `FE 5B`, alternating, starting with `AC 09` for the
first segment of each object.

Response payload: one segment header byte followed by up to *segment size* data bytes.
In the header byte, bits 4–7 hold the toggle (0 or 1) and MUST equal the toggle of the
request; bit 0 set marks the last segment (CANopen "no more segments" flag; one source
only). Repeat until the total size announced in the init response has been received.

On serial and BLE fixed mode the host reads exactly `1 + min(remaining, segment size)`
bytes between `AA` and `EA`, i.e. the device always fills segments.

### Factory data (one source)

`FD 58`, wait for `AA`, send `00 04` followed by 16 zero bytes, receive 18 bytes and a
trailer byte. Bytes 12–17 of the reply are the serial number as six ASCII digits.

### Worked example

Read the header of the third-newest dive (object `0x3002`, sub-index `02`), BLE fixed
mode:

```
H→D  BF 1A 40 02 30 02 00 00 00 00 00 00 00 00 00 00 00 00
D→H  AA 41 02 30 02 <u32le size> 00 00 00 00 00 00 00 00 EA
H→D  AC 09
D→H  AA 00 <124 bytes> EA
H→D  FE 5B
D→H  AA 10 <…> EA          (or 11 if this is the last segment)
…
```

(On serial the 16-byte payload is sent only after the device's `AA`.)

Confidence: **likely** for request/response layout and the segment toggle;
**uncertain** for the last-segment bit and the factory data command.

## Framing and integrity

<a id="ble-variable-mode"></a>

**Serial and BLE fixed mode:** as in [mares-iconhd.md](mares-iconhd.md#framing-and-integrity).

**BLE variable mode:**

```
H→D  cmd cmd^A5
D→H  one notification:
       - if the command has no payload: the whole reply  AA <payload> EA
       - if it has a payload: AA only
H→D  <payload>                     (if any)
D→H  one notification: <payload> EA
```

The concatenated reply MUST start with `AA` and end with `EA`; its payload length is
the total minus 2. A reply longer than 244 bytes cannot occur.

Retries: on timeout or framing error, wait ~1 s, purge, and repeat the command (up to
4 retries). A retried segment request keeps its toggle value.

There is no transport checksum. The profile object's records carry their own CRC
(below).

Confidence: **likely**.

## Memory layout

Not applicable: dives are addressed as objects.

| Object | Sub | Content |
|--------|-----|---------|
| `0x2000` | `02` | model code, u32le (expedited) |
| `0x2000` | `04` | at least 16 bytes; bytes 10–15 = serial number as 6 ASCII decimal digits |
| `0x2003` | `01` | u16le dive count (one source only) |
| `0x2008` | `01` | u16le number of dives in the log |
| `0x3000 + i` | `02` | header of dive `i`; `i = 0` is the newest |
| `0x3000 + i` | `03` | profile of dive `i` |

One source caps `i` at `0x400` (1024 dives).

Confidence: **likely**.

## Download procedure

```
n = dive count from 0x2008/01
for i in 0 .. n-1:
    header = read object (0x3000 + i, 02)
    fp = header bytes 8..11
    stop if fp equals the newest fingerprint already known
    profile = read object (0x3000 + i, 03)
        if the upload aborts with code 08000020: keep the header only
    emit (header, profile)
```

Fingerprint: 4 bytes at header offset `0x08` (the packed date/time).

A Sirius freedive (header mode 5) has been seen to report its missing profile not in the
init reply but in place of the first segment reply, as the 16-byte payload
`80 <idx lo> <idx hi> 03 20 00 00 08 00 00 00 00 00 00 00 00` (abort code `0x08000020`).
A host SHOULD treat that reply as "no profile" rather than as a protocol error.

An alternative incremental strategy used by one host: read headers from `0x3000` upward
until the header date is not newer than the last download, then fetch the profiles of
the new dives.

Confidence: **likely**; the freedive abort shape is from two Sirius logs only
(**uncertain**).

## Dive data format

### Header object (sub-index 02)

Offsets are from the start of the object. `X` = 8 if the log-format byte at `0x10` is 1
(Horizon), else 0; the 8 extra bytes are inserted at `0x18`, so every field from `0x18`
onward moves by `X`.

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x00 | u16le | | object type, 1 = dive header |
| 0x02 | u8 | | version minor |
| 0x03 | u8 | | version major (versions up to 2.0 are known) |
| 0x08 | u32le | | date/time, packed (below) |
| 0x0C | u32le | | settings (below) |
| 0x10 | u8 | | log format: 0 normal, 1 Horizon layout |
| 0x18 | | | single-gas O2 (one source only; meaning unclear) |
| 0x20+X | u16le | count | number of profile samples |
| 0x22+X | u16le | 0.1 m | maximum depth |
| 0x24+X | u16le | 0.1 m | average depth |
| 0x26+X | i16le | 0.1 °C | maximum temperature |
| 0x28+X | i16le | 0.1 °C | minimum temperature |
| 0x2C+X | u32le | s | surface interval before the dive (one source only) |
| 0x34+X | u8 | | non-zero = metric units for tank volume/pressure |
| 0x3E+X | u16le | mbar | atmospheric pressure; bar = raw / 1000 |
| 0x4C+X | u32le | | bit 4: decompression dive (one source only) |
| 0x54+X + 20·g | 20 bytes | | gas/tank block g = 0..4 (below) |

Header object length = `0xB8 + X`, plus 16 bytes when the major version is ≥ 1.

Settings word:

| Bits | Meaning |
|------|---------|
| 0–3 | dive mode: 0 air, 1 nitrox single gas, 2 nitrox multi-gas, 3 trimix, 4 gauge, 5 freedive, 6 SCR, 7 open circuit (generic) |
| 5–6 | water: 0 fresh, 1 salt, 2 EN 13319 density (1.020 kg/L) |
| 10–12 | altitude class (one source only) |
| 13–18 | surface timeout in minutes (end-of-dive detection); valid only when the profile object is type 0 with version ≥ 1.0, otherwise assume 3 min |

Gas/tank block (20 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| +0 | u32le | bits 0–6 O2 %; bits 7–13 N2 %; bits 14–20 He %; bits 21–22 state (0 off, 1 ready, 2 in use, 3 ignored); bit 23 changed during dive |
| +4 | u16le | tank start pressure, 0.01 bar |
| +6 | u16le | tank end pressure, 0.01 bar; 36000 = not measured |
| +8 | u16le | tank volume: litres (metric) or cu ft (imperial) |
| +10 | u16le | working pressure: bar (metric) or psi (imperial) |
| +12 … +19 | | unknown |

O2 + N2 + He should be 100. Enabled gases come first: count blocks from 0 until the
first with state 0. Tanks likewise: a block is an active tank while its start pressure is
non-zero or its end pressure is neither 0 nor 36000. Imperial tank conversion as in
[mares-iconhd.md](mares-iconhd.md#dive-data-format).

Dive time = samples × 5 s − surface timeout.

Packed date/time (u32le at `0x08`):

| Bits | Field |
|------|-------|
| 0–4 | hour |
| 5–10 | minute |
| 11–15 | day |
| 16–19 | month (1..12) |
| 20–31 | year (full) |

Local time, no time zone. Example: 2023-07-15 14:30 → `0x7E777BCE` → bytes
`CE 7B 77 7E`.

### Profile object (sub-index 03)

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u16le | profile type: 0 normal (versions up to 2.0 known), 1 (versions up to 0.2 known) |
| 0x02 | u8 | version minor |
| 0x03 | u8 | version major |
| 0x04 … | | sequence of records until the end of the object |

Every record has the form

```
<4-byte ASCII tag> <body> <u16le CRC> <the same 4-byte tag>
```

The tag identifies the record and fixes its total length:

| Tag | Bytes (hex, big-endian) | Total length | Content |
|-----|-------------------------|--------------|---------|
| `DSTR` | 44 53 54 52 | 58 | dive start |
| `TISS` | 54 49 53 53 | 138 | tissue state |
| `DPRS` | 44 50 52 53 | 34 | open-circuit sample |
| `SDPT` | 53 44 50 54 | 78 | SCR sample |
| `AIRS` | 41 49 52 53 | 16 | air-integration (tank pressure) |
| `DEND` | 44 45 4E 44 | 162 | dive end |

An unknown tag, a mismatched end tag or a CRC error make the profile unusable from that
point (the record length is unknown).

CRC: [CRC-16/XMODEM](../common/checksums.md#crc-16-xmodem) — width 16, poly `0x1021`,
init `0x0000`, refin false, refout false, xorout `0x0000`; check value for `123456789` is
`0x31C3`. Covers the record from byte 4 (after the leading tag) up to but excluding the
CRC, i.e. `length − 10` bytes. Stored little-endian at `length − 6`.

Typical order: `DSTR`, `TISS`, then `DPRS` (or `SDPT`) samples with an `AIRS` record
after every fourth sample on air-integrated dives, and finally `DEND`. One host relies on
this order and finds the first sample body at profile offset 204 (= 4 + 58 + 138 + 4).

Each `DPRS`/`SDPT` record is one sample; samples are **5 s** apart, first at 5 s.

`DPRS` fields (offsets from the record start, i.e. including the 4-byte tag):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x04 | u16le | depth, 0.1 m (one source masks with `0x07FF`) |
| 0x08 | u16le | temperature, 0.1 °C (one source uses 12 bits) |
| 0x0E | u16le | NDL, or deco stop time when in deco; minutes |
| 0x10 | u32le | active alarms, one bit each (below) |
| 0x18 | u32le | bits 2–5 bookmark number (0 = none); bits 6–9 gas/tank index; bit 18 deco stop required; bits 19–25 deco stop depth, m |
| 0x1C | u16le | CRC |
| 0x1E | 4 | `DPRS` |

`SDPT` fields:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x06 | u16le | depth, 0.1 m |
| 0x0A | u16le | temperature, 0.1 °C |
| 0x18 | u32le | active alarms |
| 0x1C | u32le | bits 2–5 bookmark; bits 6–9 gas index; bit 10 deco stop required |
| 0x20 | u32le | if deco: bits 3–9 stop depth (m), bits 10–17 stop time (min), bits 18–31 time to surface (unit unverified); else bits 0–7 NDL (min) |
| 0x48 | u16le | CRC |

`AIRS` fields: u16le tank pressure at offset 0x04, 0.01 bar, for the tank whose index
equals the gas index of the preceding sample; remaining body bytes unknown.

Example `AIRS` record (200.00 bar, other body bytes zero; synthetic):
`41 49 52 53 20 4E 00 00 00 00 C8 EB 41 49 52 53` (CRC over `20 4E 00 00 00 00` =
`0xEBC8`).

Alarm bits:

| Bit | Alarm |
|-----|-------|
| 1 | slow down |
| 2 | ascent too fast |
| 3 | uncontrolled ascent |
| 4 | MOD reached |
| 5 | CNS high |
| 6 | CNS very high |
| 7 | deco stop missed |
| 8 | deco violation |
| 9 | battery low |
| 10 | battery very low |
| 11 | transmitter battery low |
| 12 | tank pressure low |
| 13 | tank reserve reached |
| 14 | transmitter link lost |
| 15 | maximum depth alarm |
| 16 | runaway deco |
| 17 | half tank pressure reached |
| 18 | 2 min to end of no-deco time |
| 19 | no-deco limit exceeded (deco begins) |
| 20 | multi-gas: a tank is low |
| 21 | half of dive-time alarm |
| 22 | dive-time alarm reached |
| 23 | gas switch point |
| 24 | gas ignored |
| 25 | gas changed |
| 26 | gas not changed |
| 27 | gas added |

One other source reads an alarm mask `0x0B` from the u32 at offset 12 of the *following*
record, which disagrees with the table; see open questions.

Gas index MUST be below the number of enabled gases when gases are defined.

Confidence: **likely** for the record framing, CRC and the fields above; field
meanings marked "one source only" are **uncertain**.

## Model-specific variations

- Horizon (log format 1): 8 extra header bytes at `0x18`; SCR samples (`SDPT`).
- Genius: BLE fixed mode, 124-byte segments.
- Sirius-generation models: BLE variable mode, 241-byte segments.

## Open questions

1. Content of `DSTR`, `TISS`, `DEND`, the 8 Horizon bytes and the extra 16 bytes of
   header version ≥ 1.
2. Unit of the SCR time-to-surface field.
3. Is the alarm word at `DPRS` offset 0x10 (as tabulated), or is there a different flag
   word as a second source suggests?
4. Serial number: object `0x2000`/`04` (bytes 10–15) versus the `FD 58` factory-data reply
   (bytes 12–17). Are both valid on all models? Meaning of the `00 04` argument.
5. What is object `0x2003`/`01`, and when does it differ from `0x2008`/`01`?
6. Does the last-segment bit in the segment header always agree with the announced
   object size?
7. Profile type 1 (versions ≤ 0.2): which models produce it, and is its record set the
   same?

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used
  as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>,
  used as a factual reference; no code copied.
- CiA 301, CANopen application layer: SDO upload protocol and abort codes (public
  overview: <https://www.can-cia.org/can-knowledge/>), for the command-specifier and
  abort-code meanings.
