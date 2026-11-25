# Citizen Hyper Aqualand

The Citizen Hyper Aqualand is a diving watch with a depth gauge. Its PC link is a simple
serial stream. The host raises DTR, sends a start byte, then sends one request byte per
32-byte block. A block ending in `FF` is the last one. The whole transfer is a single
record holding the most recent dive: a 32-byte header, then a depth table and a
temperature table of 12-bit BCD values.

No public vendor documentation is known. The only source is libdivecomputer, so the
confidence here is at most **likely**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Citizen | Hyper Aqualand | 0 | serial (Citizen PC interface) | gauge only; metric or imperial units, set on the watch |

There are no USB, Bluetooth or IrDA identities. The interface appears to the host as a
serial port, and DTR must be driven (see below).

Confidence: **likely**.

## Transport parameters

- 4800 baud, 8N1, no flow control.
- Read timeout 1000 ms.
- After opening: wait 300 ms, then flush both directions.
- DTR is asserted for the duration of the transfer and cleared afterwards. The interface
  probably draws power from DTR or uses it as an enable line (inferred).

Confidence: **likely**.

## Session

```
host: set DTR = 1
H→D  7F                  start
     (wait 1200 ms)
H→D  FF                  request block
D→H  <32 bytes>
H→D  FF                  request the next block
D→H  <32 bytes>
...                      repeat until byte 31 of a received block is FF
host: set DTR = 0
```

The reference sends one more `FF` after receiving the final block, because it sends the
request before checking the end condition. Whether the device needs that byte is unknown.

## Commands

| H→D | Meaning | Response |
|-----|---------|----------|
| `7F` | start transfer | none; the host waits 1200 ms |
| `FF` | next block | 32 bytes |

## Framing and integrity

None. Blocks are raw 32-byte chunks with no checksum. The end of the transfer is marked by
`FF` in the last byte of a block.

## Memory layout

Not addressable. The device sends one record of N × 32 bytes containing one dive
(presumably the most recent).

## Download procedure

1. Run the session above and concatenate all blocks.
2. If fewer than 32 bytes arrived, the data is invalid.
3. Fingerprint: bytes 0x05–0x0C (the date/time field). If they equal the stored
   fingerprint, the dive is already known; otherwise emit the record as one dive.

Confidence: **likely**.

## Dive data format

### Header (32 bytes)

| Offset | Type | Meaning |
|--------|------|---------|
| 0x04 | u8 | units: `A6` = imperial (feet, °F); any other value = metric (meaning inferred) |
| 0x05 | bcd8 | century (e.g. `20`) |
| 0x06 | bcd8 | year within the century |
| 0x07 | bcd8 | month |
| 0x08 | bcd8 | day |
| 0x09 | bcd8 | not used (possibly day of week) |
| 0x0A | bcd8 | hour |
| 0x0B | bcd8 | minute |
| 0x0C | bcd8 | second |
| 0x12–0x13 | 3 BCD digits | maximum depth: both digits of 0x12, then the high nibble of 0x13. Metric: 0.1 m; imperial: 1 ft |
| 0x16–0x17 | 3 BCD digits | dive time in minutes: the low nibble of 0x16, then both digits of 0x17 |

Other header bytes are not decoded.

Example: `20 24 07 15 xx 09 30 00` at 0x05 → 2024-07-15 09:30:00 (local time, no zone).
Max depth bytes `18 5x` → 185 → 18.5 m (metric) or 185 ft (imperial).

### Samples

From byte 0x20, the profile is a stream of 12-bit values, each three BCD nibbles
(hundreds, tens, units). Values are packed back to back, so a value may start on either
half of a byte. There are two tables in a row:

1. Depth table, ended by a byte `EF`.
2. Temperature table, ended by a byte `FF`.

Reading rule used by the reference: before decoding a value at the current nibble
position, look at the byte containing that nibble and at the next byte. If either equals
the table's end marker, move to the next byte boundary. That byte MUST be the marker.
Skip it and continue with the next table at the following byte.

Depth table: one value every **5 s**. Time of entry k (from 1) is 5k s. Units: 0.1 m
(metric) or 1 ft (imperial). Every 12th entry holds the bogus value 999. It must not be
dropped, because that would shift the timing. The reference replaces it with the mean of
its two neighbours.

Temperature table: one value per **300 s**. Entry j belongs to time 300·j s (entry 0 to
the dive start). Units: 0.1 °C (metric) or 1 °F (imperial).

The dive mode is gauge, with no gas information.

### Date and time

BCD fields in local time, with no time zone (see header).

Confidence: **uncertain**. The data comes from one source, the meaning of the units byte
is inferred, and the end-marker alignment rule is unverified.

## Model-specific variations

None known.

## Open questions

1. Does the device store more than one dive? Can earlier dives be requested?
2. The meaning of header bytes 0x00–0x04 and 0x0D–0x1F apart from those listed.
3. Exact semantics of the units byte (`A6`).
4. Is the trailing `FF` after the last block needed or harmless?

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
