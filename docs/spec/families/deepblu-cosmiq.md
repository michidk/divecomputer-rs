# Deepblu COSMIQ+

The Deepblu COSMIQ+ is a recreational and freediving computer with Bluetooth Low Energy. Its
protocol is ASCII and line-based. Every packet is a start character, a hex-encoded binary
body (command, checksum, length, data) and a newline. Each dive has a 36-byte header and
4-byte samples (temperature, absolute pressure). Both are streamed as a series of data
packets after an initial "size" response.

## Devices

| Vendor | Model | Model code | Transports | BLE name |
|--------|-------|------------|------------|----------|
| Deepblu | COSMIQ+ | 0 | BLE | `COSMIQ` (exact, case-insensitive) |

The device reports no numeric model code.

Confidence: **likely**.

## Transport parameters

- BLE. The data path runs through a Nordic-style UART service. The exact UUIDs have not been
  confirmed (see [Open questions](#open-questions)).
- Notifications are at most 20 bytes. One protocol line may be split over several
  notifications. The receiver concatenates notifications until one ends in `\n` (`0A`).
- Response timeout: 1000 ms per notification. Flush input after connecting.
- Each line carries at most 20 data bytes. In practice responses hold only a few data bytes
  per line.

Confidence: **likely**.

## Session

No handshake. After connecting, the host can send commands straight away. Known hosts read
the firmware version, then the Bluetooth MAC address, then the dive count.

## Commands

Commands are written below as (command byte, data bytes). The response line echoes the
command byte.

| Function | Cmd | Request data | Response data |
|----------|-----|--------------|---------------|
| Set date/time | 20 | 6 × BCD: YY (year−2000), MM, DD, hh, mm, ss | 1 byte (status) |
| Dive count | 40 | `00` | 1 byte: number of dives (0-255) |
| Request dive header | 41 | dive number (1 = newest) | 1 byte: header length (always 36) |
| Header data | 42 | (sent by the device only) | header bytes, a few per line |
| Request dive profile | 43 | dive number | `u16be` profile length in bytes |
| Profile data | 44 | (sent by the device only) | profile bytes, a few per line |
| Firmware version | 58 | `00` | 1 byte. Bits 0-5 = version |
| MAC address | 5A | `00` | 6 bytes. Known hosts use the first 4 bytes as `u32le` serial number |

After `41`, the device sends as many `42` lines as needed to deliver the 36 header bytes.
After `43`, it sends `44` lines until the announced profile length has arrived. The host
does not send anything between those data lines.

Confidence: **likely**.

## Framing and integrity

Host → device:

```
'#'  HEX(cmd)  HEX(csum)  HEX(len)  HEX(data[0]) … HEX(data[n-1])  '\n'
```

Device → host: the same, with `$` instead of `#`.

- `HEX(x)` is two **upper-case** ASCII hex digits.
- `len` is the number of **hex characters** of data, which is 2 × n. It is not the byte
  count.
- `csum` is chosen so that the 8-bit sum of `cmd + csum + len + data[0..n-1]` is 0 mod 256.
  In other words, csum is the two's complement (negation) of the [sum8](../common/checksums.md#sum8)
  of the other bytes.
- A received line is valid when: it starts with `$`, ends with `\n`, has an even total
  length ≥ 8, decodes as hex, echoes the command, has `len` equal to the number of data hex
  characters (total length − 8), and its decoded bytes sum to 0.

Worked examples (ASCII):

```
H→D  #58A60200\n     firmware version   (0x58 + 0x02 + 0x00 = 0x5A; −0x5A = 0xA6)
H→D  #40BE0200\n     dive count
H→D  #41BC0201\n     header of dive 1   (0x41 + 0x02 + 0x01 = 0x44; −0x44 = 0xBC)
```

Confidence: **likely**.

## Memory layout

No raw memory access. Dives are addressed by number, 1 = newest.

## Download procedure

```
fw = cmd(58, [00]); mac = cmd(5A, [00]); n = cmd(40, [00])
headers = []
for i in 1..n:
    require cmd(41, [i]) == [36]
    h = receive 36 bytes from "42" lines
    if h[6..11] == known_fingerprint: break
    headers.append(h)
for i, h in enumerate(headers, start=1):
    len = u16be(cmd(43, [i]))
    profile = receive len bytes from "44" lines
    dive = h + profile
```

Fingerprint: 6 bytes at header offset 6 (date and time fields).

Confidence: **likely**.

## Dive data format

A dive is the 36-byte header followed by the profile.

Depth is stored as absolute pressure in mbar:
`depth_m = (p − surface) × 100 / (density × 9.80665)`. The device does not store density;
use 1025 kg/m³ unless the user sets another value.

### Header

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x02 | u8 | | dive mode: 2 scuba (open circuit), 3 gauge, 4 freedive |
| 0x03 | u8 | % | O2 of the single gas (scuba mode only) |
| 0x04 | u16le | mbar | bits 0-12: surface pressure. Bits 13-15 unknown, mask them off |
| 0x06 | u16le | | year (full, e.g. 2019) |
| 0x08 | u8 | | day |
| 0x09 | u8 | | month |
| 0x0A | u8 | | minute |
| 0x0B | u8 | | hour |
| 0x0C | u16le | min or s | dive time: minutes in scuba and gauge mode, seconds in freedive mode |
| 0x16 | u16le | mbar | maximum absolute pressure → max depth |
| 0x1A | u8 | s | sample interval |
| other | | | not decoded |

The date fields are binary, not BCD, even though the set-time command uses BCD. Seconds are
not stored.

### Samples

4 bytes each, starting at offset 36:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| +0 | u16le | 0.1 °C | temperature |
| +2 | u16le | mbar | absolute pressure → depth |

Samples of all `FF` are empty; skip them without advancing time. Each other sample advances
time by the header interval. The first sample is at t = 1 interval.

### Date and time

Local time, no time zone stored. Seconds are taken as 0.

Confidence: **likely** (one implementation).

## Model-specific variations

None known.

## Open questions

1. GATT UUIDs. The device is assumed to use the Nordic UART service
   (`6e400001-b5a3-f393-e0a9-e50e24dcca9e`, TX/RX `6e400002…`/`6e400003…`), but no public
   source confirms this.
2. The meaning of header bytes 0-1, 14-21 and 27-35 (likely average depth, temperatures and
   settings).
3. The meaning of the set-time status byte.
4. Whether the dive count byte limits the log to 255 dives.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
