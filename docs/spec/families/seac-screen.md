# Seac Screen family (Action, Screen, Tablet)

Seac's Screen-generation computers (Action, Screen and Tablet) share one binary protocol.
It runs over a USB serial cable and, on the Tablet, also over Bluetooth Low Energy. Each
frame is a start byte, a 16-bit length, a 16-bit command and a CRC-16. Responses add an
ACK/NAK status byte. Dives live in a profile ring buffer in flash. The host reads the range
of dive numbers, asks for each dive's start address, and then reads memory in 2 KiB blocks.
Each dive is two 64-byte header records followed by 64-byte sample records. Every record
carries its own CRC.

Seac's older Guru and Jack models use a different protocol (the DiveSystem/Ratio iDive
family) and are not covered here.

## Devices

| Vendor | Model | Model code | Transports |
|--------|-------|------------|------------|
| Seac | Action | 0x01 | USB serial |
| Seac | Screen | 0x02 | USB serial |
| Seac | Tablet | 0x10 | USB serial, BLE |

The model code is the `u32le` at offset 4 of the hardware-info response.

- BLE name: `Tablet` followed by zero or more decimal digits, with nothing else after the
  digits. The prefix match is case-insensitive.
- BLE service UUID: `84968ffe-d26d-478a-b953-5010bcf58bca`. Characteristic UUIDs are not
  publicly documented. Known hosts use the first writable and the first notifying
  characteristic of that service.
- USB: the USB-serial bridge chip and VID:PID are not known (see
  [Open questions](#open-questions)).

Confidence: **likely**.

## Transport parameters

| Parameter | USB serial | BLE |
|-----------|-----------|-----|
| Line | 115200 8N1, no flow control | n/a |
| Response timeout | 1000 ms | 6000 ms |
| Write chunking | none | at most 244 bytes per write |
| Read | byte stream | notifications concatenated into a byte stream |

Confidence: **likely**.

## Session

1. Wait 100 ms and flush input.
2. Send the single wake-up byte `61` (ASCII `a`). No response is expected, and any error is
   ignored.
3. Read **hardware info** (`18 33`, 256 bytes) and **software info** (`18 34`, 256 bytes).
4. Choose the command set and ring-buffer layout by model: Tablet (0x10) uses the `185x`
   commands; Action and Screen use the `184x` commands.

Device information:

| Source | Offset | Type | Meaning |
|--------|--------|------|---------|
| hardware info | 0x04 | u32le | model code |
| hardware info | 0x10 | u32le | serial number |
| software info | 0x14 | u32le | firmware version (Tablet) |
| software info | 0x1C | u32le | firmware version (Action, Screen) |

There is no logout command.

Confidence: **likely**.

## Commands

Command codes are 16-bit and sent big-endian. All multi-byte arguments and results in
commands are big-endian.

| Function | Action/Screen | Tablet | Request data | Response data |
|----------|---------------|--------|--------------|---------------|
| Hardware info | 1833 | 1833 | — | 256 bytes |
| Software info | 1834 | 1834 | — | 256 bytes |
| Dive number range | 1840 | 1850 | — | `u32be` first, `u32be` last dive number |
| Dive address | 1841 | 1851 | `u32be` dive number | `u32be` start address of that dive |
| Read memory | 1842 | 1852 | `u32be` address, `u32be` length (≤ 2048) | see below |

Read memory response length:

- Action/Screen: always 2048 bytes. Only the first `length` bytes are valid; the rest is
  zero padding.
- Tablet: exactly `length` bytes.

NAK error codes:

| Code | Meaning |
|------|---------|
| 02 | invalid command |
| 03 | invalid length |
| 04 | invalid data |
| 05 | battery low |
| 06 | busy |

Confidence: **likely**.

## Framing and integrity

Request:

```
55  len_hi len_lo  cmd_hi cmd_lo  data[n]  crc_hi crc_lo
len = n + 6   (length counts everything after the start byte)
```

Response:

```
55  len_hi len_lo  cmd_hi cmd_lo  payload[m]  status  crc_hi crc_lo
len = m + 7
status: 09 = ACK, 30 = NAK
```

- A NAK response carries exactly one payload byte, the error code, so its len is 8.
- An ACK response carries exactly the expected number of payload bytes.
- The receiver skips bytes until it sees `55`. Then it reads the length and the rest of the
  frame, and checks the CRC, the echoed command, the status byte and the expected length.
- On a CRC error, a framing error or a timeout, wait 100 ms, flush input and resend the
  request. Known hosts retry up to 4 times.

CRC: **CRC-16/IBM-3740** (also called CRC-16/CCITT-FALSE): width 16, poly 0x1021, init
0xFFFF, refin false, refout false, xorout 0x0000. It covers every byte from the `55` up to
the byte before the CRC, and is sent big-endian. Check value for `"123456789"` = `0x29B1`.
See [../common/checksums.md#crc-16-ibm-3740](../common/checksums.md#crc-16-ibm-3740).

Worked examples:

```
H→D  55 00 06 18 33 19 23                               hardware info
H→D  55 00 06 18 40 57 D7                               dive range (Screen)
D→H  55 00 0F 18 40 00 00 00 01 00 00 00 0C 09 C5 48    range 1..12, ACK
H→D  55 00 0E 18 42 00 01 00 00 00 00 00 80 19 56       read 128 bytes at 0x010000 (Screen)
```

Confidence: **likely**.

## Memory layout

Each dive starts at the address that the dive-address command returns. A dive occupies a
128-byte header followed by `nsamples × 64` bytes, and it may wrap around the end of the
profile ring buffer.

| Model | Profile ring buffer |
|-------|---------------------|
| Action, Screen | 0x010000 – 0x200000 (exclusive) |
| Tablet | 0x0A0000 – 0x400000 (exclusive) |

Dive addresses outside the ring buffer are a format error. Space after a dive's last sample,
before the next dive, is filled with `FF`.

Confidence: **likely**.

## Download procedure

```
(first, last) = range()
if fingerprint_number >= last: nothing new
elif fingerprint_number >= first: first = fingerprint_number + 1
for number = last down to first:
    addr[number] = dive_address(number)
end = none; remaining = ring size
for number = last down to first:
    if end is none:                       # newest valid dive
        hdr = read(addr[number], 128)
        if both header records valid:
            end = ring_add(addr[number], 128 + 64 × u32le(hdr[0x44]))
        else:
            end = addr[number]; skip this dive; continue
    length = ring_distance(addr[number], next newer start, or end)
    if length > remaining: stop          # older dives were overwritten
    remaining -= length
read the profile backward from `end` in 2048-byte blocks, splitting it into the dives above
for each dive: keep 128 + 64 × nsamples bytes; if the header is invalid, keep all `length` bytes
```

- The fingerprint is the 4-byte dive number at offset 0 of the dive (`u32le`). The host
  compares it numerically with the device's dive numbers.
- A newest dive with corrupt header records is dropped, and its start address is used as
  the end of the profile.
- A full memory dump is the whole profile ring buffer read in 2048-byte blocks.

Confidence: **likely**.

## Dive data format

The dive consists of 64-byte records. Each record has this structure:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u32le | dive number (same for every record of the dive) |
| 0x3D | u8 | record type: `CF` = header part 1, `C0` = header part 2, `AA` = sample |
| 0x3E | u16be | CRC-16/IBM-3740 over bytes 0x00..0x3D |

A record is valid when its CRC is correct, its type is right and its dive number matches.
Because the CRC is non-reflected and stored big-endian, running the CRC over all 64 bytes
gives 0 for a valid record.

### Header

The header is the first two records (128 bytes). Offsets below are from the start of the
dive, so 0x40..0x7F lie in the second record.

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x00 | u32le | | dive number |
| 0x0A | u8 | | time-zone index (see [Date and time](#date-and-time)) |
| 0x0B | u8 | | hour (UTC) |
| 0x0C | u8 | | minute |
| 0x0D | u8 | | second |
| 0x0E | u8 | | day |
| 0x0F | u8 | | month |
| 0x10 | u8 | | year − 2000 |
| 0x26 | u8 | | dive mode: 1 air/nitrox (OC), 2 gauge, 3 freedive |
| 0x44 | u32le | | number of sample records |
| 0x62 | u16le | cm | maximum depth |
| 0x64 | u32le | s | dive time |
| 0x68 | u16le | 0.01 °C | maximum temperature |
| 0x6A | u16le | 0.01 °C | minimum temperature |
| 0x72 | u16le | cm | average depth |

### Samples

Each sample is one 64-byte record of type `AA`:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x04 | u32le | s | time since dive start (must not decrease) |
| 0x08 | u16le | cm | depth |
| 0x0A | u16le | 0.01 °C | temperature |
| 0x0C | u8 | % | O2 of the breathing gas; a change means a gas switch |
| 0x0E | u16le | cm | decompression stop depth; 0 = no decompression obligation |
| 0x10 | u16le | | decompression stop time |
| 0x12 | u16le | | no-decompression time |
| 0x14 | u16le | bar | tank pressure (Tablet only; 0 = none) |
| 0x16 | u16le | % | CNS |
| 0x3B | u8 | % | gradient factor high |
| 0x3C | u8 | % | gradient factor low |

Only nitrox gases are recorded (no helium). Known hosts allow at most 2 distinct gases. The
gradient factors are taken from the first sample where they are non-zero.

### Date and time

The header stores **UTC** date and time plus an index into a fixed list of time-zone
offsets. Local time = UTC + offset. Index → offset:

```
 0 −12:00   1 −11:00   2 −10:00   3 −09:30   4 −09:00   5 −08:00   6 −07:00
 7 −06:00   8 −05:00   9 −04:30  10 −04:00  11 −03:30  12 −03:00  13 −02:00
14 −01:00  15 ±00:00  16 +01:00  17 +02:00  18 +03:00  19 +03:30  20 +04:00
21 +04:30  22 +05:00  23 +05:30  24 +05:45  25 +06:00  26 +06:30  27 +07:00
28 +08:00  29 +08:45  30 +09:00  31 +09:30  32 +09:45  33 +10:00  34 +10:30
35 +11:00  36 +11:30  37 +12:00  38 +12:45  39 +13:00  40 +13:45  41 +14:00
```

An index above 41 is invalid.

Confidence: **likely** for the record layout and header. **Uncertain** for the units of the
decompression time fields (seconds assumed).

## Model-specific variations

- Tablet: separate command codes (`185x`), a larger ring buffer, BLE support, exact-length
  read responses and tank pressure in samples.
- Action and Screen: `184x` commands, read responses padded to 2048 bytes, and no tank
  pressure.

## Open questions

1. USB-serial chip and VID:PID of the Seac cable.
2. BLE characteristic UUIDs and write type for the Tablet.
3. Units of the sample decompression times (seconds assumed).
4. Whether CCR or trimix variants exist that use other header modes.
5. The rest of the 256-byte hardware and software info blocks.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- Subsurface BLE transport (GPL-2.0), service UUID table,
  <https://github.com/subsurface/subsurface>, used as a factual reference; no code copied.
- Seac product information, <https://www.seacsub.com/>.
