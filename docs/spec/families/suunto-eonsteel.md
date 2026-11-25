# Suunto EON Steel, EON Core and D5

Suunto's current-generation dive computers expose a small remote file system. The host opens
the dive directory, lists it, and reads each dive file. Commands and replies share a 12-byte
header with a command word, a session "magic" value, a sequence number and a payload length.
Over USB the messages are carried in 64-byte HID interrupt reports; over Bluetooth Low Energy
the same messages are wrapped in an HDLC-style byte-stuffed frame with a CRC-32. Dive files use
Suunto's self-describing binary container **SBEM**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Suunto | EON Steel | none on the wire (host index 0) | USB HID, BLE | |
| Suunto | EON Core | none on the wire (host index 1) | USB HID, BLE | |
| Suunto | D5 | none on the wire (host index 2) | USB HID, BLE | |
| Suunto | EON Steel Black | none on the wire (host index 3) | USB HID, BLE | |

The device does not need a model code for the protocol. The host identifies the model from the
USB product ID or BLE name. One source reads a model byte from the init reply (see Session).

| Model | USB VID:PID | BLE advertised name (prefix match) |
|-------|-------------|------------------------------------|
| EON Steel | `1493:0030` | `EON Steel` |
| EON Core | `1493:0033` | `EON Core` |
| D5 | `1493:0035` | `Suunto D5` |
| EON Steel Black | `1493:0036` | `EON Steel Black` |

`EON Steel` is also a prefix of `EON Steel Black`; match the longer name first. One source
lists only `1493:0030`, `0033` and `0036` and labels `0036` as the D5; see open questions.

Confidence: **likely**.

## Transport parameters

### USB HID

- Vendor ID `0x1493` (Suunto).
- Data is exchanged as 64-byte interrupt transfers (OUT for host to device, IN for device to
  host). Byte 0 of every transfer is `3F`, which is also the HID report ID.
- Byte 1 is the number of valid bytes that follow (at most 62).
- Read timeout: the reference uses 5000 ms; another source uses 100 ms per report and tolerates
  up to 20 empty reads.
- See [../transports/](../transports/) for generic HID access.

### Bluetooth Low Energy

| Item | Value |
|------|-------|
| Service | `98ae7120-e62e-11e3-badd-0002a5d5c51b` |
| Notify characteristic (D→H) | `d0fd6b80-e62e-11e3-a2e9-0002a5d5c51b` |
| Write characteristic (H→D) | `c6339440-e62e-11e3-a5b3-0002a5d5c51b` |
| Write type | write with response |
| Chunk size | 20 bytes per write; no MTU negotiation needed |
| Pairing | the device is normally bonded first by the operating system |

The host enables notifications on the notify characteristic. The byte stream (in both
directions) is the HDLC-style framing described below, split into 20-byte writes and arriving
as notifications of up to 20 bytes. Frame boundaries do not have to coincide with chunk
boundaries.

Some hosts wait 300 ms before the first command and 500 ms after the init reply, and wait
50 ms before each BLE write.

Confidence: **likely** for the framing (both sources agree); **likely** for UUIDs (one source);
**uncertain** for the delays.

## Session

The host keeps two values: `magic` (u32) and `seq` (u16).

1. Start with `magic = 0x00000001`, `seq = 0`.
2. Send the init command `0x0000` with payload `02 00 2A 00`.
3. From the reply header take its magic field `m`; from now on send
   `magic = (m & 0xFFFF0000) | 0x0005`.
4. Every reply after init carries `magic + 5` (i.e. low half `0x000A`) and MUST be checked.
5. `seq` increases by one after every completed exchange, including init. Each reply carries
   the same `seq` as its request.

Init reply payload (first 48 bytes used):

| Offset | Type | Meaning |
|--------|------|---------|
| `0x10` | 16 bytes ASCII | serial number, decimal digits, NUL-padded |
| `0x20` | u32be | firmware version (reference interpretation) |
| `0x24` | u8 | model byte (other source): `0x44` = D5, `0x46` = EON Core, other = EON Steel |

The reference does not verify the command word or magic of the init reply, only its sequence
number.

There is no logout command; the host simply closes the HID device or BLE connection.

Confidence: **likely**.

## Commands

The command word is a u16le. On the wire its first (low) byte groups commands by area
(`03` clock, `10` file system) and the second (high) byte selects the operation within that
area; this grouping is an observation, not documented.

| Command | Wire bytes | Request payload | Reply payload | Notes |
|---------|-----------|-----------------|---------------|-------|
| Init | `00 00` | `02 00 2A 00` | device information (above) | |
| Set time | `03 00` | 8 bytes, see below | ignored | |
| Get time | `03 01` | | not decoded | |
| Set date | `03 02` | 8 bytes, see below | ignored | |
| Get date | `03 03` | | not decoded | |
| Read string | `11 04` | | not decoded | known but unused |
| File open | `10 00` | u32le `0`, path, `00` | ignored | path e.g. `0:/dives/5F8A1B2C.LOG` |
| File read | `10 01` | u32le `1234`, u32le byte count (≤ 1024) | u32le `1234`, u32le actual count, data | the first word is a handle, not an offset |
| File close | `10 05` | none | ignored | |
| File stat | `10 07` | none | file size as u32le at payload offset 4 | |
| Directory open | `10 08` | u32le `0`, path, `00` | ignored | path `0:/dives` |
| Directory read | `10 09` | none | u32le count, u32le last-flag, entries | repeat until last-flag ≠ 0 |
| Directory close | `10 0A` | none | ignored | |

Notes:

- File read: the first request word `1234` (`D2 04 00 00`) is echoed back unchanged and MUST be
  checked. Reads continue sequentially; an actual count of 0 ends the file.
- Directory open: the reference sends the path with a trailing `00`; another source sends
  `0:/dives` without it. Which form the device requires is not known.
- Setting the clock: the reference sends the same 8 bytes to both set-time and set-date:
  `year u16le, month, day, hour, minute, milliseconds u16le` (seconds × 1000).

Directory entry, repeated after the 8-byte prefix:

```
u32le  type      1 = file, 2 = directory
u32le  name_len
name_len bytes   name (ASCII)
00               terminator
```

Confidence: **likely**.

## Framing and integrity

### Message

```
+0   u16le  command
+2   u32le  magic
+6   u16le  sequence
+8   u32le  payload length N
+12  N bytes payload
```

Reply validation: same command word (except init), magic = sent magic + 5, same sequence,
`N` ≤ the host's buffer, and exactly `N` payload bytes received.

### USB HID carriage

Request: one 64-byte transfer `3F <12 + N> <message> <padding>`. A request MUST fit in one
transfer, so `N` ≤ 50. The reference also writes the message CRC-32 (below) into the 4 bytes
following the payload inside the padding, without counting them in the length byte; another
source pads with zeros. The device apparently ignores the padding.

Reply: one or more 64-byte transfers, each `3F <k> <k bytes>`. The first one starts with the
12-byte header. Concatenate until `12 + N` bytes have arrived. A transfer with `k` < 62 ends the
reply.

### BLE carriage

```
7E  stuff( message || crc32 )  7E
```

- `crc32` is CRC-32/ISO-HDLC (the zlib/Ethernet CRC: width 32, poly `0x04C11DB7`, init
  `0xFFFFFFFF`, reflected input and output, xorout `0xFFFFFFFF`) over the 12-byte header and
  the payload, appended **little-endian**. See
  [../common/checksums.md#crc-32-iso-hdlc](../common/checksums.md#crc-32-iso-hdlc).
  Check value: CRC over ASCII `123456789` = `CBF43926`.
- Stuffing: every `7E` or `7D` byte in the content is sent as `7D` followed by the byte XOR
  `0x20` (`7E` → `7D 5E`, `7D` → `7D 5D`).
- Receiver: discard bytes until a `7E`; then unstuff until the next `7E`. A `7D` followed by
  `7E` or by another `7D` is an error. The last 4 unstuffed bytes are the CRC.
- A whole reply arrives as one frame (the reference accepts payloads up to 2048 bytes). On CRC mismatch, purge
  and retry the command (one source retries up to 3 times).

### Worked example: init

Message (`cmd 0000`, `magic 00000001`, `seq 0000`, length 4, payload `02 00 2A 00`):

```
00 00 01 00 00 00 00 00 04 00 00 00 02 00 2A 00
```

CRC-32 of these 16 bytes is `0x309CA07A`, appended as `7A A0 9C 30`.

USB HID transfer (64 bytes, reference padding):

```
3F 10 00 00 01 00 00 00 00 00 04 00 00 00 02 00
2A 00 7A A0 9C 30 00 00 00 00 00 00 00 00 00 00
00 … 00
```

BLE frame (no byte needs stuffing), sent as two writes of 20 and 2 bytes:

```
7E 00 00 01 00 00 00 00 00 04 00 00 00 02 00 2A 00 7A A0 9C 30 7E
```

### Worked example: directory open over BLE

With `magic 0x12340005` and `seq 1`, payload `00 00 00 00` + `0:/dives` + `00` (13 bytes):

```
7E 10 08 05 00 34 12 01 00 0D 00 00 00 00 00 00 00
   30 3A 2F 64 69 76 65 73 00 A8 D1 AF D9 7E
```

Confidence: **likely** (CRC parameters and stuffing agree between sources; examples computed).

## Memory layout

Not applicable; the device presents a file system. Dives are files in `0:/dives`. Names are
`XXXXXXXX.LOG`, where `XXXXXXXX` is the dive start time as hexadecimal seconds since
1970-01-01 00:00, counted in the device's **local** wall-clock time (no time-zone offset is
applied). Because the hex digits are fixed-width, sorting names in descending order lists the
newest dive first. Sub-directories in `0:/dives` are ignored.

Example: `5F8A1B2C.LOG` → 0x5F8A1B2C = 1602886444 → 2020-10-16 22:14:04 local time.

Confidence: **likely**.

## Download procedure

```
init (Session)
directory open "0:/dives"
repeat: directory read, collect entries, until last-flag ≠ 0
directory close
sort file entries by name, descending
for each file entry:
    t := hex value of name before ".LOG"   (error if it does not parse)
    if t equals the stored fingerprint: stop
    file open "0:/dives/" + name
    file stat -> size
    while size > 0:
        file read (1234, min(size, 1024)) -> data; stop if empty
        append data; size -= len(data)
    file close
    emit (t as u32le) || file contents
```

Fingerprint: the 4-byte little-endian timestamp from the file name. A host may instead keep
the newest timestamp and download every file whose timestamp is greater.

Confidence: **likely**.

## Dive data format

### SBEM container

```
"SBEM" 00 00 00 00          8-byte file header
then repeated:
    descriptor entry
    zero or more data records
```

Descriptor entry:

```
00                 entry marker
L                  u8; if FF, a u32le length follows instead
id                 u16le descriptor ID (values below 512 observed)
text               L - 3 bytes of ASCII
00                 1 trailing byte
```

`L` counts the ID, the text and the trailing byte. The text holds one or more lines separated
by `\n`, each starting with a 5-character tag:

| Tag | Content |
|-----|---------|
| `<PTH>` | dotted path naming the field, e.g. `sml.DeviceLog.Samples.Sample.Depth` |
| `<GRP>` | comma-separated list of descriptor IDs whose values are packed together in order |
| `<FRM>` | format, e.g. `uint16,precision=2,nillable=65535`, `enum:0=Off,1=Primary,…`, `bool`, `utf8`, `float32` |
| `<MOD>` | modifier (purpose unknown) |

A later descriptor with the same ID replaces the earlier one.

Data record (repeated until the next byte is `00`, which starts a new descriptor entry):

```
T                  u8 descriptor ID; if FF, a u16le ID follows
len                u8; if FF, a u32le length follows
len bytes          value(s) encoded per the descriptor
```

Field sizes from `<FRM>`: `bool` and `enum…` = 1 byte; `utf8` = variable (the whole record);
otherwise the digits in the type name / 8 (`uint8` 1, `int16` 2, `uint32` 4, `float32` 4).
All integers and floats are little-endian; floats are IEEE-754 single precision.
`precision=k` in a format string is the number of decimal places of the SI value the raw
integer represents; `nillable=v` gives the "no value" sentinel.

A group record (`<GRP>` descriptor) contains the members' values back to back. Sample records
are typically groups that start with the time delta and depth.

### Header fields (path prefix `sml.DeviceLog.`)

| Path suffix | Encoding | Conversion / meaning |
|-------------|----------|----------------------|
| `Header.Duration` | u32 | dive duration, s |
| `Header.PauseDuration` | u32 | pause, s (one source subtracts it from the duration) |
| `Header.SampleInterval` | u8 | s |
| `Header.Depth.Max` | float32 | m |
| `Header.Depth.Avg` | float32 | m |
| `Header.DateTime` | utf8 | `YYYY-MM-DDThh:mm…` text |
| `Header.Activity` | utf8 | |
| `Header.Diving.SurfaceTime` | u32 | s |
| `Header.Diving.SurfacePressure` | u32 | Pa; / 100000 → bar |
| `Header.Diving.Algorithm` | utf8 | e.g. `Suunto Fused RGBM` |
| `Header.Diving.Conservatism` | i8 | personal adjustment, −2..+2 |
| `Header.Diving.DiveMode` | utf8 | e.g. `Air`, `Nitrox`, `CCR` |
| `Header.Diving.LowSetPoint`, `HighSetPoint` | u32 | Pa; / 100000 → bar |
| `Header.Diving.DesaturationTime` | u32 | s |
| `Header.Diving.Gases+Gas.State` | enum | **starts a new gas**: Off, Primary, Diluent, Oxygen, None (names from the enum text) |
| `Header.Diving.Gases.Gas.Oxygen` | u8 | O2 % of the current gas |
| `Header.Diving.Gases.Gas.Helium` | u8 | He % of the current gas |
| `Header.Diving.Gases.Gas.TankSize` | float32 | tank volume (see open questions) |
| `Header.Diving.Gases.Gas.TankFillPressure` | float32 | rated tank pressure (see open questions) |
| `Header.Diving.Gases.Gas.TransmitterID` | utf8 | tank pressure transmitter |
| `Header.Diving.Gases.Gas.TransmitterStartBatteryCharge`, `…EndBatteryCharge` | i8 | percent; 0 from older transmitters means unknown |
| `Device.SerialNumber`, `Device.Name`, `Device.Info.HW`, `Device.Info.SW`, `Device.Info.BSL`, `Device.Info.BatteryAtStart`, `Device.Info.BatteryAtEnd` | utf8 | device information |

The device also logs many further header fields (tissue state, CNS/OTU, altitude, set-point
switch depths, …); their names and formats are given by their own descriptors.

A `+` in a path marks the first field of a repeated element (a new gas, a new sample, a new
event), and `.` the following fields of the same element.

### Sample fields (path prefix `sml.DeviceLog.Samples`)

| Path suffix | Encoding | Raw unit | Conversion | Sentinel |
|-------------|----------|----------|------------|----------|
| `+Sample.Time` | u16 | ms | time **delta**; accumulate | |
| `.Sample.Depth` | u16 | cm | / 100 → m | `FFFF` |
| `.Sample.Temperature` | i16 | 0.1 °C | / 10 → °C | ≤ −3000 |
| `.Sample.NoDecTime` | i16 | min | no-decompression limit | −1 |
| `.Sample.Ceiling` | u16 | cm | / 100 → m | `FFFF` |
| `.Sample.TimeToSurface` | u16 | see open questions | | `FFFF` |
| `.Sample.Heading` | u16 | `precision=4` | see open questions | `FFFF` |
| `.Sample.DeviceInternalAbsPressure` | u16 | | not decoded | `FFFF` |
| `.Sample.GasTime` | i16 | see open questions | remaining gas time | −1 |
| `.Sample.Ventilation` | u16 | | not decoded | `FFFF` |
| `.Sample.Cylinders+Cylinder.GasNumber` | u8 | | 1-based gas number for the following pressure | |
| `.Sample.Cylinders.Cylinder.Pressure` | u16 | 0.01 bar | / 100 → bar | `FFFF` |
| `.Sample.Events+State.Type` / `.Events.State.Active` | enum / bool | | state change, see below | |
| `.Sample.Events+Notify.Type` / `.Events.Notify.Active` | enum / bool | | notification | |
| `.Sample.Events+Warning.Type` / `.Events.Warning.Active` | enum / bool | | warning | |
| `.Sample.Events+Alarm.Type` / `.Events.Alarm.Active` | enum / bool | | alarm | |
| `.Sample.Events.Bookmark.Name` | u16 | | bookmark | |
| `.Sample.Events.GasSwitch.GasNumber` (also seen as `Events+GasSwitch.GasNumber`) | u16 | | 1-based index into the header gas list | |
| `.Sample.Events+GasEdit.InsertGasNumber` / `.RemoveGasNumber` | u16 | | gas added/removed during the dive | |
| `.Sample.Events.SetPoint.Type` | enum | | Low, High, Custom: switch to that set point | |
| `.Sample.Events.Events.SetPoint.PO2` | u32 | Pa | custom set point, / 100000 → bar | |
| `.Sample.Events.SetPoint.Automatic` | bool | | | |
| `.Sample.Events.DiveTimer.Active` / `.Time` | | | not decoded | |

Events come in pairs: a `…Type` field (enum; the device's own `<FRM>` text gives the names)
followed by an `…Active` boolean (1 = began, 0 = ended). Enum names the device has been seen to
use:

- State: Wet Outside, Below Wet Activation Depth, Below Surface, Dive Active, Surface
  Calculation, Tank pressure available, Closed Circuit Mode.
- Notify: NoFly Time, Depth, Surface Time, Tissue Level, Deco, Deco Window, Safety Stop Ahead,
  Safety Stop, Safety Stop Broken, Deep Stop Ahead, Deep Stop, Dive Time, Gas Available,
  SetPoint Switch, Diluent Hypoxia, Air Time, Tank Pressure.
- Warning: ICD Penalty, Deep Stop Penalty, Mandatory Safety Stop, OTU250, OTU300, CNS80%,
  CNS100%, Max.Depth, Air Time, Tank Pressure, Safety Stop Broken, Deep Stop Broken, Ceiling
  Broken, PO2 High.
- Alarm: Mandatory Safety Stop Broken, Ascent Speed, Diluent Hyperoxia, Violated Deep Stop,
  Ceiling Broken, PO2 High, PO2 Low.

Hosts MUST resolve enum values through the descriptor text rather than hard-coding the numbers;
the numbering is defined per file.

Deco state: a sample with a valid NoDecTime is in no-deco mode; otherwise a non-zero ceiling or
time-to-surface indicates a decompression obligation.

Dive time: either `Header.Duration` (minus `PauseDuration`), or the sum of all sample time
deltas. Maximum depth: the larger of `Header.Depth.Max` and the deepest sample.

### Date and time

Use the timestamp from the file name (seconds since 1970 in local wall-clock time, no time
zone). `Header.DateTime` carries the same moment as text.

Confidence: **likely** for the container structure and sample encodings (both sources agree);
**uncertain** where noted.

## Model-specific variations

None at protocol level between EON Steel, EON Steel Black, EON Core and D5. Field sets inside
SBEM differ with firmware; hosts discover them from the descriptors.

## Open questions

1. USB product IDs: the reference lists `0x0035` = D5 and `0x0036` = EON Steel Black; another
   source lists only `0x0030`, `0x0033`, `0x0036` and suggests `0x0036` is the D5.
2. Model byte at init-reply offset `0x24` (`0x44` D5, `0x46` EON Core) comes from one source and
   overlaps the reference's 32-bit firmware field at `0x20..0x23` only by adjacency; not
   confirmed.
3. Units of `TimeToSurface`, `GasTime` (one reference comment says minutes, but its code divides
   by 60 as if seconds), `Heading` (`precision=4` suggests radians × 10⁴; treated as degrees by
   one implementation), `TankSize` and `TankFillPressure` (treated as litres and bar by one
   implementation, although other pressure fields are in pascal).
4. Temperature is signed in the reference and read as unsigned by another source.
5. Whether the trailing `00` in the directory-open path is required.
6. A second BLE service, `61353090-8231-49cc-b57a-886370740041` (notify
   `34802252-7185-4d5d-b431-630e7050e8f0`, write `17816557-5652-417f-909f-3aee61e5fa85`), is
   associated by one source with Suunto Spartan watches advertising as `Spartan`, using this
   same message protocol. Not confirmed.
7. Purpose of the `<MOD>` descriptor line and of the "read string" command `0x0411`.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>, used as
  a factual reference for additional SBEM sample names; no code copied.
- RFC 1662, PPP in HDLC-like Framing (byte stuffing with `7E`/`7D`),
  <https://www.rfc-editor.org/rfc/rfc1662>.
- Greg Cook, Catalogue of parametrised CRC algorithms (CRC-32/ISO-HDLC),
  <https://reveng.sourceforge.io/crc-catalogue/>.
