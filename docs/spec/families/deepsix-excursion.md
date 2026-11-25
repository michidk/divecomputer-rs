# Deep Six Excursion family (Crest CR-4, Genesis Centauri, Scorpena Alpha)

The Deep Six Excursion and several rebadged models talk over Bluetooth Low Energy. The host
sends short binary request packets addressed by a (group, command) pair. Each packet carries
a direction byte, a length byte and a one-byte complemented additive checksum. Dives are
fetched by number: the header comes back in one response, and the profile comes back in
numbered offset chunks. Two dive data layouts exist. Version 0 is older firmware and uses a
fixed 156-byte header and type-tagged samples. Version 1 and later is a self-describing
format with sample and event descriptors.

## Devices

| Vendor | Model | Model code | Transports | BLE name |
|--------|-------|------------|------------|----------|
| Deep Six | Excursion | 0 | BLE | `EXCURSION` |
| Crest | CR-4 | 0 | BLE | `Crest-CR4` |
| Genesis | Centauri | 0 | BLE | `CENTAURI` |
| Scorpena | Alpha | 0 | BLE | `ALPHA` |

- The advertised name matches exactly, case-insensitive. The device does not report a
  numeric model code; all members share code 0.
- The firmware identifier string begins `D01-` (see the software-version response).

Confidence: **likely**.

## Transport parameters

- BLE. The GATT service and characteristic UUIDs are not publicly documented (see
  [Open questions](#open-questions)). The protocol is a byte stream: each request is
  written as one packet, and each response arrives as one notification.
- A serial line configuration (115200 8N1, no flow control) is applied by known hosts but
  has no effect over BLE.
- Response timeout: 3000 ms.
- After connecting, wait about 300 ms and discard any pending input before the first
  request.
- Maximum payload per packet: 255 bytes. A full packet is 260 bytes, so the link needs an
  ATT MTU of at least 263, or the device must split notifications. See
  [Open questions](#open-questions).

Confidence: **likely** (timings and sizes from one implementation).

## Session

1. Connect, wait 300 ms, flush input.
2. Send **settings load** (`B0 28`, direction write, empty payload). The device answers with
   an empty acknowledgement. Known hosts always do this first.
3. Read the hardware version, software version and serial number (see
   [Commands](#commands)).
4. Pick the dive-index scheme from the software version. If the string starts with `D01` and
   its fifth character (index 4) is `6` or higher, use the firmware-6 commands (count / last
   / previous). Otherwise use the legacy "last dive" query.
5. There is no explicit end-of-session command.

Confidence: **likely**.

## Commands

Requests are `group, command, direction, length, payload, checksum`. The response repeats
the command and direction, and its group byte is the request group **+ 1**. Direction `00` =
write/set, `01` = read/get.

| Function | Group | Cmd | Dir | Request payload | Response payload |
|----------|-------|-----|-----|-----------------|------------------|
| Hardware version | A0 | 01 | 01 | — | 6 bytes |
| Software version | A0 | 02 | 01 | — | 6 bytes ASCII, e.g. `D01-6x`. Bytes 4..5 read as `u16be` give a firmware number |
| Serial number | A0 | 03 | 01 | — | 12 bytes. Bytes 3..11 are ASCII decimal digits |
| Last dive (legacy) | A0 | 04 | 01 | `00 00` | `u16le` number of the newest dive; equals the dive count |
| Set date | B0 | 01 | 00 | year−2000, month, day | echo of the 3 bytes |
| Set time | B0 | 03 | 00 | hour, minute, second | echo of the 3 bytes |
| Store settings | B0 | 27 | 00 | `00` | empty |
| Load settings | B0 | 28 | 00 | — | empty |
| Dive header | C0 | 02 | 01 | `u16le` dive number | header (see [Header](#header)) |
| Dive profile chunk | C0 | 03 | 01 | `u16le` dive number, `u32le` byte offset | up to 255 profile bytes |
| Dive count (fw ≥ 6) | C0 | 05 | 01 | — | `u16le` count |
| Newest dive index (fw ≥ 6) | C0 | 06 | 01 | — | 4 bytes, `u16le` index in bytes 0..1 |
| Oldest dive index (fw ≥ 6) | C0 | 07 | 01 | — | (not used by known hosts) |
| Previous dive index (fw ≥ 6) | C0 | 08 | 01 | `u32le` current index | `u32le` index of the next older dive |
| Next dive index (fw ≥ 6) | C0 | 09 | 01 | `u32le` current index | (not used by known hosts) |

To set the clock, send set date, then set time, then store settings. Check that each echo
matches what was sent.

Confidence: **likely**. Commands marked "not used" are inferred from the numbering only.

## Framing and integrity

```
byte 0     group
byte 1     command
byte 2     direction
byte 3     payload length n (0..255)
4..3+n     payload
4+n        checksum
```

Checksum: the 8-bit sum (mod 256) of bytes 0..3+n, then bitwise inverted (XOR `0xFF`). This
is [sum8](../common/checksums.md#sum8) with a final ones' complement. A receiver MUST check
the group (request + 1), command, direction, length and checksum.

Worked examples:

```
H→D  B0 28 00 00 27              load settings (sum 0xD8, inverted 0x27)
H→D  A0 01 01 00 5D              read hardware version
H→D  C0 02 01 02 05 00 35        read header of dive 5
D→H  C1 02 01 <n> <header…> <ck> header response (group C0+1)
```

The last profile chunk can be padded beyond the dive length. A host MUST truncate to the
profile length from the header.

Confidence: **likely**.

## Memory layout

No raw memory access. Dives are addressed by number (legacy firmware) or by an index that
the device walks for the host (firmware 6 and later).

## Download procedure

```
load_settings(); hw = hardware(); sw = software(); sn = serial()
if sw starts with "D01" and sw[4] >= '6':
    count = dive_count(); idx = newest_index()
    numbers = [idx, previous(idx), previous(previous(idx)), ...]   # count entries
else:
    count = last_dive(); numbers = [count, count-1, ..., 1]
for number in numbers (newest first):
    hdr = dive_header(number)
    require len(hdr) >= 128
    require len(hdr) == hdr[2] (fw ≥ 6) or len(hdr) == 156 (legacy)
    if hdr[12..17] == known_fingerprint: stop
    length = u32le(hdr[8..11])
    profile = []
    while len(profile) < length:
        chunk = profile_chunk(number, offset=len(profile))
        profile += chunk[0 : min(len(chunk), length - len(profile))]
    dive = hdr + profile
```

Fingerprint: 6 bytes at offset 12 of the header (the dive's date and time).

Confidence: **likely**.

## Dive data format

A dive is the header response followed by the profile bytes. Byte 3 of the header is the
**layout version**: 0 = legacy, any other value = descriptor-based (v1).

Depth is stored as **absolute pressure in mbar**. Convert with the stored surface pressure:

```
depth_m = (p_mbar − surface_mbar) × 100 / (density × 9.80665)
```

Density is 1024 kg/m³ for layout 0. For layout 1 it is `1000 + 10 × salinity byte`.

### Header

Layout 0 (156 bytes):

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x03 | u8 | | layout version = 0 |
| 0x04 | u8 | | dive mode (table below) |
| 0x08 | u32le | bytes | profile length |
| 0x0C | 6 × u8 | | year−2000, month, day, hour, minute, second (fingerprint) |
| 0x12 | u8 | h | UTC offset + 12, valid only when the firmware is `5B` or later |
| 0x14 | u32le | s | dive time |
| 0x18 | u32le | s | sample interval |
| 0x1C | u16le | mbar | maximum absolute pressure → max depth |
| 0x20 | u16le | 0.1 °C | minimum temperature |
| 0x30 | 6 × ASCII | | firmware id, e.g. `D01-4C`. Characters 4..5 are the firmware version |
| 0x38 | u32le | mbar | surface pressure |

Layout 1 (variable size, at least 129 bytes; the size is in byte 2):

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x02 | u8 | bytes | header size |
| 0x03 | u8 | | layout version (≠ 0) |
| 0x04 | u8 | | dive mode |
| 0x05 | u8 | s | sample interval |
| 0x07 | u8 | | salinity: density = 1000 + 10 × value kg/m³; value 0 = fresh water |
| 0x08 | u32le | bytes | profile length |
| 0x0C | 6 × u8 | | year−2000, month, day, hour, minute, second (fingerprint) |
| 0x12 | i8 | 15 min | UTC offset |
| 0x13 | u32le | s | dive time |
| 0x1D | u16le | mbar | maximum absolute pressure |
| 0x1F | u16le | 0.1 °C | minimum temperature |
| 0x21 | u16le | mbar | average absolute pressure |
| 0x23 | 6 × ASCII | | firmware id |
| 0x2B | u16le | 0.1 °C | surface temperature |
| 0x2D | u16le | mbar | surface pressure |
| 0x7F | u8 | % | gradient factor low |
| 0x80 | u8 | % | gradient factor high |

Dive mode: 0 open circuit, 1 gauge, 2 freedive, 3 closed circuit.

### Samples, layout 0

The profile is a sequence of type-tagged records:

| Type | Length | Content |
|------|--------|---------|
| 1 | 8 | alarm: +1 u8 misc, +2 u16le pressure, +4 u16le alarm time, +6 u16le alarm value |
| 2 | 6 | depth/temperature: +1 u8 misc, +2 u16le pressure (mbar), +4 u16le temperature 0.1 °C |
| 3 | 6 | decompression: +2 u16le pressure, +4 u16le value (not decoded) |
| 4 | 8 | ceiling: +2 u16le pressure, +4 u16le ceiling depth, +6 u16le ceiling time (not decoded) |
| 5 | 6 | CNS: +2 u16le pressure, +4 u16le CNS % |

- Only type 2 advances time, by the header sample interval. Its pressure field is the
  profile depth.
- Any other type byte is a format error.
- Firmware `D01-4C` writes only types 1 and 2. In that firmware a type-2 record whose
  temperature field is above 1300 is actually 8 bytes long, and temperatures below 1.0 °C
  (raw < 10) are invalid.

### Samples, layout 1

The profile starts with two descriptor tables:

```
u8 nsample_types (≤ 7)
nsample_types × { u8 type, u8 size, u8 divisor }
u8 nevent_types (≤ 7)
nevent_types × { u8 type, u8 size }
```

Then it repeats, once per sample interval:

```
u16le absolute pressure, mbar            (depth)
u8    event block length L
if L > 0:
    u16le event bitmask (bit k set → event type k present)
    for each declared event type, in declaration order, whose bit is set:
        <size> bytes of event payload
    (skip any remaining bytes of the L-byte block)
for each declared sample type with divisor ≠ 0:
    if (running sample count mod divisor) == 0: <size> bytes of sample data
```

The running sample count starts at 1 for the first sample, increments by one per sample,
and also increases by the "samples missed" count (event 10).

Sample types:

| Type | Size | Content |
|------|------|---------|
| 0 | 2 | temperature, u16le, 0.1 °C |
| 1 | 7 | deco/NDL: u8 flags (bit 1 = decompression stop, bit 2 = safety stop), u16le NDL or TTS, u16le stop depth (mbar absolute), u16le stop time |
| 2 | 2 | CNS, u16le, 0.01 % |

With a deco stop, the second field is the time to surface. Otherwise it is the
no-decompression limit. These time fields appear to be seconds.

Event types:

| Type | Size | Content |
|------|------|---------|
| 7 | 3 | gas change: gas id, O2 %, He % |
| 8 | 1 | alarm bitmask: bit 0 ascent rate, 1 ceiling, 2 ppO2, 3 max depth, 4 dive time, 5 CNS |
| 9 | 2 or 4 | setpoint change. The 2-byte form carries the setpoint in byte 0, unit 0.1 bar. The 4-byte form is not expected in samples |
| 10 | 6 | samples missed: u16le count, u32le absolute time stamp (s). Set the sample clock to the time stamp |

Unknown sample or event types can still be skipped, because their sizes are declared.

### Date and time

Six bytes at header offset 0x0C: year − 2000, month, day, hour, minute, second, in local
time. UTC offset: layout 0 with firmware `5B` or later stores (offset in hours + 12) at 0x12.
Layout 1 stores a signed count of 15-minute steps at 0x12.

Confidence: **likely** for layout 1 and the layout-0 header. **Uncertain** for layout-0
record types 1, 3 and 4 (not decoded by any known implementation) and for the units of the
deco time fields.

## Model-specific variations

- Firmware before 6 has no dive-index commands. Dives are numbered 1..N, with N the newest.
- Firmware `D01-4C` has the reduced layout-0 sample set described above.
- Rebadged models (Crest, Genesis, Scorpena) are believed to run the same firmware line.

## Open questions

1. GATT service and characteristic UUIDs, and whether writes need a response. Not in any
   public source.
2. How a 260-byte packet is carried over BLE: one notification at a large MTU, or several.
   Known hosts expect one read to return a whole packet.
3. Contents of the 6-byte hardware-version response and the 4-byte newest-index response
   beyond bytes 0..1.
4. Units of the layout-1 deco time fields (seconds assumed) and of layout-0 types 3 and 4.
5. Exact meaning of the "misc" byte in layout-0 records.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- Deep Six Excursion product page, <https://www.deepsixgear.com/>.
