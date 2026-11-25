# Cressi Edy family (Edy, Archimede, Tusa IQ-700)

Older Cressi computers and the Tusa IQ-700 that share one serial protocol. The host sends
single-byte or three-byte commands. The device echoes every command byte, and the host reads
memory pages that end in a fixed trailer byte. Memory is read raw. The logbook is a ring of
up to 60 profile pointers. Dive headers and samples use packed BCD-like nibble digits. There
is no checksum anywhere in the protocol.

Only one public reference covers this family, so most sections are **likely** at best.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Cressi | Archimede | 0x01 | serial | 32-byte pages, nibble-swapped memory, own header layout |
| Tusa | IQ-700 | 0x05 | serial | 8 KiB memory layout |
| Cressi | Edy | 0x08 | serial | 32 KiB memory layout |

The model code is returned by the device during the handshake (see [Session](#session)).
Any code other than 0x01 or 0x05 is handled with the Edy layout.

Identification: serial interface cable only. No USB VID:PID, Bluetooth name or IrDA nickname
is known for this family.

Confidence: **likely**.

## Transport parameters

Serial, 8 data bits, no parity, 1 stop bit, no flow control
(see [../transports/](../transports/)).

| Phase | Baud rate |
|-------|-----------|
| Handshake (all models) | 1200 |
| Data transfer, Edy and IQ-700 | 4800 (switch after the handshake) |
| Data transfer, Archimede | stays at 1200 |

- DTR asserted, RTS de-asserted for the whole session.
- After configuring the line and after the baud-rate switch, wait about 300 ms, then
  discard any pending input and output.
- Read timeout: about 1000 ms per read.
- On a timeout or protocol error, wait 300 ms, discard input and retry the whole command.
  The reference retries 4 times (5 attempts in total).

Confidence: **likely**.

## Session

Every command byte is sent one at a time. After each byte the host reads one byte and
requires it to equal the byte it sent (the echo). Only then does it send the next command
byte. After the last echo the host reads the response, if the command has one. Some
responses end with the trailer byte `45` (ASCII `E`).

Handshake, all at 1200 baud:

```
H→D  41            D→H  41        (echo)
H→D  42            D→H  42
H→D  43            D→H  43
                   D→H  <3 bytes> (content unknown, ignored)

H→D  44            D→H  44
                   D→H  <model:1>

H→D  0C            D→H  0C        (not sent to the Archimede, model 0x01)
                   D→H  45        (trailer)
```

The reference ignores failures of these three steps. When they fail, the model stays 0,
which selects the Edy layout.

After the handshake, Edy and IQ-700 switch the host port to 4800 baud. The Archimede stays
at 1200 baud.

Shutdown:

```
H→D  46            D→H  46        (echo, no further response)
```

Confidence: **likely**.

## Commands

| Command | H→D bytes (each echoed) | Response after the echoes | Notes |
|---------|-------------------------|---------------------------|-------|
| Init 1 | `41 42 43` | 3 bytes | Content unknown |
| Model | `44` | 1 byte: model code | |
| Init 3 | `0C` | `45` | Skipped on Archimede |
| Read page (Edy, IQ-700) | `52 <page:u16be>` | 128 bytes + `45` | page = address / 32 |
| Read page (Archimede) | `45 52 <page:u8>` | 32 bytes + `45` | page = address / 32. Data is nibble-swapped, see below |
| Quit | `46` | none | |

On Edy and IQ-700, a read returns 128 bytes (4 consecutive 32-byte pages) starting at the
requested page. Reads must therefore start on a 32-byte boundary, and the host requests every
fourth page number.

**Archimede nibble swap.** Each byte the Archimede returns has its two nibbles exchanged
(`0x12` arrives as `0x21`). Swap them back before interpreting the data. All offsets and
nibble positions below refer to the swapped-back data.

Confidence: **likely**.

## Framing and integrity

There is no length field, start byte or checksum. Integrity relies only on the per-byte
command echo and the `45` trailer.

Worked example: read the 128 bytes at 0x7F80 from an Edy (page 0x3FC):

```
H→D  52           D→H  52
H→D  03           D→H  03
H→D  FC           D→H  FC
                  D→H  <128 bytes of memory 0x7F80..0x7FFF>
                  D→H  45
```

Confidence: **likely**.

## Memory layout

Pointers are page numbers (units of 32 bytes) relative to the start of the profile area.

| Item | Edy (and unknown models) | IQ-700 and Archimede |
|------|--------------------------|----------------------|
| Memory size | 0x8000 | 0x2000 |
| Profile ring start (inclusive) | 0x3FE0 | 0x0000 |
| Profile ring end (exclusive) | 0x7F80 | 0x1F60 |
| Logbook block (read 128 bytes from) | 0x7F80 | 0x1F80 |
| Logbook pointer width | 2 bytes (u16le) | 1 byte (u8) |
| Logbook slots | 60 (index 0..59) | 60 |
| Control bytes, offset within logbook block | 0x7C | 0x3C |

Within the 128-byte logbook block:

- Slot *i* holds the profile pointer of dive *i* at block offset `i × width`.
- At `ctrl + 0`: index of the most recent slot (`last`, u8).
- At `ctrl + 1`: index of the oldest slot (`first`, u8).
- At `ctrl + 2`: end-of-profile pointer (same width as a slot pointer). It marks the
  position just past the newest dive's data.

A pointer `p` converts to an address as `p × 32 + profile_ring_start`.

If `last` is `0xFF`, the device holds no dives. Any other `first` or `last` outside 0..59 is
a format error.

Confidence: **likely**.

## Download procedure

1. Run the handshake and switch the baud rate (see [Session](#session)).
2. Read 128 bytes from the logbook block address (Archimede: four 32-byte page reads).
3. Return early if `last == 0xFF`. Otherwise validate `first` and `last`.
4. Number of dives: `count = ((last − first) mod 60) + 1`.
5. Walk the slots from `last` backwards (index decreasing, 0 wraps to 59), `count` times.
   Start with `prev = end-of-profile`. For each slot:
   - `cur` = slot address (pointer converted as above). It must lie inside the profile ring.
   - `len = (prev − cur) mod ring_size`. A result of 0 means the whole ring.
   - Stop if the running total of `len` would exceed the ring size. Older dives have been
     overwritten.
   - Read `len` bytes from `cur`, wrapping at the ring end, using page-aligned reads.
   - The dive data must be at least 32 bytes long (the header).
   - Incremental download: the fingerprint is the first 16 bytes of the dive data. Stop at
     the first dive whose fingerprint equals the stored one.
   - `prev = cur`.
6. Send Quit (`46`).

The memory can also be dumped in full by reading from address 0 up to the memory size.

Confidence: **likely**.

## Dive data format

The dive data is a 32-byte header followed by sample records. Most fields are decimal digits
stored one per nibble. Nibble positions are counted from the start of the dive data:
nibble 2k is the **high** nibble of byte k, and nibble 2k+1 is the low nibble. An *n*-digit
field starting at nibble *p* is read as the decimal number formed by nibbles *p .. p+n−1*,
most significant first.

### Header

| Field | Edy / IQ-700: nibble, digits | Archimede: nibble, digits | Conversion |
|-------|------------------------------|---------------------------|------------|
| Average depth | 1, 3 | 22, 3 | 1/10 m |
| Maximum depth | 5, 3 | 9, 3 | 1/10 m |
| Year | 8, 2 | 2, 2 | + 2000 |
| Month | 10, 1 | 5, 1 | 1..12 (one nibble, so 10..12 are hex digits A..C) |
| Day | 11, 2 | 6, 2 | |
| Minimum temperature | 22, 3 | 45, 3 | 1/10 °C (reference value; see open questions) |
| Dive time | 25, 3 | 29, 3 | see below |
| Hour | 28, 2 | 25, 2 | |
| Minute | 30, 2 | 27, 2 | |
| Gas mix 1 O2 | 46, 2 | 43, 2 | percent |
| Gas mix 2 O2 | 44, 2 | – | percent |
| Gas mix 3 O2 | 42, 2 | – | percent |

Dive time:
- Edy (0x08): first digit × 60 + the following two digits, in seconds. This looks like
  minutes and seconds.
- Other models: all three digits are minutes. Multiply by 60 for seconds.

Gas mixes: Edy and IQ-700 have up to 3, Archimede has 1. A mix is active unless the first
nibble of its field is `F`. Active mixes are stored first, so counting stops at the first
inactive one. Helium is 0.

IQ-700 sample interval flag: bit 6 (0x40) of byte 0x07.

### Samples

Records start at byte 32 and are read until fewer than 2 bytes remain or the first byte of
a record is `0xFF` (end marker).

| Bytes | Meaning |
|-------|---------|
| 0, bit 7 | If set, 4 extra bytes follow this record (6 bytes in total). Their content is unknown |
| 0, bits 5..6 | Gas mix index (Edy only. Use 0 on IQ-700 and Archimede) |
| nibbles 1..3 | Depth, 3 decimal digits, 1/10 m (low nibble of byte 0, then both nibbles of byte 1) |

Sample interval:

| Model | Interval |
|-------|----------|
| Edy | 1 s |
| IQ-700 with header byte 0x07 bit 6 set | 15 s |
| IQ-700 otherwise, Archimede | 30 s |

The first record is at time = 1 × interval. A gas mix index at or above the number of active
mixes is a format error. Report a gas switch only when the index changes.

### Date and time

Local time with no time zone and no seconds (see the header). The year is two digits from 2000.

Confidence: **likely** for the layout. **uncertain** for the dive time encoding and the
meaning of the extra 4 bytes.

## Model-specific variations

- **Archimede (0x01):** no `0C` handshake step. Stays at 1200 baud. Reads 32-byte pages with
  a different command. Data is nibble-swapped. Uses the IQ-700 memory map (8 KiB, 1-byte
  pointers) and its own header nibble positions. Has only one gas mix.
- **IQ-700 (0x05):** 8 KiB memory map, 1-byte pointers, 15 s or 30 s sample interval.
- **Edy (0x08):** 32 KiB memory map, 2-byte pointers, 1 s samples, gas index in samples.
- The same dive data format is also used by the Zeagle N2iTiON 3. That device uses a
  different download protocol and is not covered here.

## Open questions

1. What is in the 3-byte response to `41 42 43`? It may be firmware or serial information.
2. Dive time on the Edy: is it really minutes:seconds? That reading limits the field to
   under 10 minutes.
3. What are the 4 extra bytes after a sample with bit 7 set? Alarms, events or deco data?
4. The minimum temperature conversion (1/10 °C) has not been checked against a real dive.
5. What are the units of the IQ-700 interval flag, and which other header bytes hold
   settings?
6. Is the Edy-layout fallback for unknown model codes correct for other Cressi models of
   that era?
7. No serial number or firmware version is read. Are they somewhere in memory?

## References

- libdivecomputer (LGPL-2.1), used as a factual reference. No code copied.
