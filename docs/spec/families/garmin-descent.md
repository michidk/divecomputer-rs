# Garmin Descent and fēnix dive watches

Garmin's dive-capable watches (Descent Mk1/Mk2/Mk3/G1/G2/X50i and the fēnix 8 range) do
not expose a memory-dump protocol. Each dive is stored as one FIT activity file. A host
reads these files either from the watch's file system over USB (USB mass storage on the
Descent Mk1, MTP on later models) or, over Bluetooth Low Energy, through Garmin's GFDI
message protocol, which offers a file-transfer service. The dive content is the same in
every case, so this document has two halves: transports that deliver FIT files, and the
FIT messages and fields that carry dive data.

## Devices

Model codes are FIT `garmin_product` values. They appear in `file_id.product` (field 2)
and `device_info.product` (field 4, for `device_index` 0) of every activity file.

| Vendor | Model | Model code | USB mode | Notes |
|--------|-------|------------|----------|-------|
| Garmin | Descent Mk1 | 2859 | mass storage | Also has BLE GFDI |
| Garmin | Descent Mk1 (Asia/Pacific) | 2991 | mass storage | Not in the public FIT profile |
| Garmin | Descent Mk2 / Mk2i | 3258 | MTP | |
| Garmin | Descent Mk2S | 3542 | MTP | |
| Garmin | Descent Mk2 / Mk2i (Asia) | 3702 | MTP | |
| Garmin | Descent Mk2S (Asia) | 3930 | MTP | |
| Garmin | Descent G1 / G1 Solar | 4005 | MTP | |
| Garmin | Descent G1 (Asia) | 4132 | MTP (assumed) | In the FIT profile only |
| Garmin | Descent Mk3 (43 mm) | 4222 | MTP | FIT profile name: Mk3 |
| Garmin | Descent Mk3i (51 mm) | 4223 | MTP | FIT profile name: Mk3i |
| Garmin | Descent X50i | 4518 | MTP | Not in the public FIT profile |
| Garmin | fēnix 8 Solar 47 mm | 4532 | MTP | |
| Garmin | fēnix 8 Solar 51 mm (tactix 8 Solar) | 4533 | MTP | |
| Garmin | fēnix 8 43 mm | 4534 | MTP | |
| Garmin | fēnix 8 47/51 mm (also tactix/quatix 8) | 4536 | MTP | See open question 3 |
| Garmin | Descent G2 | 4588 | MTP | |
| Garmin | fēnix 8 Pro (also quatix 8 Pro) | 4631 | MTP | |
| Garmin | fēnix 8 47/51 mm AMOLED (tactix 8) | 4775 | MTP | See open question 3 |
| Garmin | fēnix 8 Solar 51 mm (tactix 8 Solar) | 4776 | MTP | |

Other dive-related products in the FIT profile, not dive computers that store dives
themselves: Descent T1 transceiver (3143), Descent T2 (4442), Descent X10 (5026, not
characterised). The fēnix E is a dive-capable model with no known model code.

FIT files may also come from other manufacturers. For example, `file_id.manufacturer` 23 is
Suunto, whose EON Core, EON Steel and D5 FIT exports can use the same dive messages.

Confidence: **verified** for codes that are in the public FIT profile (`garmin_product`
type); **likely** for 2991 and 4518 (one open-source reference only).

### Identification per transport

- **USB:** vendor ID `091E` (Garmin).
  - MTP mode: product ID = `0x4000 | model code`. Examples: Descent Mk2 3258 = `0x0CBA` →
    PID `4CBA`. fēnix 8 43 mm 4534 = `0x11B6` → PID `51B6`. Both PIDs also appear
    independently in a second source. **likely**
  - Mass-storage mode: two PIDs are known for the Descent Mk1, `2B2B` and `0003`. `2B2B` =
    `0x2000 | 2859` suggests that mass-storage PIDs are formed as `0x2000 | model code`.
    `0003` may be a generic Garmin mass-storage ID. **uncertain**
  - A host that reads MTP SHOULD accept only VID `091E` with bit `0x4000` set in the PID,
    and MAY additionally check the model code against the table above.
- **BLE:** Descent Mk1 advertises the name `Descent`, and a Mk2 variant advertises
  `Descent Mk2`. The service UUIDs are listed under
  [Transport parameters](#ble-gfdi). The watch must already be bonded with the host (pairing
  happens through the phone OS). **likely**

## Transport parameters

### USB mass storage

The watch presents a FAT-formatted volume. The host mounts it, or reads it through its own
SCSI/FAT stack. See [../transports/](../transports/) for the shared USB mass storage
details. Nothing Garmin-specific is sent. **likely**

### MTP

Standard PTP/MTP: OpenSession, GetStorageIDs, GetObjectHandles, GetObjectInfo, GetObject.
Folders are association objects (format `0x3001`), and FIT files have format `0x3000`
(undefined). See [../transports/](../transports/). **likely**

Variants seen in hosts:
- One host searches every storage for a `Garmin` folder.
- Another host uses only the first storage, and if nothing is found, retries the path with
  a `Primary/` prefix.

A robust host searches all storages and both paths.

### BLE GFDI

| Item | Value |
|------|-------|
| Service (GFDI "V1") | `6a4e2401-667b-11e3-949a-0800200c9a66` |
| Alternative service seen on Descent Mk2 | `6a4e8022-667b-11e3-949a-0800200c9a66` |
| Write characteristic (H→D) | `6a4e4c80-667b-11e3-949a-0800200c9a66` |
| Notify characteristic (D→H) | `6a4ecd28-667b-11e3-949a-0800200c9a66` |
| Older "V0" service | `9b012401-bc30-ce9a-e111-0f67e491abde`, write `df334c80-e6a7-d082-274d-78fc66f85e16`, notify `4acbcd28-7425-868e-f447-915c8f00d0cb` |
| Newer multi-link service | `6a4e2800-667b-11e3-949a-0800200c9a66` (RX `6a4e2810`…`2814`, same suffix). Not covered here. |
| Write type | Write with response |
| Write size | Frames are split into ATT-sized pieces: 20 bytes at the default MTU, larger after an MTU exchange. The receiver reassembles them by looking for the `0x00` delimiters. |
| Encryption | None on the V0/V1 service (beyond BLE bonding) |

Confidence: **likely** for the V1 UUIDs and write behaviour (two independent sources);
**uncertain** for `6a4e8022…` (one source) and for the multi-link service (one source, not
used by any dive host).

Newer firmware may require an authentication handshake (message IDs 5101–5112) before
file transfer. This is not described here (open question 6).

## Session

Over USB there is no session: the host lists and reads files.

Over BLE the watch initiates the GFDI conversation after notifications are enabled. The
host plays the "phone" role and MUST acknowledge each device message with a RESPONSE
(5000), or the watch stalls. The sequence used by one dive-log host (**likely**):

| Step | Direction | Message | Content |
|------|-----------|---------|---------|
| 1 | D→H | 5024 device information | Protocol version, product, unit id, software version, max packet size, three strings |
| 2 | H→D | 5000 response to 5024 | Status 0 plus the host's own identity (see below) |
| 3 | D→H | 5050 configuration | Capability bitmap |
| 4 | H→D | 5000 response to 5050 | Status 0 |
| 5 | H→D | 5050 configuration | Host capability bitmap: length `0A`, then `F9 FF DF FF FF FF FE CC DF 01`. One host appends 11 `00` bytes. |
| 6 | H→D | 5031 supported file types request | Empty payload |
| 7 | H→D | 5030 system event | Payload `00 01 00` (event type 0 = sync complete, followed by `01 00`; see open question 5) |
| 8 | D→H | 5043 protobuf request | Host acknowledges it (see below) |
| 9 | H→D | 5007 filter | Payload `03` |
| 10 | H→D | 5002 download request | File index 0 (the directory) |

Payload of the 5024 response (after the generic response header described under
[Commands](#commands)), all integers little-endian:

| Offset | Type | Example | Meaning |
|--------|------|---------|---------|
| 0 | u16le | `0070` = 112 | Host GFDI protocol version |
| 2 | u16le | `FFFF` | Host product number |
| 4 | u32le | 12345 | Host unit id |
| 8 | u16le | 4050 | Host software version |
| 10 | u16le | `4000` | Host maximum packet size |
| 12 | string ×3 | | Bluetooth name, manufacturer, model. Each string is a u8 length followed by that many bytes. |
| … | u8 | `01` | Protocol flags |

The device's own 5024 message has the same layout from offset 0 up to and including the
three strings. It may be followed by a "has MAC" byte and two 6-byte addresses.

The host must keep answering device-initiated messages for the whole session. Messages
seen during a download, all answered with a 5000 response with status 0 (ACK):
- 5036. One source names it "current time request" and another "notification
  subscription". The canned reply is just a status. Message 5052 is the real current-time
  request in the newer source.
- 5042 (music control capabilities).
- 5014 (weather request).
- 5043 and 5044 (protobuf request and response). The ack is followed by
  `<request id:u16le> <offset:u32le=0> <chunk status:u8=0> <status code:u8=0>`.

Confidence: **likely**. Byte sequences are from one source and checked with the CRC below;
the message names are from a second source.

## Commands

Every GFDI message, before COBS encoding, has this layout (all integers little-endian):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u16le | Total message length in bytes, counting this field, the id, the payload and the CRC |
| 2 | u16le | Message id. If bit 15 is set, the id is in compact form: id = 5000 + low byte, and bits 8–14 are a sequence number. |
| 4 | … | Payload |
| len−2 | u16le | CRC (see [Framing and integrity](#framing-and-integrity)) |

A **response** (id 5000) payload always starts with `<original id:u16le> <status:u8>`.
Status values: 0 ACK, 1 NAK, 2 unsupported, 3 decode error, 4 CRC error, 5 length error.

Messages used for log download:

| Id | Name | Dir | Payload (after id) |
|----|------|-----|--------------------|
| 5000 | Response | both | `orig_id:u16 status:u8`, then message-specific data |
| 5002 | Download request | H→D | `file_index:u16 offset:u32 request_type:u8 crc_seed:u16 size:u32`, optionally followed by `flags:u8` (bit 0 = compressed transfer; newer firmware) |
| 5000/5002 | Download response | D→H | `8A 13 status:u8 download_status:u8 file_size:u32` |
| 5004 | File transfer data | D→H | `flags:u8 crc:u16 offset:u32 data[n]` |
| 5000/5004 | Data acknowledgement | H→D | `8C 13 status:u8 transfer_status:u8 next_offset:u32` |
| 5007 | Filter | H→D | `filter_type:u8` (3 used) |
| 5024 | Device information | D→H | See [Session](#session) |
| 5030 | System event | H→D | `event_type:u8 value…` |
| 5031 | Supported file types request | H→D | Empty |
| 5050 | Configuration | both | `count:u8 bitmap[count]` |

Download request fields:
- `request_type`: 0 = continue, 1 = new. One host sends 1 for the directory and 0 for
  activity files. Another host sends 1 for both.
- `crc_seed` and `size`: 0 for a whole-file download.

Download status values: 0 OK, 1 index not found, 2 not readable, 3 not ready, 4 invalid
request, 5 CRC mismatch, 6 range exceeds file.

Transfer status values: 0 OK, 1 resend, 2 abort, 3 CRC mismatch, 4 offset mismatch,
5 sync paused.

The 5004 `crc` is a running CRC-16/ARC over the file data: seeded with the previous
chunk's CRC (0 before the first chunk) and run over this chunk's `data`. The value after
the last chunk is therefore the CRC of the whole file. Hosts SHOULD check it and check that
`offset` equals the number of bytes received so far. A dive-log host was seen that checks
neither.

Confidence: **likely**. The layouts agree between two sources; the optional `flags` byte
and the compact id are from one source only.

## Framing and integrity

**BLE framing.** Each message is COBS-encoded and wrapped in delimiters:
`00 <COBS(message)> 00`.

The COBS variant is standard (code byte `0xFF` marks a 254-byte run with no implied zero).
The leading `00` is not part of standard COBS framing but is always present.

A frame can span several notifications, and the receiver reassembles it up to the trailing
`00`. Data frames carry up to about 500 data bytes, so the encoder will emit `0xFF` codes
only if a chunk contains 254 consecutive non-zero bytes. A decoder MUST handle `0xFF` codes
correctly. One host's in-place decoder does not, and is wrong in that case.

**CRC.** CRC-16/ARC ([../common/checksums.md#crc-16-arc](../common/checksums.md#crc-16-arc)):

| Parameter | Value |
|-----------|-------|
| width | 16 |
| poly | 0x8005 (reflected form 0xA001) |
| init | 0x0000 |
| refin / refout | true / true |
| xorout | 0x0000 |
| check ("123456789") | 0xBB3D |
| Covered bytes | Message bytes from the length field up to the last payload byte (before COBS) |

This is the same CRC as the FIT file CRC. Garmin publishes it as a nibble-table algorithm
with table `0000 CC01 D801 1400 F001 3C00 2800 E401 A001 6C00 7800 B401 5000 9C01 8801 4400`.
Hosts SHOULD verify received CRCs. Responding with status 4 (CRC error) is allowed.

**Worked example 1: supported-file-types request (5031).**

```
message      06 00 A7 13 3B 75
             len=6, id=0x13A7 (5031), no payload, CRC-16/ARC(06 00 A7 13) = 0x753B
on the air   00 02 06 05 A7 13 3B 75 00
```

**Worked example 2: download request for the directory (file index 0, type new).**

```
message      13 00 8A 13 00 00 00 00 00 00 01 00 00 00 00 00 00 F5 41
             len=19, id=5002, index=0, offset=0, type=1, seed=0, size=0, CRC=0x41F5
on the air   00 02 13 03 8A 13 01 01 01 01 01 02 01 01 01 01 01 01 03 F5 41 00
```

**Worked example 3: acknowledgement of the first 502-byte chunk.**

```
message      0E 00 88 13 8C 13 00 00 F6 01 00 00 FE 59
             response to 5004, ACK, transfer OK, next offset 0x1F6 = 502, CRC 0x59FE
on the air   00 02 0E 05 88 13 8C 13 01 03 F6 01 01 03 FE 59 00
```

All three were recomputed with an independent CRC-16/ARC and COBS implementation.

Confidence: **verified** for the CRC algorithm (published in the FIT SDK) and for the
example frames (recomputed); **likely** for the framing rules.

## Memory layout

### Files on the USB volume

| Path | Contents |
|------|----------|
| `Garmin/Activity/*.fit` | One FIT activity file per activity (dives and other sports alike) |
| `Primary/Garmin/Activity/*.fit` | Same, on some MTP devices (one source) |

Directory and file names are matched case-insensitively: `GARMIN/ACTIVITY/…FIT` occurs.

File names are either of these:
- Long: `YYYY-MM-DD-HH-MM-SS.fit` (23 characters).
- 8.3 short: e.g. `C4ND0302.FIT`. The first four characters are base-36 digits (`0–9`,
  then `A–Z` = 10–35): year − 2010, month, day, hour. They are followed by the minute and
  second as two decimal digits each. Example: `C4ND0302` = 2022-04-23 13:03:02.

The name appears to give the start time of the activity. Whether that time is local or
UTC is not confirmed (**uncertain**). Hosts should take dive times from the FIT content,
not from the name.

### Directory file over GFDI

File index 0 is the directory. Its content is a sequence of 16-byte entries:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u16le | File index (use in download request) |
| 0x02 | u8 | Data type (128 = FIT file) |
| 0x03 | u8 | Sub-type (for FIT, the FIT `file` type: 4 = activity) |
| 0x04 | u16le | File number |
| 0x06 | u8 | Type-specific flags |
| 0x07 | u8 | File flags |
| 0x08 | u32le | File size in bytes |
| 0x0C | u32le | File date, seconds since the FIT epoch (1989-12-31 00:00 UTC) |

The directory length is a multiple of 16. Dive logs are entries with data type 128 and
sub-type 4. Index `0xFFFD` is a special device XML file.

Confidence: **likely** (two sources agree on the layout).

## Download procedure

### USB (mass storage or MTP)

```
list all files in Garmin/Activity whose name ends in ".fit" (any case)
sort by start time, newest first
    (for short names, decode the base-36 timestamp before comparing)
for each file:
    if file name == stored fingerprint: stop
    read whole file
    if not a dive (see "Dive detection"): skip
    deliver file; first delivered file's name becomes the new fingerprint
```

Fingerprint: one implementation uses the file name as the fingerprint, in a 24-byte
zero-padded field (`YYYY-MM-DD-HH-MM-SS.fit` + NUL). Another stores the start time of the
dive (from the first `record` timestamp or `lap.start_time`) and drops files whose start
is not later. Both work. The name-based variant needs no FIT parsing to skip old files.
**likely**

### BLE

```
handshake (see Session)
download file index 0 (the directory), collecting 5004 chunks
for each 16-byte entry with data type 128 and sub-type 4, index > 0:
    optionally skip by date (entry offset 0x0C) against the fingerprint
    send 5002 download request for that index
    expect 5000 response: status 0, download status 0, file_size
    repeat until file_size bytes received:
        receive 5004 (skip and ack unrelated device messages in between)
        check offset and running CRC
        send 5000 ack with next_offset = bytes received so far
    parse file as FIT; keep if it is a dive
```

One host ignores the last directory entry because of an off-by-one bound. Hosts MUST
process all entries. **likely**

### Dive detection

A FIT activity is a dive if its `sport` message (12), or its `session` message (18), has:
- `sport` = 53 (diving), or
- `sub_sport` in {53 single-gas, 54 multi-gas, 55 gauge, 56 apnea, 57 apnea hunting,
  63 CCR}.

These values are in the public FIT profile.

A fallback used by one host: treat the file as a dive if a `dive_summary` message has a
valid `avg_depth`. **verified** (values from the FIT profile)

## Dive data format

### FIT file structure

All facts in this subsection are from the public FIT protocol and SDK (**verified**),
except where marked.

File header:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u8 | Header size: 12 or 14 |
| 0x01 | u8 | Protocol version |
| 0x02 | u16le | Profile version: up to 2199 it is major × 100 + minor; above that, major × 1000 + minor (e.g. 21222 = 21.222) |
| 0x04 | u32le | Data size: bytes of records after the header, excluding the file CRC |
| 0x08 | 4 bytes | ASCII `.FIT` |
| 0x0C | u16le | Header CRC over bytes 0–11 (14-byte header only; 0 = not computed) |

The file ends with a u16le CRC over everything before it (header and records). It uses the
same CRC-16/ARC as GFDI. Running that CRC over the whole file including the trailing CRC
gives 0.

Each record starts with a header byte:

| Bits | Normal header (bit 7 = 0) | Compressed-timestamp header (bit 7 = 1) |
|------|---------------------------|------------------------------------------|
| 7 | 0 | 1 |
| 6 | 1 = definition message, 0 = data message | Bits 6–5: local message type (0–3) |
| 5 | Definition only: 1 = developer fields follow | (part of local type) |
| 4 | Reserved | Bits 4–0: time offset in seconds |
| 3–0 | Local message type (0–15) | (part of offset) |

Garmin's reference decoders reject compressed-timestamp headers. Rollover rule for an
offset `o` against the last full timestamp `t` (**likely**, from an open-source decoder):
new time = `(t & ~0x1F) | o`, plus `0x20` if that is less than `t`.

Definition message, after the header byte:

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u8 | Reserved |
| 1 | u8 | Architecture: 0 = little-endian, 1 = big-endian. Applies to this message's multi-byte fields. |
| 2 | u16 | Global message number (in the stated byte order) |
| 4 | u8 | Number of fields N |
| 5 | N × 3 | Per field: field number, size in bytes, base type |
| … | u8 + M × 3 | Only if header bit 5 is set: M developer fields (field number, size, developer data index) |

Data messages contain the field values in definition order, then the developer field
bytes. A field's size may be a multiple of the base-type size (an array). Unknown messages
and fields MUST be skipped using the sizes from the definition.

Base types (low 5 bits of the base-type byte; bit 7 marks multi-byte types):

| Id | Type | Size | Invalid value |
|----|------|------|---------------|
| 0x00 | enum | 1 | 0xFF |
| 0x01 | sint8 | 1 | 0x7F |
| 0x02 | uint8 | 1 | 0xFF |
| 0x03 | sint16 | 2 | 0x7FFF |
| 0x04 | uint16 | 2 | 0xFFFF |
| 0x05 | sint32 | 4 | 0x7FFFFFFF |
| 0x06 | uint32 | 4 | 0xFFFFFFFF |
| 0x07 | string | 1 | 0x00 (NUL-terminated, UTF-8) |
| 0x08 | float32 | 4 | 0xFFFFFFFF |
| 0x09 | float64 | 8 | all ones |
| 0x0A | uint8z | 1 | 0x00 |
| 0x0B | uint16z | 2 | 0x0000 |
| 0x0C | uint32z | 4 | 0x00000000 |
| 0x0D | byte | 1 | 0xFF |
| 0x0E | sint64 | 8 | 0x7FFF…FF |
| 0x0F | uint64 | 8 | all ones |
| 0x10 | uint64z | 8 | 0 |

A field holding its invalid value is absent. Field numbers 253 (timestamp, uint32),
254 (message_index, uint16) and 250 (part_index) have the same meaning in every message.

Worked example: a minimal FIT file with one `record` (timestamp 2024-06-01 10:00:00 UTC,
depth 12.345 m, 18 °C):

```
0E 20 54 08 19 00 00 00 2E 46 49 54 78 B7    header: size 14, proto 0x20, profile 2132,
                                             data 25 bytes, ".FIT", header CRC 0xB778
40 00 00 14 00 03 FD 04 86 5C 04 86 0D 01 01 definition, local 0, LE, mesg 20, 3 fields:
                                             253/4/uint32, 92/4/uint32, 13/1/sint8
00 20 A5 BD 40 39 30 00 00 12                data, local 0: ts 0x40BDA520, depth 12345, temp 18
AB 93                                        file CRC 0x93AB
```

### Header (summary values)

Message and field numbers, types and scales are from the public FIT profile (**verified**).
"Use" notes how dive-log hosts map them.

**file_id (0):**

| Field | Type | Scale / unit | Use |
|-------|------|--------------|-----|
| 0 type | enum | 4 = activity | File kind |
| 1 manufacturer | uint16 | 1 = Garmin, 23 = Suunto | |
| 2 product | uint16 | `garmin_product` | Model code |
| 3 serial_number | uint32z | | Serial |
| 4 time_created | uint32 | FIT time | |

**device_info (23):**
- `device_index` 0 is the watch itself.
- Fields: 3 serial_number (uint32z), 4 product (uint16), 5 software_version (uint16,
  scale 100, e.g. 1234 = 12.34).

**sport (12):** 0 sport (53 = diving), 1 sub_sport (see Dive detection).

**session (18):** 2 start_time (FIT time; one host uses this as dive start), 3/4 start
position lat/long, 38/39 end position lat/long (sint32 semicircles), 5 sport, 6 sub_sport,
7 total_elapsed_time (uint32, 1/1000 s). Dive-specific: 143 start_cns, 144 end_cns (uint8 %),
145 start_n2, 146 end_n2 (uint16 %), 150 min_temperature (sint8 °C), 155 o2_toxicity
(uint16 OTU), 156 dive_number (uint32).

**lap (19):** 2 start_time, 3/4 start and 5/6 end position. One host uses lap start_time
as the dive start.

**dive_settings (258):**

| Field | Type | Scale / unit | Meaning |
|-------|------|--------------|---------|
| 1 model | enum | 0 = Bühlmann ZHL-16C | Deco model |
| 2 gf_low, 3 gf_high | uint8 | % | Gradient factors |
| 4 water_type | enum | 0 fresh, 1 salt, 2 EN 13319, 3 custom | Salinity |
| 5 water_density | float32 | kg/m³ | For custom water (fresh is usually 1000, salt 1025) |
| 6 po2_warn, 7 po2_critical, 8 po2_deco | uint8 | 1/100 bar | ppO₂ limits |
| 9 safety_stop_enabled | enum (bool) | | |
| 12 apnea_countdown_enabled, 13 apnea_countdown_time | bool, uint32 s | | |
| 17 repeat_dive_interval | uint16 | s | Surface time before the activity ends |
| 18 safety_stop_time | uint16 | s | |
| 19 heart_rate_source_type | enum | 5 = wrist, 1 = ANT+ | |
| 21 travel_gas | uint16 | message_index of a dive_gas | |
| 22 / 25 ccr_low / high_setpoint_switch_mode | enum | 0 manual, 1 automatic | |
| 23 / 26 ccr_low / high_setpoint | uint8 | 1/100 bar | CCR setpoints |
| 24 / 27 ccr_low / high_setpoint_depth | uint32 | 1/1000 m | Auto-switch depths |
| 36 last_stop_multiple | uint8 | 1/10 (1.0 = 3 m) | |

**dive_gas (259)**, one message per gas, indexed by field 254 message_index:

| Field | Type | Unit | Meaning |
|-------|------|------|---------|
| 0 helium_content | uint8 | % | He fraction × 100 |
| 1 oxygen_content | uint8 | % | O₂ fraction × 100 |
| 2 status | enum | 0 disabled, 1 enabled, 2 backup only | |
| 3 mode | enum | 0 open circuit, 1 CCR diluent | |

Hosts differ: one keeps only status 1, another keeps status 1 and 2. Keeping both is
RECOMMENDED. Use the message_index as the gas number, because event 57 refers to it.

**dive_alarm (262)** and **dive_apnea_alarm (393)** (configuration, not occurrences):
- 0 depth (uint32, 1/1000 m)
- 1 time (sint32 s)
- 2 enabled
- 3 alarm_type (0 depth, 1 time, 2 speed)
- 4 sound
- 5 dive_types (array of sub_sport)
- 6 id, 7 popup_enabled, 8 trigger_on_descent, 9 trigger_on_ascent, 10 repeating

**dive_summary (268)**: one message per dive. There may be a further one summarising the
whole activity: field 0 `reference_mesg` says whether it refers to a lap or a session, and
field 1 `reference_index` says which one.

| Field | Type | Scale / unit | Meaning |
|-------|------|--------------|---------|
| 2 avg_depth | uint32 | 1/1000 m | |
| 3 max_depth | uint32 | 1/1000 m | |
| 4 surface_interval | uint32 | s | Since previous dive |
| 5 start_cns, 6 end_cns | uint8 | % | |
| 7 start_n2, 8 end_n2 | uint16 | % | Tissue load |
| 9 o2_toxicity | uint16 | OTU | |
| 10 dive_number | uint32 | | |
| 11 bottom_time | uint32 | 1/1000 s | Dive time |
| 12 avg_pressure_sac | uint16 | 1/100 bar/min | |
| 13 avg_volume_sac | uint16 | 1/100 L/min | |
| 14 avg_rmv | uint16 | 1/100 L/min | |
| 15 descent_time, 16 ascent_time, 25 hang_time | uint32 | 1/1000 s | |
| 17 avg_ascent_rate | sint32 | 1/1000 m/s | |
| 22 avg_descent_rate, 23 max_ascent_rate, 24 max_descent_rate | uint32 | 1/1000 m/s | |

**tank_summary (323):** 0 sensor (uint32z ANT channel id of the tank pod), 1 start_pressure,
2 end_pressure (uint16, 1/100 bar), 3 volume_used (uint32, 1/100 L).

**Tank pod configuration:** message 147, sensor settings (**likely**, not in the public
profile):
- 0 sensor id (uint32z)
- 2 name (string)
- 3 enabled
- 52 sensor type (28 = tank pod)
- 74 pressure units (0 psi, 1 kPa, 2 bar)
- 75 rated pressure, 76 reserve pressure (uint16)
- 77 volume (uint16, 1/10 L or 1/10 ft³)
- 78 used for gas-consumption calculation

The order of the tank pods gives the tank index.

### Samples

Samples are `record` (20) messages, normally once per second. Each sample's time is its
timestamp (field 253, or a compressed-timestamp header). The time offset into the dive is
the timestamp minus the dive start. A field that is absent or holds its invalid value in a
record keeps its previous value.

| Field | Type | Scale / unit | Meaning |
|-------|------|--------------|---------|
| 0 / 1 position_lat / long | sint32 | semicircles: deg = v × 180 / 2³¹ | Surface GPS fix |
| 3 heart_rate | uint8 | bpm | |
| 13 temperature | sint8 | °C | Water temperature, 1 °C resolution |
| 91 absolute_pressure | uint32 | Pa | Includes atmosphere |
| 92 depth | uint32 | 1/1000 m | 0 above water |
| 93 next_stop_depth | uint32 | 1/1000 m | Deco ceiling / next stop |
| 94 next_stop_time | uint32 | s | |
| 95 time_to_surface | uint32 | s | TTS |
| 96 ndl_time | uint32 | s | No-deco limit |
| 97 cns_load | uint8 | % | CNS |
| 98 n2_load | uint16 | % | Leading tissue N₂ load |
| 123 air_time_remaining | uint32 | s | |
| 124 pressure_sac | uint16 | 1/100 bar/min | |
| 125 volume_sac | uint16 | 1/100 L/min | |
| 126 rmv | uint16 | 1/100 L/min | |
| 127 ascent_rate | sint32 | 1/1000 m/s | |
| 129 po2 | uint8 | 1/100 bar | Current ppO₂ |

Field 129 is recent in the profile, and neither open-source reference reads it.

**tank_update (319)**: tank pressure samples.
- Fields: 253 timestamp, 0 sensor (uint32z, matches the tank pod id), 1 pressure (uint16,
  1/100 bar).
- Use the timestamp to place the value in the profile.
- The sensor id selects the tank. One host ignores it and assigns the pressure to the
  current gas.

**event (21)** carries dive events and gas switches:

| `event` (field 0) | Meaning | `data` (field 3, uint32) |
|-------------------|---------|--------------------------|
| 56 dive alert | Alert (marker) | Alert code, see below |
| 57 dive gas switched | Gas switch | dive_gas message_index of the new gas |
| 71 tank pressure reserve | Tank reached reserve | Sensor id |
| 72 tank pressure critical | | Sensor id |
| 73 tank lost | Tank pod signal lost | Sensor id |
| 76 tank battery low | | Sensor id |
| 0 timer | Start/stop of the activity (field 1 event_type: 0 start, 1 stop, 4 stop all) | |

One open-source reference also reads tank sensor ids from event fields 71–73. Those fields
are not in the public profile (**uncertain**). Field 15 is `start_timestamp`.

Dive alert codes (value of `data` with event 56):

| Code | Meaning | Code | Meaning |
|------|---------|------|---------|
| 0 | NDL reached / deco required | 20 | Battery low |
| 1 | Gas switch prompted | 21 | Battery critical |
| 2 | Near surface | 22 | Safety stop started |
| 3 | Approaching NDL | 23 | Approaching first deco stop |
| 4 | ppO₂ warning | 24 | Setpoint auto-switch to low |
| 5 | ppO₂ critical high | 25 | Setpoint auto-switch to high |
| 6 | ppO₂ critical low | 26 | Setpoint manual switch to low |
| 7 | Time alert | 27 | Setpoint manual switch to high |
| 8 | Depth alert | 28 | Auto setpoint switch ignored |
| 9 | Deco ceiling broken | 29 | Switched to open circuit (bailout) |
| 10 | Deco complete | 30 | Switched to closed circuit |
| 11 | Safety stop ceiling broken | 32 | Tank battery low |
| 12 | Safety stop complete | 33 | ppO₂ low on CCR diluent |
| 13 | CNS warning | 34 | Deco stop cleared |
| 14 | CNS critical | 35 | Apnea neutral buoyancy |
| 15 | OTU warning | 36 | Apnea target depth |
| 16 | OTU critical | 37 | Apnea surface |
| 17 | Ascent speed critical | 38 | Apnea high speed |
| 18 | Alert dismissed by key | 39 | Apnea low speed |
| 19 | Alert dismissed by timeout | | |

Codes 24–27 change the active CCR setpoint: low/high values come from dive_settings 23/26.
The dive starts on the low setpoint. (**verified** for the codes; **likely** for the
setpoint interpretation.)

**Several dives in one file:** apnea sessions record many dives in one activity. Each dive
is closed by a `dive_summary`. Records that follow a `dive_summary` belong to the next dive.
One host splits the file there. Another reports the file as one dive. **likely**

### Date and time

- FIT `date_time` (uint32) = seconds since 1989-12-31 00:00:00 UTC. Unix time = value +
  631065600. Values below `0x10000000` are not absolute: they are seconds since the device
  was powered on. **verified**
- Dive start: `session.start_time`. Alternatives in use: `lap.start_time`, or the first
  `record` timestamp. All are UTC.
- Local time. Sources, in order of preference:
  - `activity` (34) field 5 `local_timestamp` (type `local_date_time`): offset = local −
    the message's own timestamp.
  - `timestamp_correlation` (162) field 3 `local_timestamp`: same rule.
  - `device_settings` (2): field 1 utc_offset and field 2 time_offset (uint32, used as
    signed seconds). Field 2 may be 0 even when local time differs.
- One host accepts the resulting zone offset only if it is a multiple of 900 s between
  −12 h and +14 h. **likely**

## Model-specific variations

- Descent Mk1: USB mass storage plus BLE GFDI. Every later model: MTP.
- Air-integrated models write tank pods (sensor settings, tank_update, tank_summary) and
  the SAC/RMV fields.
- CCR-capable firmware writes sub_sport 63, setpoint fields 22–27 and alerts 24–30. Which
  models support CCR is not established here.
- The fields present vary with firmware. Hosts MUST rely on FIT definitions, never on fixed
  offsets.

## Open questions

1. Mass-storage PID rule (`0x2000 | model code`?) and the meaning of PID `0003`.
2. Whether the `Primary/` folder prefix is real on any device, or an artefact of one host.
3. Product codes 4536 and 4775: one source names 4536 the Asia/Pacific fēnix 8 47/51 mm and
   4775 the regular model. The FIT profile names 4536 "fenix8" and 4775 "tactix8 AMOLED".
   The same disagreement exists for 4776.
4. GFDI message 5036: current-time request (one source) or notification subscription
   (other source). Its correct reply payload is unknown. A bare ACK appears to suffice.
5. Meaning of the 5030 system-event payload `00 01 00`, the 5007 filter value 3, and the
   host capability bitmap.
6. Which firmware requires GFDI authentication (5101+) or the multi-link service
   `6a4e2800…`. Whether a Descent Mk2 or later accepts the V1 flow described here.
7. Whether compressed (flag bit 0) download transfers are offered by dive watches, and
   their format.
8. Exact rule for splitting multi-dive (apnea) activities.

## References

- Garmin FIT SDK and FIT protocol documentation, https://developer.garmin.com/fit/
  (protocol, file types and the `Profile.xlsx` message/type definitions; the same profile
  is in Garmin's public SDK repositories, e.g.
  https://github.com/garmin/fit-javascript-sdk, profile version 21.222).
- libdivecomputer, Subsurface fork (LGPL-2.1), https://github.com/subsurface/libdc. Used as
  a factual reference only; no code copied.
- Gadgetbridge (AGPL-3.0), https://codeberg.org/Freeyourgadget/Gadgetbridge. Its Garmin
  GFDI support was used as a factual reference only; no code copied.
- RevEng CRC catalogue, CRC-16/ARC, https://reveng.sourceforge.io/crc-catalogue/16.htm
