# Pelagic Veo 250 protocol (Oceanic, Genesis, Aeris, Sherwood, Hollis, Seemann)

The Veo 250 protocol is used by a group of mid-2000s Pelagic computers without air integration:
Oceanic Veo 180, Veo 200 and Veo 250; Genesis React Pro; Aeris XR-2; Sherwood Insight; Hollis DG02;
Seemann XP5. They connect through a serial interface cable at 9600 baud. The cable contains a
microcontroller that must be initialised first. The host reads memory in 16-byte pages, up to four
per command. Each page is followed by an 8-bit checksum, and every reply ends with a trailer byte.
Memory layout and download algorithm follow [pelagic-common.md](pelagic-common.md). The dive
record format described here is also used by the Genesis React Pro White, which itself downloads
with the [atom2 protocol](pelagic-atom2.md).

## Devices

| Vendor | Model | Model code | Transport | Notes |
|--------|-------|------------|-----------|-------|
| Genesis | React Pro | `4247` | serial | |
| Oceanic | Veo 200 | `424B` | serial | |
| Oceanic | Veo 250 | `424C` | serial | |
| Seemann | XP5 | `4251` | serial | version string starts `SEEMANN R` |
| Oceanic | Veo 180 | `4252` | serial | |
| Aeris | XR-2 | `4255` | serial | |
| Sherwood | Insight | `425A` | serial | |
| Hollis | DG02 | `4352` | serial | |
| Genesis | React Pro White | `4354` | serial | atom2 protocol, this dive format |

All use layout L-32V. Identification is by version string; see the
[model table](pelagic-common.md#devices-and-model-codes).

Confidence: **likely**.

## Transport parameters

| Parameter | Value |
|-----------|-------|
| Baud rate | 9600 |
| Framing | 8N1, no flow control |
| Read timeout | 3000 ms |
| DTR | high for the whole session |
| RTS | pulsed low to reset the cable |

See [../transports/serial.md](../transports/serial.md).

Confidence: **likely**.

## Session

1. Open the port at 9600 8N1. Set DTR high.
2. Set RTS low, wait 100 ms, set RTS high, wait 100 ms. Discard buffered data.
3. **Cable init.** Send `55 00`. The cable answers with 13 ASCII bytes `PPS--OK_V2.00`
   (`50 50 53 2D 2D 4F 4B 5F 56 32 2E 30 30`). If no byte arrives at all, continue anyway. A
   different non-empty answer is an error.
4. Wait 100 ms.
5. **Version.** Send `90 00`. This also switches the device into download mode; the device must be
   in PC mode, which the user selects on the device. The reply is `5A`, 16 version bytes, a sum8 of
   those 16 bytes, and the trailer `A5`.
6. Match the version string (see [pelagic-common.md](pelagic-common.md#version-string-matching)).
7. Download (see below).
8. **Quit.** Send `98 00`. No reply is expected.

Variant: another implementation expects `5A` followed by 13 bytes for the cable init. It does not
check them or send the quit command.

Confidence: **likely**.

## Commands

| Command | H→D bytes | D→H response | Notes |
|---------|-----------|--------------|-------|
| Cable init | `55 00` | 13 ASCII bytes, or nothing | no ACK |
| Version | `90 00` | `5A` + 16 bytes + sum8 + `A5` | |
| Read pages | `20 FL FH LL LH 00` | `5A` + n × (16 bytes + sum8) + `A5` | F = first page, L = last page, both **little-endian** page numbers (address / 16), n = L − F + 1 ≤ 4 |
| Keep-alive | `91 LL LH 00` | `5A A5 A5` | L = last page number read (little-endian) |
| Quit | `98 00` | none | |

Before every command, the host discards pending input. If the first reply byte is not `5A`, or
nothing arrives, wait 100 ms and resend (one implementation allows 2 resends). The last byte of
every reply with data MUST be `A5`.

Note that the page numbers here are little-endian, whereas the VT Pro protocol uses big-endian.

Confidence: **likely**.

## Framing and integrity

Commands are raw bytes. Replies are `5A`, the payload, and the trailer `A5`. Each 16-byte block of
the payload (version string or memory page) is followed by its [sum8](../common/checksums.md#sum8).

Worked example (Veo 250, firmware bytes `31 30`):

```
H→D  90 00
D→H  5A 56 45 4F 20 32 35 30 20 52 31 30 20 32 35 36 4B 7C A5
```

The string is "VEO 250 R10 256K". sum8 of the 16 bytes = 0x7C.

Worked example, reading the pointer page (address 0x0040 = page 4):

```
H→D  20 04 00 04 00 00
D→H  5A <16 bytes> <sum8> A5
```

Confidence: **likely**.

## Memory layout

Layout L-32V (see [pelagic-common.md](pelagic-common.md#memory-layouts)): memory 0x8000,
device-info page 0x0000 (serial encoding S1), pointer page 0x0040 (logbook pointer mode G1),
logbook ring 0x0400..0x0600 with 8-byte entries, profile ring 0x0600..0x8000, profile pointer
form E1.

Confidence: **likely**.

## Download procedure

Follow [the common procedure](pelagic-common.md#download-procedure), reading up to 4 pages per
command.

Confidence: **likely**.

## Dive data format

A dive record is the 8-byte logbook entry followed by the profile. Its minimum size is 56 bytes.

```
0x00..0x07   logbook entry
0x08..0x27   profile header
0x28..       8-byte samples
last 16      footer
```

### Header

| Field | Location | Type, unit | Conversion |
|-------|----------|------------|------------|
| Sample interval | b0x27 & 0x03 | index | 0..3 → 2, 15, 30, 60 s. React Pro and React Pro White: use index (i + 1) mod 4 instead |
| Dive time | b(footer + 3), b(footer + 4) | minutes, hours | seconds = 60 × b(footer+3) + 3600 × b(footer+4) |
| O2 | b(footer + 6) | % | 0 → 21 % |

There is one gas mix and no tank pressure. Max depth is taken from the samples.

### Samples

8-byte samples from 0x28 to the footer. Skip all-`00` samples. Each sample is one interval after
the previous one.

| Field | React Pro, React Pro White, Insight | All others |
|-------|-------------------------------------|------------|
| Depth | s2, ft | s2, ft |
| Temperature | s6, °F | s7, °F |
| Deco stop | s7 >> 4, units of 10 ft (0 = none) | s5 >> 4, units of 10 ft |
| Deco / NDL time | ((s3 & 0xC0) << 2) \| s4, minutes | u16le@4 & 0x0FFF, minutes |

When the stop value is 0, the time is the no-decompression limit.

### Date and time

From the logbook entry:

| Field | Source |
|-------|--------|
| Minute | b2 (binary) |
| Hour | b3 (binary, 0..23) |
| Day | b1 & 0x1F |
| Month | b7 >> 4 |
| Year | 2000 + (b5 >> 4) + ((b1 & 0xE0) >> 1), then +3 for Veo 200 and Veo 250, +2 for React Pro |

No seconds, no time zone.

Worked example (Veo 250): logbook bytes `xx 0C 1E 0E xx 90 xx 60`.
- day = 0x0C & 0x1F = 12
- minute = 0x1E = 30, hour = 0x0E = 14
- month = 0x60 >> 4 = 6
- year = 2000 + (0x90 >> 4) + ((0x0C & 0xE0) >> 1) + 3 = 2000 + 9 + 0 + 3 = 2012

Result: 2012-06-12 14:30.

Confidence: **likely** for fields; **uncertain** for the year composition (see open questions).

## Model-specific variations

- React Pro and React Pro White: shifted interval index, different temperature and deco bytes.
- Insight: same sample bytes as the React Pro, normal interval index.
- React Pro White downloads with the atom2 protocol (layout L-64R).

## Open questions

- The year formula combines a 4-bit field with a 3-bit field weighted by 16, and adds per-model
  offsets. Real dumps are needed to confirm it.
- Whether a `5A` precedes the cable init reply (the sources disagree).
- Meaning of the other logbook, header and footer bytes (for example tank or gas settings,
  max depth).

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used as a
  factual reference; no code copied.
