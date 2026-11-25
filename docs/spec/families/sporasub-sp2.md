# Sporasub SP2

The Sporasub SP2 is a spearfishing and freediving watch with heart-rate logging. It connects
through a USB serial cable at 460800 baud. The protocol uses framed binary packets with a
two-byte header, a two-byte trailer and a 16-bit additive checksum. The host reads the whole
64 KiB memory in 128-byte blocks, then finds the dives in a linear log area starting at
0x0060.

## Devices

| Vendor | Model | Model code | Transports |
|--------|-------|------------|------------|
| Sporasub | SP2 | 0 | USB serial |

The USB-serial bridge chip and VID:PID are not known (see [Open questions](#open-questions)).

Confidence: **likely**.

## Transport parameters

- 460800 baud, 8 data bits, no parity, 1 stop bit, no flow control.
- Clear RTS, set DTR, wait 100 ms, then flush input and output.
- Response timeout: 1000 ms.
- On a framing, checksum or timeout error, wait 100 ms, flush input and resend the request.
  Known hosts retry up to 4 times (not for the initial version request).

Confidence: **likely**.

## Session

1. Configure the line as above.
2. Send **version** (`10`). The response has 23 payload bytes. Bytes 1..2 as `u16be` are the
   serial number; the other bytes are not decoded.
3. Read memory or set the clock. There is no logout command.

## Commands

The response command byte is the request command + 1.

| Function | Cmd | Request data | Resp cmd | Response data |
|----------|-----|--------------|----------|---------------|
| Version | 10 | — | 11 | 23 bytes |
| Read memory | 12 | `u16le` address, `u8` length (≤ 128) | 13 | `length` bytes |
| Set clock | 39 | year−2000, month, day, hour, minute, second | 3A | 1 byte: 0 = OK |

Confidence: **likely**.

## Framing and integrity

```
A0 A2  len_hi len_lo  cmd  data[n]  sum_hi sum_lo  B0 B3
len = n + 1                         (command byte + data)
sum = (cmd + data[0] + … + data[n-1]) mod 65536, sent big-endian
```

The checksum is [sum16](../common/checksums.md#sum16) over the command byte and the data.
Responses have the same layout. A receiver reads exactly `expected payload + 9` bytes and
checks the header, the trailer, the length, the command and the checksum.

Worked examples:

```
H→D  A0 A2 00 01 10 00 10 B0 B3                 version
H→D  A0 A2 00 04 12 60 00 80 00 F2 B0 B3        read 128 bytes at 0x0060
```

Confidence: **likely**.

## Memory layout

64 KiB (0x0000–0xFFFF), read in 128-byte blocks.

| Address | Type | Meaning |
|---------|------|---------|
| 0x0002 | u16le | number of dives stored |
| 0x0004 | u16le | end-of-log pointer: first free byte after the newest dive (0x0060..0x10000) |
| 0x0060 – 0xFFFF | | dive log area |

Dives are stored oldest first, starting at 0x0060. Each dive is a 32-byte header followed
by `nsamples × 4` bytes. The next dive starts at the following 32-byte boundary.

Confidence: **likely**.

## Download procedure

```
mem = read(0x0000, 0x10000) in 128-byte blocks
n = u16le(mem[2]); eop = u16le(mem[4]); require 0x60 <= eop <= 0x10000
addr = 0x60; dives = []
while addr + 32 <= 0x10000 and len(dives) < n and addr != eop:
    ns = u16le(mem[addr]); length = 32 + 4 × ns
    if addr + length > 0x10000: break
    dives.append((addr, length))
    addr += round_up(length, 32)
for (addr, length) in reversed(dives):     # newest first
    if mem[addr+2 .. addr+7] == known_fingerprint: stop
    dive = mem[addr .. addr+length]
```

Fingerprint: the 6 date/time bytes at dive offset 2.

Confidence: **likely**.

## Dive data format

### Header

32 bytes:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x00 | u16le | | number of samples |
| 0x02 | u8 | | day |
| 0x03 | u8 | | month |
| 0x04 | u8 | | year − 2000 |
| 0x05 | u8 | | second |
| 0x06 | u8 | | minute |
| 0x07 | u8 | | hour |
| 0x08 | u8 | s | dive time, seconds part |
| 0x09 | u8 | min | dive time, minutes part (dive time = minutes × 60 + seconds) |
| 0x14 | u16le | cm | maximum depth |
| 0x16 | u16le | 0.1 °C | maximum temperature |
| 0x18 | u16le | 0.1 °C | minimum temperature |
| 0x1A | u8 | | settings: bits 0-1 sample interval (0 → 1 s, 1 → 2 s, 2 → 5 s, 3 → 10 s); bit 3 set = fresh water, clear = salt water |
| other | | | not decoded |

All dives are reported as freedives. No gas information is stored.

### Samples

Each sample is a `u32le`:

| Bits | Raw unit | Meaning |
|------|----------|---------|
| 0-13 | cm | depth |
| 14-23 | 0.1 °C, offset −20 °C | temperature = raw / 10 − 20 °C |
| 24-31 | beats per minute | heart rate; 0 = no reading |

The first sample is at one interval after the start. Each later sample adds one interval.

### Date and time

Local time; no zone stored. The byte order is day, month, year, second, minute, hour.

Confidence: **likely** (one implementation). The meaning of the max/min temperature header
fields is **uncertain**: they may be swapped, or offset like the samples.

## Model-specific variations

None known.

## Open questions

1. USB-serial chip and VID:PID.
2. Contents of the version response beyond the serial number.
3. Whether the header temperatures use the same −20 °C offset as the samples.
4. Header bytes 0x0A..0x13 and 0x1B..0x1F (likely dive number, surface time and other
   settings).

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- Sporasub product information, <https://www.sporasub.com/>.
