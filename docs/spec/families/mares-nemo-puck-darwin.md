# Mares Nemo, Puck and Darwin (ASCII-hex memory protocol)

This document covers the older Mares computers that expose a small (16 KiB or 32 KiB)
memory through a serial link, usually an infrared or USB-serial interface cable. Three
variants exist:

| Variant | Devices | How memory is obtained |
|---------|---------|------------------------|
| **N** (Nemo push) | Nemo, Nemo Steel, Nemo Titanium, Nemo Excel, Nemo Apneist | The device sends its whole memory on its own after the diver selects PC mode. The host only listens. |
| **P** (Puck poll) | Puck, Puck Air, Nemo Air, Nemo Wide, Puck Wrist | The host polls memory with ASCII-hex read commands at 38400 baud. |
| **D** (Darwin poll) | Darwin, M1, M2, Darwin Air, Airlab | The same ASCII-hex read command at 9600 baud through an interface that echoes every command. |

Variants N and P store dives in the same "length-prefixed profile ring" format and share
one dive parser. Variant D uses a separate logbook table plus a profile ring.

Later Mares computers (Icon HD and newer) use a binary protocol, see
[mares-iconhd.md](mares-iconhd.md) and [mares-genius.md](mares-genius.md).

## Devices

Model codes are the byte at memory offset `0x01` (variants N and P). Variant D devices do
not report a model; the host has to know which layout to use.

| Vendor | Model | Model code | Variant | Transports | Notes |
|--------|-------|------------|---------|------------|-------|
| Mares | Nemo | 0 | N | serial (IR cable) | also sold as Nemo Steel / Nemo Titanium |
| Mares | Nemo Wide | 1 | P | serial | memory layout "Nemo Wide" |
| Mares | Nemo Air | 4 | P | serial | 32 KiB layout, tank trailer (12 bytes) |
| Mares | Puck | 7 | P | serial | |
| Mares | Nemo Excel | 17 (0x11) | N | serial | |
| Mares | Nemo Apneist | 18 (0x12) | N | serial | short profile ring, 1 s freedive profile |
| Mares | Puck Air | 19 (0x13) | P | serial; see note | 32 KiB layout, tank trailer (7 bytes) |
| Mares | Puck Wrist | ? | P | serial | named only by one source; layout assumed equal to Puck |
| Mares | Darwin, M1, M2 | (host-selected 0) | D | serial (IR cable) | 52-byte logbook entries |
| Mares | Darwin Air, Airlab | (host-selected 1) | D | serial (IR cable) | 60-byte logbook entries, 3-byte samples |

Identification: no USB VID:PID belongs to the dive computer itself. The interface cables
are generic USB-serial bridges (FTDI, Silicon Labs CP210x `10C4:EA60`, Prolific PL2303
`067B:2303`), see `../transports/`. One source also lists Puck Air as reachable over BLE
through the Mares "BlueLink Pro" adapter (advertised name prefix `Mares bluelink pro`),
which tunnels the serial byte stream; see [mares-iconhd.md](mares-iconhd.md#ble) for that
adapter's GATT layout.

Confidence: **likely** (model codes and variants from one reference implementation; the
Puck Wrist and BLE Puck Air entries from a second source only, **uncertain**).

## Transport parameters

| Variant | Baud | Framing | DTR | RTS | Read timeout | Other |
|---------|------|---------|-----|-----|--------------|-------|
| N | 9600 | 8N1, no flow control | set (1) | set (1) | 1000 ms per read | host waits indefinitely (polling every 100 ms) for the first byte |
| P | 38400 | 8N1, no flow control | cleared (0) | cleared (0) | 1000 ms | |
| D | 9600 | 8N1, no flow control | set (1) | set (1) | 1000 ms | wait 100 ms after opening, then purge; **50 ms pause before every command**; interface echoes each command |

After configuring the line, the host discards any buffered input and output.

A second source configures variant D with a 60 ms read timeout (600 ms for Airlab) and
leaves DTR/RTS untouched, and drives variant-P devices without touching DTR/RTS either.
The values in the table are the conservative choice. Multi-brand interface cables may
need their own mode setup before the IR link works; that is a cable property and is not
described here.

Confidence: **likely**.

## Session

There is no handshake, wake-up or identity command in any variant.

- **N:** the user puts the computer into PC/download mode; the computer then starts
  transmitting. The host first waits for a preamble of **20 consecutive `EE` bytes** (any
  other byte restarts the count) and then receives the memory image (see
  [Download procedure](#download-procedure)).
- **P and D:** the host sends read commands right away. Model detection for variant P
  is a read of the first 32 bytes and a look at offset `0x01`.

Confidence: **likely**.

## Commands

Variants P and D use one command.

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Read memory | `<` + hex(`51` `aL` `aH` `n`) + hex(cs) + `>` (12 bytes) | `aL`/`aH`: 16-bit address, **low byte first**; `n`: byte count, 1..32 (`0x20`) | `<` + hex(n data bytes) + hex(cs) + `>` = 2·n + 4 bytes | ASCII, uppercase hex digits |

`hex(x)` means each byte written as two uppercase ASCII hex digits, most significant
nibble first. On variant D the interface first echoes the 12 command bytes; the host reads
and discards them (comparing them with what was sent is optional) before reading the
response.

Variant N has no commands: the device pushes data.

Confidence: **likely**.

## Framing and integrity

### ASCII read frame (variants P and D)

```
H→D  3C <8 hex digits: "51" aL aH n> <2 hex digits: cs> 3E
D→H  3C <2·n hex digits: data> <2 hex digits: cs> 3E
```

Checksum `cs` is a [sum8](../common/checksums.md#sum8) (8-bit sum modulo 256, initial 0)
of the **ASCII digit bytes** between `<` and the checksum digits, i.e. the sum of the
character codes, not of the decoded binary bytes. The checksum itself is sent as two hex
digits. The response checksum covers the `2·n` data digits.

A response is rejected if its first byte is not `<` (`0x3C`), its last byte is not `>`
(`0x3E`) or its checksum does not match. On a timeout or a rejected response the host
waits about 100 ms, discards pending input and resends the same command, up to 4
retries. (A second source uses 5 attempts with 20 ms pauses.)

Worked example, read 32 bytes at `0x0100`:

```
digits  "51" "00" "01" "20"  -> ASCII 35 31 30 30 30 31 32 30
sum8    0x35+0x31+0x30+0x30+0x30+0x31+0x32+0x30 = 0x189 -> 0x89 -> "89"
H→D     3C 35 31 30 30 30 31 32 30 38 39 3E        "<5100012089>"
```

More command vectors: address `0x0000` → `<5100002088>`; address `0x0B30` →
`<51300B209D>`. A response carrying the 32 bytes `00 01 … 1F` is 68 bytes long:
`<000102…1E1F54>`: the 64 data digits sum to `0x0D54`, so the checksum digits are `54`.

### Pushed packets (variant N)

After the 20-byte `EE` preamble the device sends 512 packets of 66 bytes each:

```
<32 data bytes> <sum8 of those 32 bytes> <the same 32 data bytes again> <sum8>
```

Data is raw binary (not ASCII hex). The host accepts the first copy if its sum8
matches; otherwise the second copy if its sum8 matches. If both match but the copies
differ, the transfer is corrupt. If neither matches, the transfer has failed (there is no
way to ask for a resend). 512 × 32 = 16384 bytes = the whole memory.

Confidence: **likely** (checksum vectors computed; framing from one implementation, the
P/D command frame agreed by a second source).

## Memory layout

All variants: 16-bit addresses. Multi-byte values in the **N/P** areas are little-endian;
values in the **D** logbook are big-endian (samples remain little-endian).

### Variants N and P

| Address | Size | Meaning |
|---------|------|---------|
| `0x0001` | u8 | model code |
| `0x0008` | u16be | serial number (one source: u24be at `0x0007`) |
| `0x006B` | u16le | end-of-profile pointer: address just past the newest dive |
| `0x0070` … | | start of the profile ring |

Ring geometry per layout:

| Layout | Used by (model codes) | Memory | Profile ring | Freedive profile area |
|--------|-----------------------|--------|--------------|----------------------|
| Nemo | 0, 17 (and unknown models on N) | `0x4000` | `0x0070`–`0x3400` | `0x3400`–`0x4000` |
| Nemo Apneist | 18 | `0x4000` | `0x0070`–`0x0800` | `0x0800`–`0x4000` |
| Nemo Wide | 1 | `0x4000` | `0x0070`–`0x3400` | `0x3400`–`0x4000` |
| Nemo Air / Puck Air | 4, 19 | `0x8000` | `0x0070`–`0x8000` | none |
| Puck | 7 (and unknown models on P) | `0x4000` | `0x0070`–`0x4000` | none |

Ranges are half-open (`[begin, end)`). The ring wraps from `end` back to `begin`. Unused
memory reads as `FF`.

### Variant D

| Address | Size | Meaning |
|---------|------|---------|
| `0x0008` | u16be | serial number |
| `0x008A` | u16be | end-of-profile pointer (address just past the newest profile) |
| `0x008C` | u8 | index (0..49) of the newest logbook slot |
| `0x0100` | 50 × entry | logbook ring: entry `i` at `0x0100 + i · E` |

| Layout | Entry size `E` | Profile ring | Sample size |
|--------|----------------|--------------|-------------|
| Darwin / M1 / M2 | 52 | `0x0B30`–`0x4000` | 2 |
| Darwin Air / Airlab | 60 | `0x0CC0`–`0x3FFF` | 3 |

A second source describes the same area differently: a dive count u16be at `0x0080`
(capped at 50), the newest slot at `0x008C`, and a per-slot profile pointer u16be at
`0x008D + 2·slot`. Both descriptions can be true at once; see open questions.

Confidence: **likely** for the N/P layout; **uncertain** for the D header fields, where
the sources differ.

## Download procedure

### Variant N

1. Wait for data, then for the 20-byte `EE` preamble.
2. Receive 512 packets as above; concatenate the 32-byte payloads into a 16 KiB image.
3. Select the layout by the model byte at `0x0001` and extract dives as for variant P.

### Variant P

1. Read 32 bytes at `0x0000`; the model byte at `0x0001` selects the layout.
2. Read the whole memory (`0` to memory size) in 32-byte commands.
3. Extract dives (below).

There is no incremental read: the whole memory is always transferred. Incremental
behaviour comes from stopping the dive walk at the first known fingerprint.

### Dive extraction for N and P

Each dive in the profile ring is stored as

```
[u16le total length L] [samples] [header] [optional trailer]
```

and ends exactly where the next (newer) dive begins. The newest dive ends at the
end-of-profile pointer. The host walks backwards from that pointer:

```
linearise the ring so that the byte just before the pointer is the last byte
pos = ring size
loop while pos >= 3:
    trailer = 0
    if the 3 bytes before pos are AA BB CC:
        trailer = 7 if model is Puck Air (19) else 12
    stop if pos < trailer + 3
    mode = byte at pos - trailer - 1
    stop if mode == FF            (never-written memory)
    header = 53, sample = 2
    if trailer: sample = 3 if Puck Air else 5
    if mode == freedive-mode(model): header = 28, sample = 6
    n = u16le at pos - trailer - 3
    L = 2 + n * sample + header + trailer
    stop if pos < L               (dive partly overwritten)
    pos = pos - L
    the u16le at pos MUST equal L, otherwise the data is corrupt
    emit bytes [pos, pos + L) as one dive
```

`freedive-mode(model)` is 3 for Nemo Wide, Nemo Air, Puck and Puck Air, and 2 for all
others (Nemo, Nemo Excel, Nemo Apneist).

**Freedive profiles (Nemo, Nemo Excel, Nemo Apneist):** only the newest freedive session
has a detailed profile, stored in the freedive profile area (e.g. `0x3400`–`0x4000`). It is
a sequence of u16le depth samples in 0.1 m; each individual freedive of the session ends
with a `0000` sample. While extracting the newest freedive session (the first freedive met
on the backwards walk), count terminating zeros from the start of that area until the
count equals the session's dive count `n`; if the count cannot be reached, the data is
inconsistent. The covered bytes are appended after the session record so a parser sees
`L` bytes of record followed by the raw profile area.

**Fingerprint:** the 5 bytes at `pos + L - trailer - 8` (the date/time bytes of the
header, see below). Stop the walk at the first dive whose fingerprint matches the last
known dive.

### Variant D

1. Read the whole memory (`0x4000` bytes) in 32-byte commands.
2. Read the end-of-profile pointer at `0x008A` and the newest slot at `0x008C`; both must
   be in range.
3. For `i` = 0..49, slot = (newest − i) mod 50, entry at `0x0100 + slot · E`:
   - `n` = u16be at entry + 6. Stop if `n` = `FFFF`.
   - profile length = `n` × sample size; stop if it exceeds the ring bytes not yet
     consumed by newer dives.
   - The profile occupies the `length` bytes ending at the current profile cursor
     (initially the end-of-profile pointer), wrapping from ring begin to ring end.
     Move the cursor back by `length`.
   - The dive record handed to the parser is `entry (E bytes) || profile`.
   - Fingerprint: the first 6 bytes of the entry (date/time).

Confidence: **likely** for N and P; **likely** for D's algorithm as described, with the
pointer location **uncertain** (see Memory layout).

## Dive data format

### Variants N and P (one record per dive)

Offsets below are relative to `q`, the end of the header, i.e. `q = L − trailer`. The
header occupies `[q − 53, q)` (or `[q − 28, q)` for freedive sessions).

#### Header, scuba/gauge modes (53 bytes)

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| q−1 | u8 | | mode: 0 air, 1 nitrox, 2 freedive (Nemo family) or gauge (Puck family), 3 gauge (Nemo family) or freedive (Puck family) |
| q−3 | u16le | count | number of samples `n` |
| q−4 | u8 | min | minute |
| q−5 | u8 | h | hour |
| q−6 | u8 | | day |
| q−7 | u8 | | month (1..12) |
| q−8 | u8 | | year − 2000 |
| q−10 | u16le | 0.1 m | maximum depth; m = raw / 10 |
| q−11 | i8 | °C | minimum (bottom) temperature |
| q−12 | u8 | | water type flag (second source: 0 → one type, non-zero → the other; mapping unknown) |
| q−42 | u8 | 0.01 bar | maximum ppO2 (second source only) |
| q−43 | u8 | % | O2 percentage, valid in nitrox mode |
| q−44 | u8 | | non-zero = dive needed decompression (second source only) |

Dive time = `n` × 20 s. Air mode implies 21 % O2. Gauge and freedive modes have no gas
mix.

#### Tank trailer

| Trailer | Models | Layout (relative to `q`) |
|---------|--------|--------------------------|
| 12 bytes | Nemo Air | q+0 u16le tank volume; q+2 u16le working pressure (psi), `FFFF` = metric tank; q+4 u16le start pressure, 0.01 bar; q+6 u16le end pressure, 0.01 bar; q+8 u8 (one source: RMV); q+9 `AA BB CC` |
| 7 bytes | Puck Air | q+0 u16le start pressure, bar; q+2 u16le end pressure, bar; q+4 `AA BB CC` |

Tank volume: if the working pressure is `FFFF` the volume is metric in 0.1 L
(litres = raw / 10). Otherwise it is the imperial gas capacity at the working pressure;
water volume in litres = `raw × 28.316846592 / (wp × 6.894757293 / 101.325)` (cu ft of
free gas → litres of water capacity), and working pressure in bar = `wp × 0.06894757`.
A working pressure of 0 with an imperial tank is invalid. One source applies a ÷10 to the
imperial raw value as well; see open questions.

#### Samples, scuba/gauge modes

Sample `i` starts at `2 + i · sample size` and is taken every **20 s** (first sample at
20 s).

| Bytes | Type | Meaning |
|-------|------|---------|
| +0 | u16le | bits 0–10 depth in 0.1 m; bit 12 deco stop required (else in no-deco); bit 13 deco ceiling violated; bits 14–15 ascent-rate warning level (0 = none) |
| +2 | u8 | sample size 3 (Puck Air): tank pressure in bar. Sample size 5 (Nemo Air): pressure drop in bar since the previous pressure point; see below |
| +3..+4 | | sample size 5 only: unknown |

Nemo Air pressure reconstruction: start from the trailer start pressure; at every third
sample (samples 3, 6, 9, … counting from 1) subtract the drop byte (bar) and report the
result. One source instead subtracts one third of the byte at every sample; see open
questions.

#### Freedive session (28-byte header)

A freedive session record stores one 6-byte summary per freedive instead of a profile.

| Offset | Type | Meaning |
|--------|------|---------|
| q−1 | u8 | mode (the freedive value) |
| q−3 | u16le | number of freedives `n` |
| q−8 … q−4 | 5 × u8 | year − 2000, month, day, hour, minute (as above) |
| q−10 | u16le | maximum depth of the session, 0.1 m |
| q−11 | i8 | minimum temperature, °C |

Summary `i` at `2 + 6 i`: u16le maximum depth (0.1 m), u8 dive-time seconds, u8 dive-time
minutes, u8 surface-time seconds, u8 surface-time minutes. Total session dive time is the
sum of all dive times.

If the profile area was appended (newest session only), each freedive's profile follows in
order: u16le depth samples, 0.1 m, every 4 s (every 1 s on Nemo Apneist), ending with a
`0000` sample. The number of samples is `ceil(dive time / interval)`; the last interval
may be shorter. Time reconstruction: add the surface time before the dive, then step by
the interval.

### Variant D (Darwin family)

Record = logbook entry (big-endian) followed by the profile.

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x00 | u16be | | year (full, e.g. 2008) |
| 0x02 | u8 | | month (1..12) |
| 0x03 | u8 | | day |
| 0x04 | u8 | | hour |
| 0x05 | u8 | | minute |
| 0x06 | u16be | count | number of samples `n`; dive time = `n` × 20 s |
| 0x08 | u16be | 0.1 m | maximum depth |
| 0x0A | i8 | °C | minimum temperature |
| 0x0B | u8 | | high nibble: water type (second source; mapping unknown) |
| 0x0C | u8 | | bits 0–1 mode: 0 air, 1 gauge, 2 nitrox |
| 0x0E | u8 | % | O2 in nitrox mode (21 % otherwise) |
| 0x0F | u8 | 0.1 bar | maximum ppO2 (second source only) |

Airlab / Darwin Air additions (60-byte entry):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x12 | u16be | tank volume, 0.1 L (one source reads only the low byte at 0x13) |
| 0x14 | u16be | working pressure, psi; `FFFF` = none (second source only) |
| 0x17 | u16be | start pressure, bar |
| 0x19 | u16be | end pressure, bar |
| 0x3A | u8 | non-zero = decompression dive (second source only) |

Samples follow the entry, every 20 s:

| Bytes | Type | Meaning |
|-------|------|---------|
| +0 | u16le | bits 0–10 depth, 0.1 m; bit 11 deco stop; bit 12 ceiling violation; bits 13–15 ascent-rate warning level |
| +2 | u8 | Airlab only: pressure drop, bar |

Airlab pressure: start from the start pressure; at samples 1, 4, 7, … (1-based) subtract
the drop byte and report.

### Date and time

All variants store local time with minute resolution and no time zone; seconds are 0.
N/P: year offset from 2000. D: full year.

Confidence: **likely** for fields present in the reference implementation; fields marked
"second source only" are **uncertain**.

## Model-specific variations

- Nemo Apneist: short profile ring (`0x0800`), freedive profile interval 1 s.
- Nemo Wide, Nemo Air, Puck, Puck Air: the meaning of mode values 2 and 3 is swapped
  relative to the Nemo family (3 = freedive layout).
- Nemo Air and Puck Air dives carry the `AA BB CC` trailer marker and larger samples.

## Open questions

1. Variant D header: is the profile end pointer at `0x008A` (u16be), or are per-slot
   pointers at `0x008D + 2·slot` authoritative? Is `0x0080` a dive counter?
2. Serial number width/location: u16be at `0x0008` or u24be at `0x0007`?
3. Nemo Air pressure samples: drop applied every third sample, or one third of the byte at
   every sample?
4. Nemo Air imperial tank volume: raw in cu ft or 0.1 cu ft?
5. Water-type and max-ppO2 bytes are reported by one source only; their encoding is not
   resolved.
6. Variant D timing: 1000 ms timeout with a 50 ms pre-command pause, or a short (60 ms)
   timeout without pause? DTR/RTS required or not?
7. Is Puck Air reachable through the BlueLink Pro BLE adapter with this protocol (one
   source only)? Is Puck Wrist identical to Puck?
8. Variant N: does a host-polled ASCII read (as in variant P) also work on the Nemo? One
   source drives a device labelled "Nemo" with polled reads at 9600 baud.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used
  as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1),
  <https://github.com/subsurface/libdc>, used as a factual reference; no code copied.
