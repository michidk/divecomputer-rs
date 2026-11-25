# Dive Rite NiTek Q

The Dive Rite NiTek Q uses a serial protocol of single ASCII command letters. A handshake
returns a 32-byte identity block. The whole 32 KiB memory is then read as 128 blocks of
256 bytes, each followed by 2 checksum bytes whose algorithm is unknown. Memory holds a
10-entry logbook (newest first) with pointers into a profile ring. The profile is a
stream of tagged records and plain depth samples.

The NiTek Trio is a different family; see `zeagle-n2ition3.md`.

No public vendor documentation is known. The only source is libdivecomputer, so the best
confidence here is **likely**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Dive Rite | NiTek Q | 0 | serial (interface cable / USB-serial) | OC trimix and CCR modes |

There are no USB IDs, Bluetooth names or IrDA nicknames of its own.

Confidence: **likely**.

## Transport parameters

- 9600 baud, 8N1, no flow control.
- Read timeout 1000 ms.
- After opening: wait 100 ms, then flush both directions.

Confidence: **likely**.

## Session

```
H→D  48                       'H'  handshake
D→H  <32 bytes>               identity block
...
H→D  44                       'D'  disconnect (on close)
```

Identity block: a big-endian u32 at offset 0x0A is the serial number. The other bytes are
not decoded.

Confidence: **likely**.

## Commands

| Byte | ASCII | Meaning | Response |
|------|-------|---------|----------|
| `48` | H | handshake / identify | 32 bytes, no checksum |
| `55` | U | upload request | 256 bytes + 2 checksum bytes. Content not used; the reference keeps it but ignores it |
| `52` | R | reset the block pointer to the start of memory | none |
| `42` | B | send the next 256-byte block | 256 bytes + 2 checksum bytes |
| `44` | D | disconnect | none |
| `3E` | > | defined in the reference as a keep-alive, never sent | unknown |

Confidence: **likely**; the `U` and `3E` semantics are **uncertain**.

## Framing and integrity

Blocks are raw data followed by 2 bytes that look like a checksum. The algorithm is
unknown and the reference does not check it. An implementation should record these
bytes. A future version of this spec may define the algorithm once dumps are available.

## Memory layout

32768 bytes, read as blocks 0..127 after `R`.

| Address | Size | Content |
|---------|------|---------|
| 0x0320 | 10 × 6 | logbook: dive start date/time per entry; entry 0 = newest |
| 0x0384 | 10 × 2 | u16be profile start address per logbook entry |
| 0x03E6 | 2 | u16be end-of-profile pointer (where the next profile byte will go) |
| 0x03E8–0x7FFF | 31768 | profile ring buffer |

When a dive is added, the device shifts the logbook and address tables by one entry so
that index 0 is always the newest. The profile ring is appended in place and wraps from
0x7FFF to 0x03E8. At most 10 dives are reachable.

Confidence: **likely**.

## Download procedure

```
H→D 'H'; read 32 bytes (identity)
H→D 'U'; read 256 + 2 bytes (ignored)
H→D 'R'
repeat 128 times: H→D 'B'; read 256 + 2 bytes; keep the 256 data bytes
mem = concatenated blocks (32768 bytes)

end = u16be(mem[0x03E6])            # must be within 0x03E8..0x7FFF
for i in 0..9:
    log = mem[0x0320 + 6i .. +6]
    if log is all zero: stop
    start = u16be(mem[0x0384 + 2i])  # must be within the ring
    if log == fingerprint: stop
    profile = ring bytes [start, end), wrapping from 0x8000 back to 0x03E8
    emit  log ++ profile             # newest first
    end = start
H→D 'D' on close
```

Fingerprint: the 6 logbook bytes (date/time).

Confidence: **likely**.

## Dive data format

The dive blob is the 6-byte logbook entry followed by the profile bytes.

### Header (logbook entry)

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u8 | year − 2000 |
| 1 | u8 | month |
| 2 | u8 | day |
| 3 | u8 | hour |
| 4 | u8 | minute |
| 5 | u8 | second |

Binary values; no time zone.

### Profile records

The profile is read sequentially. A byte `FF` starts a tagged record, and the next byte is
its type:

| Type | Total length | Content |
|------|--------------|---------|
| `01` settings | 27 bytes | offset 0x10 bit 2: 1 = metric, 0 = imperial; offset 0x11: sample interval, s. Other bytes not decoded |
| `02` OC samples | 2 bytes | the samples that follow are open-circuit samples |
| `03` CC samples | 2 bytes | the samples that follow are closed-circuit samples |
| `04` gas change | 7 bytes | offset 5: O₂ %, offset 6: He %; offsets 2–4 not decoded |

Any other type means the data is corrupt. Offsets are counted from the `FF`.

Outside tagged records, after a `02` or `03` tag:

| Mode | Sample layout |
|------|---------------|
| OC (`02`) | u16be depth |
| CC (`03`) | u16be depth, then u8 ppO₂ in 0.01 bar |

Depth unit: 0.1 m (metric) or 0.1 ft (imperial, × 0.3048 for metres). Each sample advances
the clock by the interval from the settings record. A sample before any settings record,
or before any `02`/`03` tag, is invalid.

Tags stay unambiguous because a depth sample cannot begin with `FF`: its value would have
to be at least 0xFF00 tenths (inferred). The ppO₂ byte of a CC sample is consumed as
part of the sample and is never read as a tag.

Gas mixes: build the list from the distinct (O₂, He) pairs in gas change records; the
reference allows up to 7. The dive is CCR if any `03` sample appears, otherwise OC.

Dive time = number of samples × interval. Max depth = the largest depth sample.

Example: `FF 04 00 00 00 20 00` = switch to EAN32. In CC mode, `01 2C 82` = 30.0 m at
ppO₂ 1.30 bar.

Confidence: **likely** for the structure; **uncertain** for the meaning of the remaining
settings bytes.

## Model-specific variations

None known.

## Open questions

1. The block checksum algorithm (2 bytes after each 256-byte block and after the `U`
   reply).
2. The purpose of the `U` request and its 256-byte answer. Possibly the settings or a
   header page.
3. The `3E` keep-alive: is it needed during long pauses?
4. The remaining bytes of the identity block (model or firmware version?).

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
