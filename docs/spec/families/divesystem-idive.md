# DiveSystem / Ratio iDive family (iDive, iX3M, iX3M 2021, iDive 2, iX3M 2, ATOM, Seac Guru/Jack)

Computers designed by DiveSystem and sold as DiveSystem, Ratio and Seac. One request/reply
protocol runs over USB-serial, Bluetooth classic (RFCOMM serial port) and Bluetooth Low
Energy. Every message is a frame with a `55` start byte, a length and a big-endian
CRC-16. Dives are addressed by dive number, not by memory address. The host fetches a dive
header, then sample records by sample index. There are two command sets: the original iDive
set and the iX3M set. The iX3M set also has a variant for firmware major version 4 and later
("APOS4").

## Devices

Sorted by model code (u16 from the identity reply).

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| DiveSystem | Orca | 0x02 | serial | iDive set |
| DiveSystem | iDive Pro | 0x03 | serial | iDive set |
| DiveSystem | iDive DAN | 0x04 | serial | iDive set |
| DiveSystem | iDive Tech | 0x05 | serial | iDive set |
| DiveSystem | iDive Reb | 0x06 | serial | iDive set |
| DiveSystem | iDive Stealth | 0x07 | serial | iDive set |
| DiveSystem | iDive Free | 0x08 | serial | iDive set |
| DiveSystem | iDive Easy | 0x09 | serial | iDive set |
| DiveSystem | iDive X3M | 0x0A | serial | iDive set |
| DiveSystem | iDive Deep | 0x0B | serial | iDive set |
| Ratio | iX3M GPS Pro / Easy / Deep / Tech+ / Reb / Fancy | 0x21..0x26 | serial, BT classic | iX3M set |
| Ratio | iX3M Pro Fancy / Easy / Pro / Deep / Tech+ / Reb | 0x31..0x36 | serial | |
| Ratio | iDive Free / Fancy / Easy / Pro / Deep / Tech+ / Reb | 0x40..0x46 | serial | |
| Ratio | iDive Color Free / Fancy / Easy / Pro / Deep / Tech+ / Reb | 0x50..0x56 | serial | |
| Ratio | iX3M 2021 GPS Fancy / Easy / Pro / Deep / Tech+ / Reb | 0x60..0x65 | serial, BLE | |
| Ratio | iX3M 2021 Pro Fancy / Easy / Pro / Deep / Tech+ / Reb | 0x70..0x75 | serial | |
| Ratio | iDive 2 Free / Fancy / Easy / Pro / Deep / Tech / Reb | 0x80..0x86 | serial, BLE | |
| Ratio | iX3M 2 GPS Gauge / Easy / Pro / Deep / Tech / Reb | 0x90..0x95 | serial, BLE | |
| Ratio | ATOM | 0x96 | serial, BLE | |
| Ratio | iX3M 2 Gauge / Easy / Pro / Deep / Tech+ | 0x100..0x104 | serial, BLE | |
| Seac | Jack | 0x1000 | serial | |
| Seac | Guru | 0x1002 | serial | |

Every model with code ≥ 0x21 uses the iX3M command set. Codes 0x60..0xFFF form the "iX3M 2
generation" for the decompression model codes (see [Samples](#samples)). Another reference
also names 0x41 "iDive Fancy" and 0x45 "iDive Tec".

Identification:
- **Bluetooth classic and BLE:** the advertised name is `DS`, `IX5M` or `RATIO-` followed by
  decimal digits only (case-insensitive prefix). Another reference expects `IX3M…` or
  `DS…` over Bluetooth classic and `IX5M…` or `RATIO-…` over BLE.
- **Serial:** USB-serial cable. No VID:PID is established by public sources.

Confidence: **likely**.

## Transport parameters

### Serial

115200 baud, 8 data bits, no parity, 1 stop bit, no flow control. DTR and RTS are not
changed. After opening, wait about 300 ms and discard pending input and output. Read
timeout about 1000 ms.

### Bluetooth classic

RFCOMM with the Serial Port Profile UUID `00001101-0000-1000-8000-00805F9B34FB`. One reference
uses an insecure (unauthenticated) socket. The byte protocol is the same as over serial.

### BLE

Microchip "Transparent UART" service (BM70/RN4870 modules):

| Role | UUID |
|------|------|
| Service | `49535343-fe7d-4ae5-8fa9-9fafd205e455` |
| Notify (D→H) | `49535343-1e4d-4bd9-ba61-23c647249616` |
| Write (H→D) | `49535343-8841-43f4-a8d4-ecbe34729bb3` |

The frame byte stream is carried unchanged. A frame may be split across several writes or
notifications. One reference writes 20-byte chunks with response and does not request a
larger MTU. Another packetises at up to 244 bytes per write and per notification.

Confidence: **likely** for serial and the BLE UUIDs. **uncertain** for the BLE chunking
requirements.

## Session

1. Open the transport (see above).
2. **Identity:** `[ID, ED]`.
   - The command set can be chosen from the model the user selected: codes ≥ 0x21 use
     iX3M ID `11`, otherwise iDive ID `10`.
   - Alternatively, probe. Try one ID command and fall back to the other if the reply is
     missing or shorter than 5 bytes. One reference tries `11` first for Ratio/Seac
     products and `10` first for DiveSystem products.
3. Parse the identity data:

   | Offset | Type | Meaning |
   |--------|------|---------|
   | 0 | u16le | Model code |
   | 2 | u32le | Firmware version. Decimal digits `M mm mmm nnn`: major = fw ÷ 10,000,000, then minor = (fw ÷ 100,000) mod 100, patch = (fw ÷ 1000) mod 100, build = fw mod 1000 |
   | 6 | u32le | Serial number (decimal) |
   | 10.. | | iX3M set only: 16 more bytes, content unknown |

4. If the iX3M set is in use and the firmware major is ≥ 4, switch to the **APOS4** variant.
   Sample records become 64 bytes, and 3 are returned per request.
5. **Range:** `[RANGE, 8D]` returns the first and last dive numbers stored. If the device
   answers NAK with error 0x58, there are no dives.

There is no explicit shutdown on serial or Bluetooth.

Confidence: **likely**.

## Commands

| Purpose | iDive set | iX3M set | iX3M APOS4 | Request payload | Reply data length |
|---------|-----------|----------|------------|-----------------|-------------------|
| Identity | `10` | `11` | `11` | `cmd ED` | 10 / 26 / 26 |
| Dive number range | `98` | `78` | `78` | `cmd 8D` | 4 |
| Dive header | `A0` | `79` | `79` | `cmd <num:u16le>` | 50 / 54 / 54 |
| Samples | `A8` | `7A` | `7A` | `cmd <index:u16le>` (1-based) | 42 / 54 / 3 × 64 = 192 |
| Set clock | – | `13` | `13` | see below | 0 |

Range reply data: `first:u16le, last:u16le`. `first > last` is invalid.

### Reply payload

Every reply frame's payload is:

```
<echo of the command byte> <data> <status>
```

- `status = 06` (ACK): the data length must equal the value in the table.
- `status = 15` (NAK): the payload is exactly `<cmd> <error code> 15`.

| Error code | Meaning |
|------------|---------|
| 0x10 | Invalid command |
| 0x20 | Invalid length |
| 0x30 | Invalid data |
| 0x40 | Unsupported |
| 0x58 | Not available. For the range command this means no dives |
| 0x5F | Unreadable. For a dive header, skip that dive |
| 0x60 | Busy. Retry |

Retries: on a CRC error, timeout or busy error, wait 100 ms and resend. One reference allows
9 retries and does not retry other NAK error codes. Another resends up to 8 times on CRC
errors or timeouts, and retries any NAK up to 10 times with a 50 ms delay.

### Set clock (iX3M set only)

```
13 <t:u32le> <home tz index:u8> <travel tz index:u8>
```

`t` is UTC seconds since 2008-01-01 00:00:00. The time zone index is a position in the table
under [Date and time](#date-and-time). A travel index of `FF` means "unchanged". If the
device answers NAK 0x20 or 0x30, resend without the travel byte.

### Firmware update

The iX3M set also has a bootloader entry command (`0A C9 4B`). It is out of scope here.

Confidence: **likely**.

## Framing and integrity

The same frame layout is used in both directions:

```
55 <len:1> <payload:len> <crc:u16be>
```

- `len` is 1..255. A received `len` below 2 is invalid.
- CRC-16/CCITT-FALSE: width 16, poly 0x1021, init 0xFFFF, refin false, refout false,
  xorout 0x0000. It covers the start byte `55`, `len` and the payload, and is transmitted
  **big-endian**. See
  [../common/checksums.md#crc-16-ibm-3740](../common/checksums.md#crc-16-ibm-3740).
- A receiver should skip bytes until it sees `55`.
- There is no escaping and no encryption.

Worked examples:

```
H→D  55 02 10 ED 0C 2E            iDive identity
H→D  55 02 11 ED 3F 1F            iX3M identity
H→D  55 02 98 8D F2 B9            iDive range
H→D  55 02 78 8D E2 0B            iX3M range
H→D  55 03 79 01 00 C9 1D         iX3M header, dive 1
H→D  55 03 7A 01 00 90 4D         iX3M samples from index 1
H→D  55 03 A0 01 00 32 18         iDive header, dive 1
D→H  55 06 78 01 00 0C 00 06 E4 E9   range reply: dives 1..12, ACK
D→H  55 03 78 58 15 08 9E         range reply: NAK, error 0x58 (no dives)
```

Confidence: **likely** (the examples were computed independently and match the second
reference).

## Memory layout

Not applicable. Dives are addressed by number in `[first, last]`.

## Download procedure

1. Identity and range (see [Session](#session)).
2. For `num` from `last` down to `first` (newest first):
   - request the header. Skip the dive on NAK 0x5F;
   - incremental download: the fingerprint is the 4 bytes at header data offset 7 (the
     start timestamp). Stop at the first match. Alternatively, compare the start time with
     the newest stored dive;
   - `n` = sample count (header offset 1);
   - for `j = 0, k, 2k, …` while `j < n`, request samples with index `j + 1`. `k` is the
     number of records per reply (1, or 3 for APOS4). When `j + k > n`, the reply's extra
     records are garbage. Keep only `n − j` records;
   - the dive is the header data followed by the sample records, without echo and status
     bytes.

One reference collects the new dive numbers first, then downloads them oldest first and
re-reads each header. Both orders work.

Confidence: **likely**.

## Dive data format

All offsets are relative to the reply data with the echo byte removed. This holds for the
header and for each sample record.

### Header (50 bytes iDive, 54 bytes iX3M)

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 1 | u16le | count | Number of sample records |
| 7 | u32le | s | Start time, seconds since 2008-01-01 00:00:00 (fingerprint) |
| 11 | u16le | iDive: mbar. iX3M: 0.1 mbar | Atmospheric pressure (iDive ÷1000 → bar, iX3M ÷10000 → bar) |
| 13 | i32le | s | Surface interval before the dive. Negative values mean 0 (one reference) |
| 34 | u8 | – | Water type: 0 = salt, other = fresh |
| 42 (0x2A) | u32le | – | Firmware version when recorded (iX3M) |
| 46 (0x2E) | u32le | – | Firmware version when recorded (iDive) |
| 48 | u8 | – | Time zone: even value 0..82. Value ÷ 2 is the index into the zone table. Used when the recorded firmware is APOS4 |

Dive time, maximum depth, gas mixes and tanks are not stored in the header. Derive them from
the samples (dive time = last sample time, max depth = deepest sample).

### Samples

Record size: 42 bytes (iDive), 54 bytes (iX3M) or 64 bytes (iX3M APOS4).

Which record size to parse with: use 64 if the dive was recorded with APOS4 firmware. Also
use 64 if `data length == header size + n × 64`, which means the dive was recorded with older
firmware but downloaded with APOS4. Otherwise use 54 for iX3M and 42 for iDive.

| Offset | Type | Unit | Meaning |
|--------|------|------|---------|
| 2 | u32le | s | Time since dive start. Must strictly increase |
| 6 | u16le | 1/10 m | Depth |
| 8 | i16le | 1/10 °C | Temperature |
| 10 | u8 | % | O2 of the active gas |
| 11 | u8 | % | He of the active gas |
| 14 | u8 | – | Decompression model code (see below) |
| 15 | u8 | % | Gradient factor high |
| 16 | u8 | % | Gradient factor low |
| 18 | u8 | – | Dive mode: 0 open circuit, 1 SCR, 2 CCR, 3 gauge, 4 freedive |
| 19 | u16le | mbar | Setpoint (SCR and CCR modes) |
| 21 | u16le | 1/10 m | Deco stop depth. 0 = no-deco |
| 23 | u16le | s | 42- and 54-byte records: no-deco time when the stop depth is 0, otherwise time to surface. One reference reads it as deco time with 0xFFFF = none. 64-byte records: deco stop time |
| 25 | u16le | s | 64-byte records: time to surface |
| 29 | u16le | % | CNS (÷100 → fraction) |
| 40 | i32le | mm | Altitude (GPS info record only) |
| 44 | i32le | 1e-7 ° | Longitude (GPS info record only) |
| 47 | u8 | – | 64-byte records: tank. Low nibble = transmitter ID. Bit 5 = 300-bar transmitter (double the pressure). Bit 6 = connection lost. Bit 7 = no transmitter |
| 48 | i32le | 1e-7 ° | Latitude (GPS info record only) |
| 49 | u8 | bar | 64-byte records: tank pressure, low 8 bits |
| 50 | u16le | ° | 64-byte records: compass bearing. 0xFFFF = none. Leading zeros mean "not yet valid" |
| 52 | u16le | – | iX3M records: record type / tank word. Value 1 = GPS info record (not a sample; its location applies to the next sample). For 64-byte samples: bit 15 = tank word valid, bits 0..4 = radio channel, bit 5 = pressure bit 8 |

Gas mixes: each distinct (O2, He) pair is a mix, up to 8. A change in the pair is a gas
switch. Tanks: each distinct transmitter ID with non-zero pressure, up to 10. The dive mode
is taken from the first sample.

Decompression model code (offset 14):

| Models | 0 | 1 | 2 | 3 |
|--------|---|---|---|---|
| iX3M 2 generation (codes 0x60..0xFFF) | Bühlmann | Bühlmann ZHL-16B | Bühlmann ZHL-16C | VPM |
| All others | Bühlmann | VPM | Dual (Bühlmann with gradient factors) | – |

### Date and time

The start time is a count of seconds since 2008-01-01 00:00:00 (Unix 1199145600).

- **Recorded with APOS4 firmware:** the count is UTC. Add the offset of the zone at header
  offset 48 to get local time. The dive's UTC offset is then known.
- **Older firmware:** one reference treats the count as UTC and converts it with the host's
  local time zone. Another adds the zone offset from offset 48 in every case. See open
  questions.

Zone table: index → UTC offset. 0 −12:00, 1 −11:00, 2 −10:00, 3 −9:30, 4 −9:00, 5 −8:00,
6 −7:00, 7 −6:00, 8 −5:00, 9 −4:30, 10 −4:00, 11 −3:30, 12 −3:00, 13 −2:00, 14 −1:00,
15 ±0:00, 16 +1:00, 17 +2:00, 18 +3:00, 19 +3:30, 20 +4:00, 21 +4:30, 22 +5:00, 23 +5:30,
24 +5:45, 25 +6:00, 26 +6:30, 27 +7:00, 28 +8:00, 29 +8:45, 30 +9:00, 31 +9:30, 32 +9:45,
33 +10:00, 34 +10:30, 35 +11:00, 36 +11:30, 37 +12:00, 38 +12:45, 39 +13:00, 40 +13:45,
41 +14:00.

Confidence: **likely** for time, depth, temperature, gas, deco stop depth and the header
fields above. **uncertain** for offset 23 semantics, the pre-APOS4 time zone handling,
and the GPS and tank fields.

## Model-specific variations

- **iDive set (codes < 0x21):** 10-byte identity, 50-byte header, 42-byte samples, no set
  clock, firmware at header offset 46.
- **iX3M set (codes ≥ 0x21):** 26-byte identity, 54-byte header, 54-byte samples, GPS info
  records, set clock.
- **APOS4 (iX3M set, firmware major ≥ 4):** 64-byte samples, 3 per request, tank pressure
  and bearing, separate deco time and TTS, time zone index.
- **iX3M 2 generation (0x60..0xFFF):** different decompression model codes.
- **Seac Jack/Guru (0x1000, 0x1002):** model code ≥ 0x21, so the iX3M set applies.

## Open questions

1. Offset 23 in 42- and 54-byte samples: NDL/TTS, or deco time with 0xFFFF = none?
2. Time zone handling for dives recorded before APOS4: is the stored count UTC or local time?
3. What are the 16 extra identity bytes of the iX3M set, and the rest of the header bytes?
4. BLE chunking: is a 20-byte write size required, or is a larger MTU (244) accepted by all
   models?
5. Bluetooth names: does `IX3M…` appear on Bluetooth classic devices (the original iX3M)?
   Does `DS…` appear on BLE?
6. Seac Jack/Guru: is the iX3M set right for them? One reference probes `11` first for
   these products.
7. How long can the busy (0x60) state last, and how many retries are needed?

## References

- libdivecomputer (LGPL-2.1), used as a factual reference. No code copied.
- Microchip Technology, BM70/BM71 Bluetooth Low Energy Module data sheet and "Transparent
  UART Service" (service `49535343-FE7D-4AE5-8FA9-9FAFD205E455`), https://www.microchip.com/
- Bluetooth SIG, Assigned Numbers (Serial Port Profile UUID 0x1101),
  https://www.bluetooth.com/specifications/assigned-numbers/
- Greg Cook, "Catalogue of parametrised CRC algorithms" (RevEng),
  https://reveng.sourceforge.io/crc-catalogue/
