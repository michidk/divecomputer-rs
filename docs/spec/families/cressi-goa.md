# Cressi Goa family (Cartesio, Goa, Leonardo 2.0, Donatello, Michelangelo, Raffaello, Neon, Nepto)

Newer Cressi computers, used over a USB-serial cable and, on recent models, over Bluetooth
Low Energy. There is no raw memory access. The host asks for a logbook index and then for
each dive by number.

- **Serial:** commands are short binary frames with a CRC-16. Bulk replies arrive as
  512-byte XMODEM-like blocks, each acknowledged with ACK.
- **BLE:** the host writes only the command byte and its arguments. Bulk replies arrive as
  raw 512-byte blocks followed by a fixed text trailer. Identity data is read from GATT
  characteristics.

The dive record format depends on a *data format version* (0..5) derived from the firmware.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Cressi | Cartesio | 1 | serial, BLE | |
| Cressi | Goa | 2 | serial, BLE | |
| Cressi | Leonardo 2.0 | 3 | serial, BLE | |
| Cressi | Donatello | 4 | serial, BLE | |
| Cressi | Michelangelo | 5 | serial, BLE | |
| Cressi | Raffaello | 6 | serial, BLE | |
| Cressi | Neon | 9 | serial, BLE | |
| Cressi | Nepto | 10 | serial, BLE | |

The model code is byte 4 of the identity block (see [Session](#session)). Which models
actually have BLE hardware is not established. Hosts accept BLE for all of them.

BLE advertised name: the model code in **hexadecimal**, an underscore, then hex digits.
For example, `2_1A2B3C` would be a Goa. Another reference matches names against the regular
expression `[0-9]{1,2}_[0-9]+` (decimal digits only). The two agree for codes 1..9 with
all-decimal suffixes but differ for Nepto (`a_…`) and for hex suffixes. See open questions.

Serial: USB-serial interface cable. Its VID:PID is not established by public sources.

Confidence: **likely**.

## Transport parameters

### Serial

- 115200 baud, 8 data bits, no parity, 1 stop bit, no flow control.
- Read timeout: 3000 ms in one reference, 600 ms in another.
- Line control varies between references:

| Variant | Sequence |
|---------|----------|
| A | De-assert RTS. De-assert DTR. Wait 100 ms. Discard pending input and output |
| B | Assert RTS. Assert DTR. Wait 30 ms. De-assert DTR. Wait 3 ms. Assert DTR. Wait 3 ms. De-assert DTR |

- Wait about 100 ms before sending each command. Without this delay transfers often fail.

### BLE

| Role | UUID |
|------|------|
| Service | `6e400001-b5a3-f393-e0a9-e50e24dc10b8` |
| Notify (D→H) | `6e400002-b5a3-f393-e0a9-e50e24dc10b8` |
| Write (H→D) | `6e400001-b5a3-f393-e0a9-e50e24dc10b8` (same value as the service UUID) |
| Read: serial and model, 5 bytes | `6e400003-b5a3-f393-e0a9-e50e24dc10b8` |
| Read: firmware, 2 bytes | `6e400004-b5a3-f393-e0a9-e50e24dc10b8` |
| Read: 2 more bytes | `6e400005-b5a3-f393-e0a9-e50e24dc10b8` |

The UUIDs use the Nordic UART Service base, but the characteristic roles differ from
standard NUS (where …0002 is write and …0003 is notify). The notify and write roles above
come from a single reference. Discover the characteristics and check their properties
before relying on them.

- Writes use write-with-response in 20-byte chunks (one reference). No MTU change is
  required.
- Wait about 2000 ms before each command (one reference uses 1500 ms after the write).
- Read timeout: about 5 s. Use about 10 s of inactivity to detect the end of a bulk transfer.

Confidence: **likely** for serial. **uncertain** for the BLE characteristic roles and the
name pattern.

## Session

### Identity

**Serial:** send command 0x00 (see [Commands](#commands)). The reply frame carries the
identity block as its payload. The block is 9 or 11 bytes.

**BLE:** no command is sent. Read the three characteristics above and concatenate their
values in the order …0003, …0004, …0005 to form the identity block. One reference reads only
…0003 and …0004.

Identity block:

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u32le | Serial number |
| 4 | u8 | Model code |
| 5 | u16le | Firmware version (decimal, e.g. 166) |
| 7 | 2 bytes | Unknown |
| 9 | u16le | Data format version (present only when the block is 11 bytes) |

When the block has no explicit data format version, derive it from the firmware:

| Firmware | Data format version |
|----------|---------------------|
| 100..110 | 3 |
| 161..165 | 0 |
| 166..169 | 1 |
| 170..179 | 2 |
| 200..299 | 4 |
| ≥ 300 | 5 |
| anything else | unsupported |

### Shutdown

Serial only: send command 0x1D (leave PC-link mode). No shutdown is sent over BLE.

Confidence: **likely**.

## Commands

### Serial frame

```
H→D  AA AA AA <len:1> <cmd:1> <args:len> <crc:u16le> 55
```

`len` is the number of argument bytes (0..12). See [Framing](#framing-and-integrity) for
the CRC.

| Cmd | Purpose | Arguments | Reply | Notes |
|-----|---------|-----------|-------|-------|
| 0x00 | Identity | none | frame with a 9- or 11-byte payload | |
| 0x13 | Set clock | year u16le, month, day, hour, minute, second (7 bytes) | empty frame | Serial only |
| 0x1D | Exit PC link | none | empty frame | Serial only |
| 0x21 | Logbook index, format versions 0..3 | none | empty frame, then bulk data | 23-byte entries |
| 0x22 | Dive | dive number u16le | empty frame, then bulk data | |
| 0x23 | Logbook index, format versions ≥ 4 | none | empty frame, then bulk data | 15-byte entries |

Reply frames use the same layout as commands. An "empty frame" has `len = 0`, so it is
8 bytes. One reference observed command byte 0x21 in the reply to both 0x21 and 0x22 and
used it as an "accepted" check. Another reference does not check that byte.

### BLE

Only the command byte and its arguments are written. There is no `AA`, length, CRC or `55`.

| Bytes written | Purpose | Notes |
|---------------|---------|-------|
| `02 00` | Logbook index | Used for all format versions |
| `03 <num:u16be>` | Dive | Dive number **big-endian** |

There is no reply frame. Bulk data follows directly. Set clock and exit are not available.

Confidence: **likely**.

## Framing and integrity

### Command and reply frames (serial)

CRC-16/XMODEM: width 16, poly 0x1021, init 0x0000, refin false, refout false,
xorout 0x0000. It covers `len`, `cmd` and the arguments, and is transmitted
**little-endian**. See
[../common/checksums.md#crc-16-xmodem](../common/checksums.md#crc-16-xmodem) (check value
0x31C3).

Worked examples:

```
Identity              AA AA AA 00 00 00 00 55
Logbook (v0..3)       AA AA AA 00 21 43 34 55
Logbook (v4+)         AA AA AA 00 23 01 14 55
Dive number 1         AA AA AA 02 22 01 00 FF 36 55
Exit PC link          AA AA AA 00 1D 9C C3 55
Set clock 2024-03-15 10:20:30
                      AA AA AA 07 13 E8 07 03 0F 0A 14 1E D0 C6 55
```

A receiver checks the three `AA` bytes, a length of 12 or less, the CRC and the trailing `55`.

### Bulk data, serial

After the reply frame, the device sends one or more 517-byte packets:

```
D→H  <3 bytes header> <512 data bytes> <crc:u16le>
H→D  06                         (ACK)
...
D→H  04                         (EOT, after the last packet)
H→D  06
```

- CRC: CRC-16/XMODEM over the 512 data bytes, little-endian.
- The contents of the 3-byte packet header are not documented. They are probably an
  XMODEM-style start byte and block number.
- On a bad packet, one reference sends NAK `15` (up to 5 times) and waits for a resend.
  Another treats it as an error. One reference waits 50 ms before each ACK or NAK.

### Bulk data, BLE

Notifications carry the raw 512-byte data blocks with no header and no CRC. After the last
block the device sends a 16-byte trailer:

```
45 4F 54 20 78 6D 6F 64 65 6D 00 00 00 00 00 00        "EOT xmodem" + 6 × 00
```

The host sends no ACK.

### Payload length

The first two data bytes of the first block are a u16le `L`: the number of payload bytes that
follow. The transfer holds `L + 2` meaningful bytes, padded up to a multiple of 512. A host
should strip the 2-byte length and keep exactly `L` bytes. (One reference instead computes
the block count as `ceil(L / 512)`. See open questions.)

Confidence: **likely** for the CRC and framing. **uncertain** for the packet header bytes.

## Memory layout

Not applicable: the protocol has no address-based access.

## Download procedure

1. Open the transport. On serial, apply the line control and purge.
2. Read the identity and work out the data format version.
3. Fetch the logbook index: serial command 0x21 (versions 0..3) or 0x23 (versions ≥ 4), or
   BLE `02 00`. Strip the 2-byte length.
4. Split the payload into entries of 23 bytes (versions 0..3) or 15 bytes (versions ≥ 4).

   | Entry offset | 23-byte entry | 15-byte entry |
   |--------------|---------------|---------------|
   | 0 | dive number u16le (0 = unused) | dive number u16le |
   | 2 | dive mode u8 | dive mode u8 |
   | fingerprint (6 bytes) | offset 17 | offset 3 |
   | 15 | number of sample words u16le (one reference) | – |

   In the 23-byte entry the fingerprint is the dive date and time: year u16le, month, day,
   hour, minute.

5. Walk entries from the **end** of the index backwards (newest first). Stop at dive
   number 0 or when the 6 fingerprint bytes equal the stored fingerprint.
6. For each new dive, request it by number (serial 0x22, BLE `03 hi lo`) and strip the
   2-byte length. Verify that the record starts with the same dive number. Verify that the
   6 bytes at record offset 12 (versions 0..3) or 4 (versions ≥ 4) equal the index
   fingerprint.
7. On serial, send 0x1D at the end.

Parsing a dive needs the identity block (for the format version) and the index entry (for
the dive mode). Store both with the dive record. The record does not contain them.

An alternative incremental strategy compares the index date with the newest stored dive
time instead of a fingerprint.

Confidence: **likely**.

## Dive data format

Offsets are relative to the dive record after the 2-byte length is removed, so offset 0 is
the dive number. The dive mode comes from the index entry:

| Mode | Meaning |
|------|---------|
| 0 | Scuba (air) |
| 1 | Nitrox |
| 2 | Freedive |
| 3 | Gauge |
| 5 | Advanced freedive (format version 4 only) |

### Header

The header length depends on version and mode. Its **last two bytes** are a CRC-16/CCITT-FALSE
(poly 0x1021, init 0xFFFF, no reflection, xorout 0) over all preceding header bytes, stored
little-endian. See [../common/checksums.md#crc-16-ibm-3740](../common/checksums.md#crc-16-ibm-3740).

**Scuba and nitrox:**

| Field | v0 | v1, v2 | v3 | v4 | v5 | Unit |
|-------|----|--------|----|----|----|------|
| Header length | 90 | 92 | 92 | 82 | 83 | bytes |
| Sample count u16le | 10 | 10 | 10 | 10 | 2 | words |
| Sample-rate index u8 | – | – | – | 2 | – | see below |
| Date/time | 12 | 12 | 12 | 4 | 4 | see below |
| Dive time u16le | 20 | 20 | 20 | 11 | 11 | s |
| Gas 1, 2, 3 entries | 26, 28, – | 26, 28, – | 26, 28, 87 | 17, 19, 21 | 17, 19, 21 | 2 bytes each |
| Atmospheric pressure u16le | 30 | 30 | 30 | 23 | 23 | mbar (÷1000 → bar) |
| Max depth u16le | 73 | 73 | 73 | 66 | 66 | 1/10 m |
| Average depth u16le | 75 | 75 | 75 | 68 | 68 | 1/10 m |
| Min temperature u16le | 77 | 77 | 77 | 70 | 70 | 1/10 °C |

Gas entry: byte +1 is the O2 percentage. Byte +0 is unknown. A gas with O2 = 0 is unused,
and counting stops at the first unused gas. Helium is 0. One reference also reads a deco
flag at offset 80 and a surface interval at offset 35 (versions 0..3).

**Freedive:**

| Field | v0 | v1..v3 | v4, v5 |
|-------|----|--------|--------|
| Header length | 34 | 38 | 27 |
| Sample count u16le | 10 | 10 | 2 |
| Sample-rate index u8 | – | – | 10 |
| Date/time | 12 | 12 | 4 |
| Dive time u16le (s) | 20 | 20 | 11 |
| Max depth u16le (1/10 m) | 23 | 23 | 15 |
| Min temperature u16le (1/10 °C) | 25 | 25 | 17 |

One reference reads the freedive dive time at 29 and a surface interval (s) at 27 for
versions 0..3. See open questions.

**Gauge:**

| Field | v0 | v1..v3 | v4, v5 |
|-------|----|--------|--------|
| Header length | 38 | 40 | 28 |
| Sample count u16le | 10 | 10 | 2 |
| Sample-rate index u8 | – | – | 10 |
| Date/time | 12 | 12 | 4 |
| Dive time u16le (s) | 20 | 20 | 11 |
| Atmospheric u16le (mbar) | 22 | 22 | 13 |
| Max depth u16le (1/10 m) | 24 | 24 | 15 |
| Average depth u16le (1/10 m) | 26 | 26 | 17 |
| Min temperature u16le (1/10 °C) | 28 | 28 | 19 |

**Advanced freedive** (v4 only): header length 28, sample count at 2, sample-rate index
at 10, date/time at 4, dive time at 22, max depth at 16, min temperature at 18. A 13-byte
block of dip statistics follows the samples.

Sample-rate index: 1 → 0.5 s, 2 → 1 s, 3 → 2 s, 4 → 5 s. Other values are invalid. If the
layout has no index, the interval is 2 s for freedive and 5 s for every other mode.

### Samples

The samples start right after the header: the given number of u16le words.
`type = word & 3`, `value = word >> 2` (14 bits).

| Type | Meaning | Decoding |
|------|---------|----------|
| 0 | Scuba depth sample | depth = `value & 0x7FF` (1/10 m). Gas index = `(value >> 11) & 1` for v0..v2, `(value >> 11) & 3` for v3+. Time advances by the interval |
| 1 | Freedive depth sample | depth = `value & 0x7FF` (v0..v3) or `value & 0xFFF` (v4+), 1/10 m. Time advances by the interval |
| 2 | Surface period | `value` = seconds at the surface. Depth 0. Time advances by `value` s |
| 3 | Temperature | `value` = 1/10 °C. Applies to the next emitted sample |

Gas switches are only meaningful in scuba and nitrox modes. The gas index refers to the
header gas list.

Variants reported by another reference: it ignores type 2 words, and it treats temperature
as signed (bit 15 set means negative).

### Date and time

`year u16le, month u8 (1..12), day u8, hour u8, minute u8`. Local time, no seconds and no
time zone. These 6 bytes are the fingerprint, in both the index entry and the record
(the fingerprint offsets above point at this field).

Confidence: **likely** for scuba layouts and samples. **uncertain** for freedive and gauge
offsets in versions 0..3, the type 2 semantics, and the sign of temperature.

## Model-specific variations

- The data format version, not the model, selects the layouts. Firmware 100..110 maps to
  version 3. That range presumably belongs to a newer model line with its own numbering.
- One reference says firmware ≥ 166 moves the scuba sample start by 2 bytes. This matches
  header length 90 → 92 between versions 0 and 1.
- Advanced freedive mode (5) exists only in version 4.

## Open questions

1. BLE characteristic roles: is …0001 really the write characteristic? Which of
   …0002 and …0003 notifies?
2. BLE name pattern: hex model code (`a_…` for Nepto) or decimal only?
3. What do the 3 header bytes of a serial bulk packet contain, and does the device resend
   after a NAK?
4. Does the payload length `L` include the 2 length bytes? (This decides
   `ceil((L+2)/512)` versus `ceil(L/512)` blocks.)
5. What do identity bytes 7..8 and characteristic …0005 mean? Does …0005 carry the data
   format version on newer firmware?
6. What does the command byte in the serial reply frame mean (always 0x21)?
7. Freedive header offsets in v0..v3 (dive time 20 versus 29) and the surface interval
   fields.
8. Is temperature signed, and what does gas entry byte +0 hold?
9. Does the logbook index command over BLE (`02 00`) return 15-byte entries for
   version ≥ 4 devices?

## References

- libdivecomputer (LGPL-2.1), used as a factual reference. No code copied.
- Nordic Semiconductor, Nordic UART Service (base UUID `6e400001-b5a3-f393-e0a9-e50e24dc10b8`),
  https://docs.nordicsemi.com/bundle/ncs-latest/page/nrf/libraries/bluetooth/services/nus.html
- Greg Cook, "Catalogue of parametrised CRC algorithms" (RevEng),
  https://reveng.sourceforge.io/crc-catalogue/
