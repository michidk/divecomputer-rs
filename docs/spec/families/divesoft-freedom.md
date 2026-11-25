# Divesoft Freedom / Liberty

Divesoft Freedom dive computers and the Liberty closed-circuit rebreather controller store each
dive as a binary log made of a fixed header followed by 16-byte records. The same log can be
fetched two ways:

1. **Bluetooth Low Energy** (Bluetooth-capable hardware revisions). The host talks a
   message protocol: messages split into fragments, each fragment protected by a CRC-16 and
   wrapped in byte-stuffed `7E` frames.
2. **USB mass storage** (all hardware revisions with USB). The computer appears as a disk
   holding a `DIVELOG` directory with one `.DLF` file per dive.

A Divesoft developer has stated publicly that a `.DLF` file has the same format as a dive
downloaded over Bluetooth. The [Dive data format](#dive-data-format) section therefore covers
both paths.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Divesoft | Liberty (CCR controller), hardware 1.x | 1 | USB storage | no Bluetooth |
| Divesoft | Freedom, hardware 2.x | 2 | USB storage | no Bluetooth |
| Divesoft | Freedom, hardware 3.x | 5 | USB storage | no Bluetooth |
| Divesoft | Liberty, hardware 2.x | 10 | BLE, USB storage | |
| Divesoft | Freedom (incl. Freedom+), hardware 4.x | 19 | BLE, USB storage | |

The model code is byte 0 of the version response (see [Commands](#commands)).

Identification:

- BLE: the advertised name starts with `Freedom` or `Liberty` (case-insensitive prefix match).
- BLE service UUID `0000fcef-0000-1000-8000-00805f9b34fb` (16-bit UUID `0xFCEF`).
- USB mass storage: the USB VID:PID is not known (see [Open questions](#open-questions)).
  A host can recognise the volume by its `DIVELOG` directory of `.DLF` files.

Confidence: **likely** (model codes and BLE names from one reference implementation; service
UUID from a second open-source project's BLE table).

## Transport parameters

### BLE

- Service `0xFCEF` as above. The write and notify characteristics are not documented
  publicly. Known hosts use the first writable characteristic and the first
  notify-capable characteristic in that service.
- The host writes the byte-stuffed frame stream (see [Framing](#framing-and-integrity)) in
  chunks of at most **244 bytes**. Incoming notifications are concatenated and de-stuffed.
- Receive timeout: 3000 ms per frame.
- A serial-style line configuration of 115200 8N1 with no flow control is applied, but has
  no effect on BLE.

### USB mass storage

- A standard USB Mass Storage Class device with a FAT file system; see `../transports/`.
- Dive logs: directory `DIVELOG` in the volume root, file extension `.DLF`, one dive per
  file. File names are 8-digit numbers, for example `00000033.DLF`.
- The volume may also hold non-dive files: a system log (`.DSF` and `.CSV`) and a
  configuration (`.DCF` and `.CSV`). Some firmware also writes a `.CSV` text copy of each
  dive. A host only needs the `.DLF` files.

Confidence: **likely** for BLE parameters; **likely** for the storage layout (the user manual
and an independent implementation agree on the directory and extension).

## Session

BLE only:

1. Open the BLE link and subscribe to notifications.
2. Send **CONNECT** (message type 2) with payload `u16le compression` followed by an ASCII
   client name with no terminator. Known hosts send compression = 1. The client name is
   free-form.
3. The device answers with **CONNECTED** (type 3), a 36-byte payload:

   | Offset | Type | Meaning |
   |--------|------|---------|
   | 0x00 | u16le | compression accepted |
   | 0x02 | u8 | protocol version, major |
   | 0x03 | u8 | protocol version, minor |
   | 0x04 | 16 × ASCII | serial number |
   | 0x14 | 16 bytes | not decoded |

4. Optionally send **VERSION** (type 4) with an empty payload. The device answers with
   type 5 and a 26-byte payload:

   | Offset | Type | Meaning |
   |--------|------|---------|
   | 0x00 | u8 | model code (see [Devices](#devices)) |
   | 0x01 | u8 | hardware version, major |
   | 0x02 | u8 | hardware version, minor |
   | 0x03..0x05 | 3 × u8 | firmware version major, minor, patch |
   | 0x06 | u32le | firmware build number |
   | 0x0A | 16 × ASCII | serial number. Characters 5..15 are decimal digits that form the numeric serial |

No authentication or pairing step is known beyond normal BLE bonding. There is no explicit
disconnect message.

Confidence: **likely**.

## Commands

Every request gets exactly one response message. The response type is normally the request
type + 1. The dive-list request is the exception: it is answered with type 67 or type 71,
depending on the log format version.

| Request | Type | Request payload | Response type | Response payload |
|---------|------|-----------------|---------------|------------------|
| Echo | 0 | not used by known hosts | 1 | (generic result) |
| Connect | 2 | `u16le compression`, ASCII client name | 3 | 36 bytes, see above |
| Version | 4 | empty | 5 | 26 bytes, see above |
| Dive data | 64 | `u32le handle`, `u32le offset`, `u32le length` (12 bytes) | 65 | `length` bytes of the dive log, starting at `offset` |
| Dive list | 66 | `u32le start handle`, `u8 direction`, `u8 max count` (6 bytes) | 67 (v1 logs) or 71 (v2 logs) | list records, see below |

The **dive list** request:

- `start handle`: `0xFFFFFFFF` starts at the newest dive. Otherwise give the handle of the
  last record received in the previous batch.
- `direction`: 1 goes from newer to older dives (the only value used by known hosts).
- `max count`: known hosts use 100.

The response is a sequence of fixed-size records:

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 4 | `u32le` dive handle |
| 4 | 20 | dive fingerprint (opaque) |
| 24 | 32 (type 67) or 64 (type 71) | copy of the dive header (see [Header](#header)) |

Record size is therefore 56 bytes (v1) or 88 bytes (v2). Fewer than `max count` records
means the end of the list has been reached.

Confidence: **likely**.

## Framing and integrity

Two layers sit between a message and the BLE byte stream.

### Fragment (message layer)

A message payload is split into fragments of at most **256 payload bytes**. Each fragment
is:

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 1 | bits 0-3: message sequence number (mod 16). Bits 4-7: fragment index within the message (mod 16) |
| 1 | 1 | flags: bit 7 = 1 on host→device fragments. Bit 6 = 1 on the last fragment. All other bits 0 |
| 2 | 2 | `u16le` message type |
| 4 | 2 | `u16le` payload length of this fragment (`n`) |
| 6 | n | payload |
| 6+n | 2 | `u16le` CRC over bytes 0..5+n |

- The host keeps a 4-bit message counter. It increments the counter **before** each request,
  so the first message after connecting uses sequence number 1. All fragments of a request
  carry the same sequence number. The fragment index counts 0, 1, 2, … within the message.
- Device→host fragments echo the request's sequence number and number their own fragments
  from 0. They have flags `0x40` on the last fragment and `0x00` otherwise, so bit 7 is clear
  in that direction. Every fragment of one response carries the same message type.
- A receiver MUST check the sequence/index byte, the flags, the length field (it equals the
  fragment size minus 8) and the CRC.

CRC: **CRC-16/IBM-SDLC** (also known as CRC-16/X-25): width 16, poly 0x1021, init 0xFFFF,
refin true, refout true, xorout 0xFFFF. Check value for `"123456789"` = `0x906E`.
See [../common/checksums.md#crc-16-ibm-sdlc](../common/checksums.md#crc-16-ibm-sdlc).

### Byte stuffing (frame layer)

Each fragment travels in its own frame:

- The frame starts and ends with `7E`.
- Inside the frame, each `7E` or `7D` byte is sent as `7D` followed by the byte XOR `0x20`.
  For example, `7E` becomes `7D 5E` and `7D` becomes `7D 5D`.
- A receiver drops bytes before the first `7E`. It treats `7E` while an escape is pending,
  or `7D 7D`, as a framing error.

There is no address or control field. Despite the HDLC-like stuffing, this is not HDLC framing.

### Worked examples

CONNECT, the first message (sequence 1), payload `01 00` + ASCII `dcrs`:

```
fragment  01 C0 02 00 06 00 01 00 64 63 72 73 08 78
          |  |  |---| |---| |---------------| |---|
          |  |  type2  len6  payload           CRC 0x7808 (LE)
          |  flags C0: host→device, last fragment
          seq 1, fragment 0
H→D       7E 01 C0 02 00 06 00 01 00 64 63 72 73 08 78 7E
```

VERSION as message 2: `7E 02 C0 04 00 00 00 42 C6 7E`.

First DIVE LIST as message 3 (start `FFFFFFFF`, direction 1, count 100):
`7E 03 C0 42 00 06 00 FF FF FF FF 01 64 01 9B 7E`.

Escaping example: the fragment bytes `12 7E 34 7D` are sent as `7E 12 7D 5E 34 7D 5D 7E`.

Confidence: **likely**.

## Memory layout

Not applicable. The device exposes dives by handle (BLE) or as files (USB storage). It
offers no raw memory access.

## Download procedure

### BLE

```
connect(); optionally version()
records = []
start = 0xFFFFFFFF
loop:
    batch = dive_list(start, direction=1, max=100)
    for rec in batch:
        if rec.fingerprint == known_fingerprint: stop the whole list
        records.append(rec); start = rec.handle
    if len(batch) < 100: break
for rec in records (newest first):
    n = record count from rec.header (v1: u32le@16 bits 0-17; v2: u32le@20)
    length = header_size + 16 * n          # 32 or 64 + 16n
    dive = dive_data(rec.handle, offset=0, length)
    require dive[0:header_size] == rec.header
    hand dive to the parser; store rec.fingerprint
```

- The fingerprint for incremental download is the 20-byte opaque value from the list record.
  It is not part of the dive bytes.
- A host can read a dive in pieces by using the `offset` field. Known hosts read the whole
  dive at once and let the fragment layer split it.

### USB storage

1. Mount or read the FAT volume and list `DIVELOG/*.DLF` (case-insensitive).
2. For a quick "new dives only" filter, read the first 12 bytes of each file. Compare the
   `u32le` start time at offset 8 with the newest dive already known. The 20-byte BLE
   fingerprint is not stored in the file, so a file-based host needs its own fingerprint,
   for example the header start time or the file name.
3. Parse each file with the dive data format below.

Confidence: **likely** for BLE, **likely** for storage.

## Dive data format

A dive log (BLE dive-data payload or `.DLF` file) is a header followed by 16-byte records.
All integers are little-endian.

### Header

Two header versions exist, told apart by the first four bytes:

| Bytes 0..3 | ASCII | Version | Header size |
|-----------|-------|---------|-------------|
| `44 69 76 45` | `DivE` | v1 | 32 |
| `44 69 56 45` | `DiVE` | v2 | 64 |

Both versions carry a header checksum at offset 4: a `u16le` **CRC-16/MODBUS** (width 16,
poly 0x8005, init 0xFFFF, refin true, refout true, xorout 0x0000) computed over header bytes
6 to the end of the header (26 bytes for v1, 58 for v2). Check value for `"123456789"` =
`0x4B37`. See [../common/checksums.md#crc-16-modbus](../common/checksums.md#crc-16-modbus).

**v1 header (32 bytes)**

| Offset | Type | Raw unit | Conversion / meaning |
|--------|------|----------|----------------------|
| 0x00 | 4 bytes | | signature `DivE` |
| 0x04 | u16le | | header CRC (see above) |
| 0x08 | u32le | s | start time, seconds since 2000-01-01 00:00:00, local time |
| 0x0C | u32le | | bits 0-16: dive time in s. Bits 27-29: dive mode (table below) |
| 0x10 | u32le | | bits 0-17: number of 16-byte records. Bits 18-27: minimum temperature, signed 10-bit, 0.1 °C |
| 0x14 | u16le | cm | maximum depth; m = raw / 100 |
| 0x18 | u16le | 10 Pa | surface pressure; bar = raw / 10000 |
| 0x1A | u8 | % | initial diluent O2 (CCR modes) |
| 0x1B | u8 | % | initial diluent He (CCR modes) |
| other | | | not decoded |

**v2 header (64 bytes)**

| Offset | Type | Raw unit | Conversion / meaning |
|--------|------|----------|----------------------|
| 0x00 | 4 bytes | | signature `DiVE` |
| 0x04 | u16le | | header CRC |
| 0x08 | u32le | s | start time, seconds since 2000-01-01 00:00:00 UTC |
| 0x0C | u32le | s | dive time |
| 0x12 | u8 | | dive mode (table below) |
| 0x14 | u32le | | number of 16-byte records |
| 0x18 | i16le | 0.1 °C | minimum temperature |
| 0x1C | u16le | cm | maximum depth |
| 0x20 | u16le | 10 Pa | surface pressure; bar = raw / 10000 |
| 0x26 | u16le | cm | average depth |
| 0x28 | i16le | min | UTC offset of the device clock |
| 0x34 | 4 × ASCII | | serial number, first part |
| 0x38 | 8 × ASCII | | serial number, second part |
| other | | | not decoded |

Dive mode values:

| Value | Mode | Common model |
|-------|------|--------------|
| 0 | unknown | — |
| 1 | open circuit | OC |
| 2 | closed circuit | CCR |
| 3 | manual CCR | CCR |
| 4 | freediving | freedive |
| 5 | gauge / bottom timer | gauge |
| 6 | active SCR | SCR |
| 7 | passive SCR | SCR |
| 8 | bailout from CCR (open circuit on a CCR dive) | CCR |

Modes 2, 3, 6, 7 and 8 count as "rebreather" modes for gas-mix interpretation (see events
5/23/24 below).

File length = header size + 16 × record count.

### Records

Each record is 16 bytes. Records consisting entirely of `FF` bytes are padding; skip them.
The first `u32le` of every record is a common word:

| Bits | Meaning |
|------|---------|
| 0-3 | record class (table below) |
| 4-20 | time since dive start, s (17 bits) |
| 21-30 | record identifier (10 bits); meaning depends on the class |
| 31 | not used |

Time stamps should increase. Small backward steps of 1-5 s occur in real logs. Skip records
with such a step until time catches up, and treat a larger backward jump as corrupt data.

Record classes:

| Class | Meaning |
|-------|---------|
| 0 | profile point |
| 1 | event triggered by a diver action |
| 2 | automatic event |
| 3 | diver error / warning |
| 4 | internal error |
| 5 | activity event |
| 6 | configuration |
| 7 | measurement |
| 8 | decompression state (tissue loadings) |
| 9 | information event |

#### Class 0 – profile point

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| +4 | u16le | cm | depth |
| +6 | u16le | 10 Pa | ppO2; bar = raw / 10000. 0 = not available |

Identifier 0 (and 0x3FF in older firmware) marks the primary point record, which carries
more fields. Identifier 1 is a secondary point record whose remaining fields are not decoded.

| Offset | Type | Meaning |
|--------|------|---------|
| +8 | u32le | bits 0-9: no-decompression limit, min. Bits 10-19: time to surface, min. Bits 20-29: temperature, signed 10-bit, 0.1 °C |
| +12 | u16le | ceiling, cm. Non-zero means a decompression obligation |
| +15 | u8 | setpoint, 0.01 bar. 0 = none |

#### Classes 1-5 and 9 – events

The event code is `u16le` at +4. Codes are the same across these classes. The ones with a
known payload:

| Code | Event | Payload |
|------|-------|---------|
| 1 | setpoint changed manually | +6 u8 new setpoint, 0.01 bar |
| 2 | setpoint changed automatically | +6 u8 new setpoint, 0.01 bar |
| 5 | breathing gas changed | +6 u8 O2 %, +7 u8 He % (open-circuit gas) |
| 23 | diluent changed | +6 u8 O2 %, +7 u8 He % |
| 24 | dive mode changed | +6 u8 O2 %, +7 u8 He %, +8 u8 new mode. The gas is a diluent if the new mode is a rebreather mode, otherwise an open-circuit gas |
| 26 | bookmark set by the diver | — |
| 30 | CNS update | +6 u16le CNS, % |
| 55 | waypoint | +8 u16le compass heading, degrees. Bit 15 set = no heading |

Other codes are warnings and status notifications with no decoded payload: ascent too fast,
ceiling broken, ppO2 too high or too low, low battery, O2 sensor problems, solenoid
problems, stop completed, depth or time alarms, peer device lost or regained, and similar.
The full range seen in the field is 0-59.

#### Class 6 – configuration

The record identifier selects the content:

| Id | Content |
|----|---------|
| 2 | O2 sensor calibration: +4 4 × u16le, one per sensor. Each is a ppO2 per millivolt factor; with the mV readings below, ppO2 [Pa] = mV × factor (see id 3 measurement) |
| 3 | serial number: +4 4 × ASCII, +8 8 × ASCII |
| 4 | decompression settings: +4 u16le flags (bit 1 = salt water, bit 5 = VPM model; otherwise Bühlmann with gradient factors), +8 u8 GF low %, +9 u8 GF high % |
| 5 | firmware/hardware version: +4 model code, +5/+6 hardware major/minor, +7..+9 firmware major/minor/patch, +10 u16le flags, +12 u32le build |
| 7 | gas integration (one record per tank): +4 O2 %, +5 He %, +6 u16le tank volume 0.1 L, +8 u16le working pressure bar, +10 u8 transmitter slot (0-11), +11 u8 gas id (10 = oxygen of the rebreather, 11 = diluent, others = open-circuit gas) |
| 9 | diluent list: 4 entries of 3 bytes at +4: O2 %, He %, state (bit 0 = enabled, bits 1-7 = gas id) |
| 0, 1, 6, 8 | rebreather tests, ascent settings, CCR settings; not decoded |

Water density: salt water = 1028 kg/m³ and fresh water = 1000 kg/m³ when converting
pressure to depth. Depths in the log are already in cm, so this matters only for
re-computation.

Known firmware bug in id-7 records: some pre-release versions stored the oxygen and diluent
gas ids as 0-9 instead of 10/11. A gas id that is ≤ 9 and not larger than the previous
record's id should be read as id + 10.

#### Class 7 – measurement

| Id | Content |
|----|---------|
| 0 | ppO2 per sensor: +4 4 × u16le, 10 Pa (bar = raw / 10000). 0 or `FFFF` = no value |
| 1 | battery; not decoded |
| 2 | helium; not decoded |
| 3 | O2 sensor millivolts: +4 4 × u16le, 0.01 mV. +12 4 × u8 sensor state. Combine with the class 6 id 2 calibration factor: ppO2 [bar] = (raw / 100) × factor / 100000. Ignore sensors with state 4 (uncalibrated) or 8 (absent), or with a zero factor |
| 4 | position: +4 i32le latitude, +8 i32le longitude, both in 1e-6 degrees |
| 5 | ambient pressure; not decoded |
| 6 | gas consumption (SAC); not decoded |
| 7 | tank pressures: +4 12 × u8, one per transmitter slot, 2 bar per unit (bar = raw × 2). 0 or `FF` = no reading |
| 8 | display brightness; not decoded |
| 9 | gas-integration status; not decoded |

Sensor state values: 0 normal, 1 over range, 2 disabled, 3 excluded, 4 uncalibrated,
5 error, 6 offline, 7 inhibited, 8 not present.

#### Class 8 – decompression state

Tissue loading snapshots; not decoded. Skip them.

### Gas mixes and tanks

A dive's gas list is assembled from, in this order:

1. configuration id-7 records (gas integration);
2. enabled diluents from configuration id-9 records;
3. the initial diluent from the v1 header, in rebreather modes;
4. the gases named in events 5, 23 and 24.

Gases are de-duplicated by (O2, He, type), where type is open circuit, oxygen or diluent.
Tank pressures (measurement id 7) map to tanks by transmitter slot. A tank's working-pressure
or volume value above 400 bar or 99.0 L means "not set".

### Date and time

- Start time: `u32le` at header offset 8, seconds since 2000-01-01 00:00:00 (Unix time
  946684800).
- v1: the value is local wall-clock time; no zone is stored.
- v2: the value is UTC. Local time = value + (i16le at offset 0x28) × 60 s.

Confidence for the whole dive format: **likely** (one reference implementation). An
independent clean-room reading of `.DLF` files agrees on the v1 header (start time,
duration bits, minimum temperature bits, maximum depth), on the point record (time bits,
depth, temperature bits) and on configuration ids 3 and 4. It disagrees on the remaining
point-record fields (see [Open questions](#open-questions)).

## Model-specific variations

- Models 1, 2 and 5 (no Bluetooth) can only be read through USB storage.
- Log format v1 (`DivE`, 32-byte header) comes from older firmware. Format v2 (`DiVE`,
  64-byte header) adds UTC offset, average depth and serial number. Over BLE the dive-list
  response type tells the host which format it will get (67 → v1, 71 → v2).

## Open questions

1. **Point record, bytes +12..+15.** One implementation reads a `u16le` ceiling in cm at
   +12 and a `u8` setpoint in 0.01 bar at +15. Another reads `u32le` at +12 divided by 100
   as a decompression stop depth. These agree for small values only if +14/+15 are zero.
   Needs a real log with deco and a setpoint.
2. **BLE characteristics.** The write and notify characteristic UUIDs inside service
   `0xFCEF`, and whether writes need a response, are not publicly documented.
3. **USB VID:PID** of the mass-storage interface is not known.
4. **Point record id 1** and many event payloads (warnings, sensor events) are not decoded.
5. Whether the `CONNECT` compression value (1) changes the dive-data encoding. All known
   captures use 1 and receive uncompressed records.
6. Whether a dive log read over BLE has the same 20-byte fingerprint available in any form
   inside the `.DLF` file.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- Subsurface BLE transport (GPL-2.0), service UUID table,
  <https://github.com/subsurface/subsurface>, used as a factual reference; no code copied.
- Divesoft Freedom user manual (log directory and file types),
  <https://www.divesoft.com/en/manuals/freedom-user-manual/freedom-user-manual-02> and
  <https://www.scubadoctor.com.au/downloads/computer-manuals/Divesoft_Freedom_Bottom_Timer_User_Manual_EN.pdf>.
- Subsurface mailing list, statement that `.DLF` matches the Bluetooth format,
  <https://groups.google.com/g/subsurface-divelog/c/FWh8gwfE444>.
- RevEng CRC catalogue, <https://reveng.sourceforge.io/crc-catalogue/16.htm>.
