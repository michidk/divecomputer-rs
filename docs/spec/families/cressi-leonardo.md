# Cressi Leonardo family (Leonardo, Giotto, Newton, Drake)

Cressi computers with a 32,000-byte memory and a serial interface cable. Commands and
responses are short ASCII frames wrapped in `{` and `}`. They carry hexadecimal text
protected by a CRC-16. A special command streams the whole memory as raw binary with a
trailing CRC. The host always dumps the full memory and then extracts dives from a logbook
ring and a profile ring.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Cressi | Leonardo | 1 | serial | |
| Cressi | Giotto | 4 | serial | |
| Cressi | Newton | 5 | serial | |
| Cressi | Drake | 6 | serial | Freediving computer: per-dive sample interval, no gas mix |

The model code is the byte at memory address 0x0000 (see [Memory layout](#memory-layout)).

Identification: USB-serial interface cable. No VID:PID is established by public sources
(see open questions). No wireless transport is known.

Confidence: **likely**.

## Transport parameters

- Serial, 115200 baud, 8 data bits, no parity, 1 stop bit, no flow control.
- Read timeout: around 1000 ms per read in one reference, 600 ms in another.
- The full-memory command can take several seconds. Allow at least 10 to 15 s for the
  whole transfer.

Two wake-up sequences are documented. Both end with RTS asserted and DTR de-asserted:

| Variant | Sequence |
|---------|----------|
| A | Assert RTS. Assert DTR. Wait 200 ms. De-assert DTR. Wait 100 ms. Discard pending input and output |
| B | Assert RTS. Assert DTR. Wait 30 ms. De-assert DTR. Wait 3 ms. Assert DTR. Wait 3 ms. De-assert DTR |

Confidence: **likely** for the line settings. **uncertain** for which wake-up timing the
device actually needs.

## Session

There is no explicit handshake beyond the line wake-up. A host can:

1. Optionally read the identity bytes with a normal memory read (see [Commands](#commands)).
   One reference reads 19 bytes (0x13) at address 0x1000. It takes the model from the first
   byte and the serial number from the next three bytes (u24le). The other reference takes
   model and serial from memory address 0x0000 of the full dump instead.
2. Run the full-memory dump.

No shutdown command is known.

Confidence: **likely** for the dump. **uncertain** for the identity location (see open
questions).

## Commands

### Read memory

```
H→D  '{' <addr:4 hex> <len:4 hex> <crc:4 hex> '}'      (14 bytes)
D→H  '{' <data:2×len hex> <crc:4 hex> '}'              (2×len + 6 bytes)
```

- `addr` and `len` are 16-bit values written as 4 uppercase ASCII hex digits, most
  significant first.
- One reference limits `len` to 32 bytes per request and loops for larger reads.
- Each data byte is 2 uppercase hex digits.

### Dump all memory

```
H→D  7B 31 32 33 44 42 41 7D             "{123DBA}"
D→H  7B 21 44 35 42 33 7D                "{!D5B3}"   (header, verified by one reference)
D→H  <32000 raw binary bytes>
D→H  <crc:4 ASCII hex digits>
```

The other reference does not check the 7-byte header. It discards everything up to and
including the first `}`.

There are no ACK or NAK bytes. On a bad CRC or a timeout the host discards input
(after about 100 ms) and re-sends the command. The references allow 4 to 20 retries for
reads and up to 3 attempts for the dump.

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Read memory | `{AAAALLLLCCCC}` | address, length | `{` hex data, CRC `}` | ≤ 32 bytes per request in one reference |
| Dump | `{123DBA}` | none | `{!D5B3}` + 32000 bytes + 4 hex CRC | |

Confidence: **likely**.

## Framing and integrity

All CRCs are CRC-16/CCITT-FALSE: width 16, poly 0x1021, init 0xFFFF, refin false,
refout false, xorout 0x0000. See
[../common/checksums.md#crc-16-ibm-3740](../common/checksums.md#crc-16-ibm-3740) (catalogue
name CRC-16/IBM-3740, check value 0x29B1).

| Frame | CRC covers | CRC transmitted as |
|-------|------------|--------------------|
| Read command | The 8 ASCII hex characters of address and length | 4 uppercase hex digits, high byte first |
| Read response | The 2×len ASCII hex data characters (between `{` and the CRC) | 4 hex digits, high byte first |
| Dump | The 32000 **binary** bytes | 4 ASCII hex digits, high byte first |

Worked example: read 0x13 bytes at 0x1000. The CRC over the ASCII text `10000013` is 0x9306.

```
H→D  7B 31 30 30 30 30 30 31 33 39 33 30 36 7D        "{100000139306}"
```

Read 0x20 bytes at 0x0000. The CRC over `00000020` is 0xB1E5.

```
H→D  7B 30 30 30 30 30 30 32 30 42 31 45 35 7D        "{00000020B1E5}"
```

Response carrying the two data bytes `01 02`. The CRC over `0102` is 0x4938.

```
D→H  7B 30 31 30 32 34 39 33 38 7D                    "{01024938}"
```

Confidence: **likely** (the command example was computed independently and matches the
second reference).

## Memory layout

Total size 32000 bytes (0x0000..0x7CFF).

| Address | Size | Content |
|---------|------|---------|
| 0x0000 | 1 | Model code |
| 0x0001 | 3 | Serial number, u24le |
| 0x0062 | 2 | Number of dives, u16le (informational) |
| 0x0064 | 2 | Address of the most recent logbook entry, u16le |
| 0x0066 | 2 | End-of-profile pointer, u16le |
| 0x0100..0x1437 | 60 × 82 | Logbook ring: 60 entries of 82 (0x52) bytes |
| 0x1000 | 19 | Identity block read by one reference (lies inside the logbook area; see open questions) |
| 0x1438..0x7CFF | 27080 | Profile ring buffer |

Logbook entry *n* starts at `0x0100 + n × 82`. The latest-entry pointer must satisfy
`(ptr − 0x0100) mod 82 == 0`.

An entry whose 82 bytes are all `0xFF` is unused.

Each logbook entry holds two profile pointers. The pointers mark the first and the last word
of the dive's profile block in the ring:

| Entry offset | Type | Meaning |
|--------------|------|---------|
| 2 | u16le | `start`: address of the block's first word |
| 4 | u16le | `end`: address of the block's last word |

The profile block itself repeats the pointers as a cross-check. The word at `start` equals
`end`, and the word at `end` equals `start`. The samples are the bytes strictly between
them: from `start + 2` for `((end − start) mod 27080) − 2` bytes, wrapping at 0x7D00 back to
0x1438.

Confidence: **likely**.

## Download procedure

1. Wake up the line and send the dump command. Verify the header and the CRC.
2. `latest = (word@0x0064 − 0x0100) / 82`.
3. For `i` in 0..59: `idx = (latest − i) mod 60`, newest first.
   - Stop at an all-`0xFF` entry.
   - Check that `start` and `end` lie in the profile ring. Check that the previous (newer)
     dive's `start` equals this dive's `end + 2`. For the newest dive, compare with the
     end-of-profile pointer at 0x0066.
   - Incremental download: the fingerprint is the 5 bytes at entry offset 8 (year, month,
     day, hour, minute). Stop when it matches the stored fingerprint.
   - Track the profile space consumed (`length + 4` per dive). Once the running total
     exceeds the ring size, emit older dives with an empty profile.
   - Verify the cross-check words, then copy the samples (with wrap).
   - Output the dive as the 82-byte entry followed by the sample bytes.

A simpler alternative walks the logbook forwards from 0x0100 in 82-byte steps. It stops when
the entry's first u16 is 0xFFFF and filters dives by date. It fetches the samples from
`start + 2` in the same way.

Confidence: **likely**.

## Dive data format

### Header (logbook entry, 82 bytes)

| Offset | Type | Raw unit | Conversion / meaning |
|--------|------|----------|----------------------|
| 0x00 | u16le | – | 0xFFFF in an unused entry |
| 0x02 | u16le | address | Profile block start pointer |
| 0x04 | u16le | address | Profile block end pointer |
| 0x06 | u16le | samples | Number of samples. Dive time = value × interval |
| 0x08 | u8 | year − 2000 | Fingerprint byte 0 |
| 0x09 | u8 | month 1..12 | |
| 0x0A | u8 | day | |
| 0x0B | u8 | hour | |
| 0x0C | u8 | minute | Fingerprint byte 4 |
| 0x17 | u8 | s | Sample interval (Drake only. Other models use a fixed 20 s). 0 is invalid |
| 0x18 | u8 | 1/10 bar | Maximum ppO2 setting (not Drake) |
| 0x19 | u8 | % | O2 of the gas mix (not Drake). One reference says 0xFF means air (21 %) |
| 0x20 | u16le | 1/10 m | Maximum depth |
| 0x22 | u8 | °C | Minimum (bottom) temperature, whole degrees |

The Drake has no gas mix. The other models have exactly one, with no helium.

### Samples

Each sample is a u16le word, read through the ring. Repeat while at least 2 bytes remain:

- **Surface marker:** if at least 4 bytes remain and the *following* word is `0xFF00`, the
  current word is a surface interval. The duration in seconds is `byte0 + (byte1 & 0x07) × 60`.
  Depth is 0 and time advances by that duration. Consume 4 bytes.
- **Depth sample**, otherwise. Time advances by the sample interval.

| Bits | Meaning |
|------|---------|
| 0..10 (`& 0x07FF`) | Depth, 1/10 m |
| 14..15 (`& 0xC000`) | Ascent-rate warning level 0..3 (0 = none) |

A second reference masks depth with `0x00FF` and the flag with `0xFF00` for the Drake
(model 6). See open questions.

The first depth sample is at time = 1 × interval.

### Date and time

Local time, no time zone, no seconds. Year = byte + 2000.

Confidence: **likely** for the shared fields. **uncertain** for the Drake bit masks and for
the ppO2 and O2 settings.

## Model-specific variations

- **Drake (6):** the sample interval is stored per dive at 0x17. There is no gas mix. The
  depth mask may differ (see above).
- **Leonardo, Giotto, Newton:** fixed 20 s interval, one nitrox mix.

## Open questions

1. Where do the identity bytes live? One reference uses dump bytes 0x0000..0x0003, the other
   a separate 19-byte read at 0x1000, which falls inside the logbook ring. Possibly the
   device maps a separate identity area at 0x1000 for the read command. This needs a dump.
2. What do the 19 bytes at 0x1000 contain beyond model and serial (firmware?)?
3. Drake depth encoding: is the mask `0x07FF` or `0x00FF`, and what is in the upper bits?
4. What do the ascent flag levels mean (which rates)? One host treats levels 1 and 2 as
   informational only.
5. Which wake-up timing (variant A or B) is required, and is the dump header always
   `{!D5B3}`?
6. Does the read-memory command accept lengths larger than 32?
7. What USB VID:PID does the original cable use?

## References

- libdivecomputer (LGPL-2.1), used as a factual reference. No code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), used as a factual reference. No code copied.
- Greg Cook, "Catalogue of parametrised CRC algorithms" (RevEng),
  https://reveng.sourceforge.io/crc-catalogue/
