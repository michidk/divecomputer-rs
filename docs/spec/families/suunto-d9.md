# Suunto D9 / Vyper 2 family (D-series, i-series, Vyper 2, Cobra 2/3, Vyper Air, HelO2)

Suunto's third-generation wired-interface computers (roughly 2004–2018). All share one packet
protocol: a command byte, a 16-bit big-endian length, parameters, and an XOR checksum, over a
half-duplex serial cable switched with RTS. Memory is read in blocks of up to 120 bytes. Dives
live in a ring buffer as a doubly linked list whose head pointers are in a fixed header block.

Two **link variants** exist, differing only in how the host drives the cable:

| Variant | Devices | Speed | Echo | RTS while sending |
|---------|---------|-------|------|-------------------|
| **V2** ("Vyper 2 link") | Vyper 2, Cobra 2, Vyper Air, Cobra 3, HelO2 | 9600 | no | asserted |
| **D9** ("D-series link") | D9, D6, D4, and the i-series (D4i, D6i, D9tx, DX, Vyper Novo, Zoop Novo, D4f) | 9600 (classic D) or 115200 (i-series) | yes, interface echoes every command byte | cleared |

Commands, framing, memory layout and dive format are common to both variants. Dive record
layouts differ by **model group** (classic, D4, HelO2, i-series, DX).

## Devices

| Vendor | Model | Model code | Link variant | Memory layout | Dive group |
|--------|-------|------------|--------------|---------------|------------|
| Suunto | D9 | `0x0E` | D9, 9600 | 32 KiB | classic |
| Suunto | D6 | `0x0F` | D9, 9600 | 32 KiB | classic |
| Suunto | Vyper 2 | `0x10` | V2 | 32 KiB | classic |
| Suunto | Cobra 2 | `0x11` | V2 | 32 KiB | classic |
| Suunto | D4 | `0x12` | D9, 9600 | 32 KiB | D4 |
| Suunto | Vyper Air | `0x13` | V2 | 32 KiB | classic |
| Suunto | Cobra 3 | `0x14` | V2 | 32 KiB | classic |
| Suunto | HelO2 | `0x15` | V2 | 32 KiB, HelO2 fingerprint | HelO2 |
| Suunto | D4i (also "D4i Novo") | `0x19` | D9, 115200 | 64 KiB | i-series |
| Suunto | D6i (also "D6i Novo") | `0x1A` | D9, 115200 | 64 KiB | i-series |
| Suunto | D9tx | `0x1B` | D9, 115200 | 64 KiB | i-series |
| Suunto | DX | `0x1C` | D9, 115200 | 64 KiB, DX fingerprint | DX |
| Suunto | Vyper Novo | `0x1D` | D9, 115200 | 64 KiB | i-series |
| Suunto | Zoop Novo | `0x1E`, `0x1F` | D9, 115200 | 64 KiB | i-series |
| Suunto | D4f | `0x20` | D9, 115200 | 64 KiB | i-series |

The model code is byte 0 of the version reply (see Session).

Identification: wired only, through Suunto's USB interface cable (an FTDI USB-serial bridge;
one source associates FTDI product ID `0403:F680` with it, unconfirmed). See
[../transports/](../transports/).

Confidence: **likely**.

## Transport parameters

Common: 8 data bits, no parity, 1 stop bit (8N1), no flow control. DTR is asserted for the
whole session (it powers the interface) and cleared at the end. After asserting DTR, wait,
then purge both directions before the first command.

| Parameter | V2 variant | D9 variant |
|-----------|------------|------------|
| Baud rate | 9600 | 9600 for D4/D6/D9, 115200 for i-series |
| Read timeout | 3000 ms (reference) | 3000 ms (reference); 600 ms (other source) |
| Settle after DTR | 100 ms (reference); 1000 ms (other source) | same |
| Before each command | wait ≥ 600 ms (reference) or ≥ 680 ms since previous command (other source) | ≥ 200 ms since previous command (other source, classic D only); none (reference) |
| Send sequence | assert RTS, write, wait until the bytes have left the UART, clear RTS, read reply | clear RTS, write, read and verify the echo, assert RTS, read reply |
| Echo | none | exact copy of the command |

Waiting until the bytes have left the UART (V2): the reference computes
`bytes × 10 bits / 9600 baud + 2 ms` from the moment the write started and sleeps for the
remainder; the other source sleeps 5 ms before and 15 ms after the write. Both clear RTS before
reading.

D9 variant, other source: after the write it sleeps 8 ms and then asserts RTS, reading the echo
together with the reply.

**Speed autodetection (D9 variant).** A host that does not know the model can try the version
command at 9600 baud and, on failure, at 115200 baud (try 115200 first when the user selected an
i-series model). The successful speed is used for the rest of the session.

Confidence: **likely** for line settings, RTS polarity and echo behaviour; **uncertain** for
exact delays.

## Session

```
open port 8N1, assert DTR, settle, purge
H→D  0F 00 00 0F                       version
D→H  0F 00 04 <model> <fw2> <fw1> <fw0> <xor>
```

- `model`: model code (table above).
- `fw2 fw1 fw0`: firmware version, 24-bit big-endian (one source only uses it for i-series).

Then read the serial number: 4 bytes at `0x0023` (classic, D4, HelO2, V2 devices) or `0x0024`
(i-series and DX). Each byte is a value 0..99; concatenate them as two-digit decimal groups
(e.g. `0C 22 38 01` → 12345601). The reference reads 8 bytes there and uses the first 4.

There is no explicit logout; clear DTR and close.

Confidence: **likely**.

## Commands

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Version | `0F 00 00 0F` | none | `0F 00 04 <4 bytes> <xor>` | 8 bytes |
| Read memory | `05 00 03 AH AL N <xor>` | 16-bit address, `N` ≤ 120 (`0x78`) | `05 LH LL AH AL N <N bytes> <xor>` | `LH LL` = N + 3 |
| Write memory | `06 LH LL AH AL N <N bytes> <xor>` | address, `N` ≤ 120, data; `LH LL` = N + 3 | `06 00 03 AH AL N <xor>` | not needed for download |
| Set clock | `10 00 07 YH YL MM DD hh mm ss <xor>` | year u16be, month, day, hour, minute, second | `10 00 01 <status> <xor>` | `status` = `01` on success |
| Reset max depth | `20 00 00 20` | none | `20 00 00 20` | clears the stored maximum depth |

Response validation (all commands):

1. Byte 0 equals the command byte.
2. Bytes 1–2, as u16be, plus 4 equals the total response length.
3. The parameter bytes that the response repeats (for a read: address and count) equal the
   request.
4. The last byte equals the XOR of all preceding response bytes.

A failed exchange (timeout or validation failure) may be retried; the reference retries twice
(three attempts), the other source up to 16 times with 50 ms pauses.

Confidence: **likely**. The read-reply length bytes are given by the reference as `N + 3`;
the other source describes the reply header as a copy of the request (`05 00 03 …`). See open
questions.

## Framing and integrity

```
<cmd:1> <len:2, u16be> <payload:len> <xor:1>
```

`len` counts only the payload. The checksum is the 8-bit XOR of every preceding byte of the
packet, initial `00` ([../common/checksums.md#xor8](../common/checksums.md#xor8); test vector:
ASCII `123456789` → `31`).

On the D9 variant the host first receives an exact echo of the command (same length), which
MUST be checked and discarded before the response.

Worked example (read the 8-byte ring header at `0x0190`, D9 variant at 9600 baud):

```
H→D  05 00 03 01 90 08 9F
D→H  05 00 03 01 90 08 9F                         echo
D→H  05 00 0B 01 90 08 9A 01 05 00 10 20 9A 01 A2
     │  └len=11┘ addr  N  └──── 8 data bytes ───┘ xor
```

Set-clock example for 2026-11-24 12:34:56:

```
H→D  10 00 07 07 EA 0B 18 0C 22 38 FF
```

Confidence: **likely**.

## Memory layout

| Item | Classic, D4, V2 devices | HelO2 | i-series | DX |
|------|-------------------------|-------|----------|----|
| Memory size | `0x8000` | `0x8000` | `0x10000` | `0x10000` |
| Serial number | `0x0023` | `0x0023` | `0x0024` | `0x0024` |
| Ring header | `0x0190` | `0x0190` | `0x0190` | `0x0190` |
| Profile ring | `0x019A..0x7FFD` | `0x019A..0x7FFD` | `0x019A..0xEBEF` | `0x019A..0xEBEF` |
| Fingerprint (offset in dive data, 7 bytes) | `0x11` | `0x17` | `0x13` | `0x17` |

The ring's exclusive end is `0x7FFE` (32 KiB devices) or `0xEBF0` (64 KiB devices).

### Ring header at `0x0190` (8 bytes)

| Offset | Type | Meaning |
|--------|------|---------|
| `0x0190` | u16le | address of the start of the **newest** dive |
| `0x0192` | u16le | non-zero when the ring holds data (disambiguates full vs. empty when the two pointers below are equal) |
| `0x0194` | u16le | **end** pointer: address just past the newest dive (write position) |
| `0x0196` | u16le | **begin** pointer: start of the oldest data still in the ring |

The newest-dive and end pointers MUST lie inside the ring. If the begin pointer is outside the
ring, treat the whole ring as occupied.

### Dive record in the ring

```
+0  u16le  address of the previous (older) dive record
+2  u16le  address of the next (newer) record, i.e. the end of this one
+4  ...    dive data (header, sample configuration, samples)
```

The dive data passed to the parser below starts at `+4`; all dive-data offsets in this
document are relative to that point. (Offsets measured from the record start are 4 larger.)

Confidence: **likely** (the pointer roles agree between sources; the meaning of `0x0192` and the
begin pointer comes from the reference only).

## Download procedure

```
read version, choose layout from model code
read serial number
hdr := read 8 bytes at 0x0190
last, count, end, begin := hdr as four u16le
check last and end inside ring
if begin inside ring:
    total := ring distance from begin to end
             (if begin = end: full ring when count ≠ 0, else empty)
else:
    total := ring size
current := last ; following := end
while total > 0:
    size := ring distance from current to following, counting a full ring when equal
    if size < 4 or size > total: error
    read `size` bytes ending at `following`, walking backwards through the ring
         in blocks of ≤ 120 bytes and wrapping from ring start to ring end
    prev := u16le at record+0 ; next := u16le at record+2
    check prev and next inside ring
    if next ≠ following and next ≠ current: error
    if next = current:
        the dive is incomplete (still being written); skip it, report a data error at the end
    else:
        if dive data[fp .. fp+7] = stored fingerprint: stop
        emit dive data (record from +4)
    total := total - size
    following := current ; current := prev
```

Ring distance from `a` to `b` is `b - a` if `b ≥ a`, else `(ring_end - a) + (b - ring_start)`.

Stopping on age: instead of the fingerprint, a host may compare the dive's start time with the
newest already-downloaded dive and stop at the first dive that is not newer. One source does
this; it also stops when the previous-dive pointer equals the current position or falls inside
the newest dive (wrap-around overlap).

Incremental fingerprint: the 7 date/time bytes at the model's fingerprint offset.

Confidence: **likely**.

## Dive data format

Offsets below are relative to the start of the dive data (record +4).

### Header: common fields

| Offset | Type | Raw unit | Conversion | Meaning |
|--------|------|----------|------------|---------|
| `0x00` | u8 | | | not decoded |
| `0x01` | u32le | | | logbook format ID (selects sub-layouts, see below) |
| `0x09` | u16le | cm | / 100 → m | maximum depth |

### Header: fields that move by model group

| Field | Classic | D4 | HelO2 | i-series | DX |
|-------|---------|----|-------|----------|----|
| Dive time: offset | `0x0B` | `0x0B` | `0x0D` | `0x0D` | `0x0D` |
| Dive time: unit | minutes | seconds | minutes | seconds | seconds |
| Date/time offset | `0x11` | `0x11` | `0x17` | `0x13` | `0x17` |
| Date/time order | h, m, s, year u16le, month, day | same as classic | same as classic | year u16le, month, day, h, m, s | same as i-series |
| Gas mode byte | `0x19` | `0x19` | `0x1F` | `0x1D` | `0x21` |
| Sample interval (s) | `0x18` | `0x18` | `0x1E` | `0x1E` | `0x22` |
| Conservatism | `0x1E` (raw) | `0x1E` (raw) | `0x23` (raw − 2) | `0x21` (raw − 2) | `0x25` (raw − 2) |
| Gas table offset | `0x21` | `0x21` | `0x54` | `0x5F` or `0x67` | `0xC1` or `0xC3` |
| Gas table entries | 3 × 1 byte | 3 × 1 byte | 8 × 6 bytes | 1, 2, 3 or 8 × 6 bytes | 11 × 6 bytes |
| Initial gas index | 0 | 0 | `0x26` | `0x28` or `0x2D` | `0x31` |
| Sample configuration offset | `0x3A` | `0x3B` | table end | table end | table end |

"Table end" = gas table offset + entries × 6.

One source gives surface-interval hours and minutes at `0x1B`/`0x1C` (classic, D4, i-series)
and at date offset + 10/11 (HelO2) or + 8/9 (DX); the reference does not decode them.

#### Logbook format IDs (u32le at `0x01`)

| ID | Seen on | Effect |
|----|---------|--------|
| `0x1871C062` | D4i / D6i, first format, D6i with 2 mixes | gas table at `0x5F`, initial gas at `0x28` |
| `0x1871C063` | D6i, first format, 3 mixes | gas table at `0x5F`, 3 entries, initial gas at `0x28` |
| `0x18724062` | D4i / D6i, second format | gas table at `0x67`, initial gas at `0x2D`; D6i: 5-byte gas-change events |
| `0x18922062` | DX, first format | gas table at `0xC1` |
| `0x18924062` | DX, second format | gas table at `0xC3` |

i-series gas-table entry counts: D4i, Zoop Novo, D4f → 1; D6i and Vyper Novo → 3 with IDs
`0x1871C063`/`0x18724062`, else 2; D9tx → 8 (table at `0x87`, independent of ID).

#### Gas mode byte

| Value | Mode |
|-------|------|
| 0 | air: one mix, 21 % O2 |
| 1 | nitrox |
| 2 | gauge: no mixes |
| 3 | free dive: no mixes |
| 4 | mixed gas (trimix) |
| 5 | closed circuit (DX) |

#### Gas table

- Classic and D4: up to 3 single bytes, each an O2 percentage; a `00` or `FF` ends the list.
- All other groups: 6-byte entries; byte +1 is O2 %, byte +2 is He %; the other four bytes
  are not decoded. All listed entries count, including unused ones.
- DX: the first 3 entries are closed-circuit diluents, the remaining 8 open-circuit gases.
  Initial gas byte at `0x31`: low 7 bits are an index; if bit 7 is **clear** the index is into
  the open-circuit part (add 3), if set it is into the diluent part.

Applies only when the mode is not air, gauge or free dive.

### Sample configuration and profile

At the sample configuration offset `c`:

```
c + 0        u8   n, number of sampled channels (1..3)
c + 1        u8   not decoded
c + 2 + 3k   3 bytes per channel k:
               +0  channel type
               +1  period: sampled on every tick whose index is a multiple of this (0 = never)
               +2  flags: bits 2..4 select a divisor from {1, 2, 4, 5, 10, 50, 100, 1000}
p = c + 2 + 3n   profile block
```

Channel types:

| Type | Size | Meaning | Conversion |
|------|------|---------|------------|
| `0x64` | u16le | depth | raw / divisor → m |
| `0x68` | u16le | tank pressure; `FFFF` = no reading | raw / divisor → bar |
| `0x74` | i8 | temperature | raw / divisor → °C |

Any other type is an error (the record cannot be parsed).

HelO2 only: if the three bytes at `p` are not `01 00 00`, an extra 12-byte block precedes the
profile; add 12 to `p`.

```
p + 0..2   01 00 00        (looks like an initial event-block terminator, see below)
p + 3      u16le           tick index at which the first event block appears ("marker")
p + 5 ...  ticks
```

Ticks: tick `t` (starting at 0) is at time `t × sample interval` seconds. For each tick, for
each channel in configuration order whose period divides `t`, read that channel's value. After
the channel values, if `t + 1` equals the current marker, an **event block** follows.

### Event block

A sequence of events, terminated by event `01`:

| Code | Length after code | Fields | Meaning |
|------|-------------------|--------|---------|
| `01` | 4 | u16le current, u16le delta | end of block. `current` MUST equal the marker; the new marker = marker + delta |
| `02` | 2 | state, seconds | surfaced |
| `03` | 2 | type, seconds | warning / state change; bit 7 of `type` set = condition ended, clear = began |
| `04` | 4 | number, seconds, u16le heading | heading `FFFF`: bookmark; otherwise compass bearing, heading / 2 = degrees |
| `05` | 2 | O2 %, seconds | gas change to the listed mix with that O2 (He 0) |
| `06` | 4 or 5 | type, He %, O2 %, [ppO2], seconds | gas change by index; 5 bytes on DX, Vyper Novo, and D6i with format ID `0x18724062` |

`seconds` is the offset of the event within the current tick.

Event `06` details: low nibble of `type` is the gas index; bit 7 clear means an open-circuit gas
(on the DX add the 3 diluent slots), bit 7 set means a diluent, in which case `ppO2` / 10 is the
new set point in bar. The He and O2 bytes must match the gas table entry.

Event `03` type codes (low 7 bits):

| Code | Meaning |
|------|---------|
| `00` | voluntary safety stop |
| `01` | mandatory safety stop |
| `02` | deep stop |
| `03` | decompression |
| `04` | ascent rate warning |
| `05` | ceiling broken |
| `06` | mandatory safety stop ceiling error |
| `07` | below deco floor |
| `08` | dive time alarm |
| `09` | depth alarm |
| `0A` | oxygen limit fraction 80 % |
| `0B` | oxygen limit fraction 100 % |
| `0C` | ppO2 warning |
| `0D` | air time warning |
| `0E` | RGBM warning |
| `0F` | ppO2 high |
| `10` | ppO2 low |
| `11` | tissue level warning |
| `12` | tissue calculation overflow |
| `13` | deep stop |
| `14` | mandatory safety stop |

Codes `00`–`03`, `13`, `14` are states (stop in progress / deco in progress) rather than
alarms; a host can derive the current deco state from them (deep stop > deco > safety stop >
no-deco).

An unknown event code has unknown length; the reference continues after the code byte, which
may desynchronise. Treat it as a parse error unless more is known.

Tank start/end pressure is not in the header; derive them from the first and last (or highest
and lowest) valid pressure samples.

### Date and time

Local device time, second resolution, full 16-bit year (little-endian), no time zone.

Confidence: **likely** for offsets that both sources give (most of the header, the channel
types, event codes `01`, `03`, `05`); **uncertain** for event `02` and `04` (one source reads
`02` as surfaced and `04` as bookmark/heading, the other leaves them undecoded), for the HelO2
12-byte block, and for the 5-byte `06` rule.

## Model-specific variations

- V2 link (Vyper 2, Cobra 2/3, Vyper Air, HelO2): no echo, RTS asserted while sending.
- D9 link (D9, D6, D4, i-series): echo, RTS cleared while sending; i-series at 115200 baud.
- D4: dive time in seconds and sample configuration one byte later than classic.
- HelO2: trimix gas table, optional extra 12-byte block before the profile.
- i-series and DX: 64 KiB memory, serial at `0x0024`, year-first date.
- DX: closed-circuit diluents and set points.

## Open questions

1. Read-reply bytes 1–2: the reference requires `00 (N+3)`; the other source describes them as
   `00 03`. The reference's rule is consistent with the version reply (`00 04` for 4 data bytes).
2. Meaning of the ring header word at `0x0192` (a dive count?) and of dive-data byte `0x00`.
3. The 12-byte HelO2 block and the four undecoded bytes of each 6-byte gas entry.
4. Exact required inter-command delays and DTR settle time; the sources differ by up to 10×.
5. Lengths of unknown event codes.
6. Whether the Zoop Novo's two model codes (`0x1E`, `0x1F`) differ in anything else.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
