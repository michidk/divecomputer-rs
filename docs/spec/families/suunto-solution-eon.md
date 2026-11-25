# Suunto Solution and Eon

The oldest Suunto dive computers that can be downloaded: the Solution (circa 1990s, air only)
and the Eon generation (Eon, Solution Alpha, Solution Nitrox / Vario). Both are read over a
wired serial interface at 1200 baud and return their whole memory in one transfer. There are
no addressed reads: the host pulls the complete memory image and then carves dives out of a
profile ring buffer inside it.

The two families use different wake-up sequences and memory sizes, but share the same idea
of a delta-encoded depth profile in feet terminated by marker bytes. The Eon-style dive record
is also reused by the Suunto Spyder (see [suunto-vyper.md](suunto-vyper.md)).

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Suunto | Solution | none reported | serial | 256-byte memory, handshake per byte |
| Suunto | Eon | none reported | serial | 2304-byte memory, single bulk reply |
| Suunto | Solution Alpha | none reported | serial | same protocol as Eon |
| Suunto | Solution Nitrox (also sold as Vario) | none reported | serial | Eon protocol; nitrox flag in dive header |

None of these devices report a model code; the host must know which family it is talking to.

Identification: the devices are attached through Suunto's serial interface cable (RS-232 or a
USB-serial adapter). No USB IDs, Bluetooth names or IrDA nicknames apply. Generic serial
handling is described in [../transports/](../transports/).

Confidence: **likely** (single reference implementation).

## Transport parameters

| Parameter | Value |
|-----------|-------|
| Baud rate | 1200 |
| Framing | 8 data bits, no parity, **2 stop bits** (8N2) |
| Flow control | none |
| RTS | cleared (de-asserted) when the port is opened |
| DTR | Solution: asserted before the transfer starts. Eon: not changed by the reference |
| Read timeout | 1000 ms per read |

Confidence: **likely**.

## Session

### Solution

The Solution transfer is a lock-step byte dialogue. Every step is host-driven; the device
echoes a fixed byte or a triplet before each data byte.

```
H→D  FF
D→H  3F
H→D  4D 01 01
repeat for i = 1 .. 255:
    D→H  01 <i> <value>        announce address i and its value
    H→D  <i>                   acknowledge the address
    D→H  <value>               value again
    H→D  0D                    carriage return, request next
D→H  02 00 80                  end of memory
H→D  80
D→H  80
H→D  20
D→H  3F                        idle prompt again
```

Memory address 0 is never transmitted; treat it as `00`. The reference implementation only
warns (does not abort) when the fixed bytes differ from the expected values, and when the two
copies of a value disagree.

### Eon generation

```
H→D  50                        ASCII 'P'
D→H  <2304 bytes memory> <sum8>
```

The reply is 2305 bytes. The device sends it as a continuous stream; the host should keep
reading until all 2305 bytes have arrived (read in whatever chunk size the port delivers).

Two optional configuration writes exist; neither produces a reply in the reference:

| Purpose | H→D bytes |
|---------|-----------|
| Set the owner name / personal text | `4E` ('N') followed by exactly 20 bytes of text, zero-padded |
| Set the profile sample interval | `54` ('T') followed by one byte: the interval in seconds |

Confidence: **likely**.

## Commands

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Solution: start dump | `FF`, then `4D 01 01` | none | see Session | byte-by-byte dialogue |
| Eon: dump memory | `50` | none | 2304 data bytes + 1 checksum | |
| Eon: write name | `4E <20 bytes>` | text | none | configuration only |
| Eon: write sample interval | `54 <s>` | seconds | none | configuration only |

## Framing and integrity

The Solution transfer has no checksum; integrity relies on the double transmission of each
value.

The Eon reply ends with an 8-bit additive checksum: the low 8 bits of the sum of the 2304
memory bytes (initial value 0, no final XOR). See
[../common/checksums.md#sum8](../common/checksums.md#sum8). A mismatch MUST be treated as a
failed download.

Worked example of the sum: memory bytes `00 01 02 … 0F` followed by 2288 zero bytes give a
checksum byte of `78`.

Confidence: **likely**.

## Memory layout

### Solution (256 bytes)

| Address | Contents |
|---------|----------|
| `0x00` | not transmitted |
| `0x18` | u8: address of the end-of-profile marker inside the ring |
| `0x1D..0x1F` | serial number, 3 × `bcd8`, most significant first (6 decimal digits) |
| `0x20..0xFF` | profile ring buffer (224 bytes) |

### Eon generation (2304 bytes = `0x900`)

| Address | Contents |
|---------|----------|
| `0x000..0x0FF` | settings and device data |
| `0x0F4..0x0F6` | serial number, 3 × `bcd8` (6 decimal digits) |
| `0x100..0x8FF` | profile ring buffer (2048 bytes) |

There is no explicit end-of-profile pointer; the host scans the ring from `0x100` upwards for
the first `82` byte.

### Ring buffer markers

| Byte | Meaning |
|------|---------|
| `82` | end-of-profile: the write position. Exactly one is expected in the ring |
| `80` | end of one dive's samples |

Confidence: **likely**.

## Download procedure

### Eon: walking the ring

Dives are stored in forward order. The newest dive ends just before the `82` marker. Each
dive's samples end with `80` and are followed by two trailer bytes (temperature, end
pressure), so the first byte of a dive sits three bytes after the previous dive's `80`.

```
eop := first address a in [0x100, 0x900) with mem[a] = 0x82, else error
prev := eop
cur  := eop
repeat at most 2048 times:
    cur := cur - 1, wrapping from 0x100 to 0x8FF
    if mem[cur] = 0x82: stop (whole ring visited)
    if mem[cur - 3 (wrapping)] = 0x80:
        dive := bytes from cur up to (not including) prev, wrapping at 0x900 -> 0x100
        emit dive (newest first)
        prev := cur
end: the walk must terminate on the 0x82 marker, otherwise the image is corrupt
```

Incremental download: the 5 bytes at dive offset 6 (the start date and time) identify a dive.
Stop when they equal the stored fingerprint of the newest dive from the previous download.

### Solution: walking the ring

The Solution stores its profile **backwards**. Starting at the address held in `0x18`
(which must point at an `82` byte inside `0x20..0xFF`), walk **forwards** through the ring,
wrapping from `0xFF` to `0x20`, collecting bytes into a buffer that is filled from its end so
that the bytes come out reversed (i.e. in chronological order). A dive boundary is reached
when the byte two positions ahead of the current one (with wrap-around) is `80`; at that point
the collected bytes since the previous boundary form one dive. Stop at the next `82`.

There is no fingerprint for the Solution; the host has no date to compare.

Confidence: **likely**. The Solution walk was transcribed from a single implementation and has
not been checked against a dump; see open questions.

## Dive data format

All depths are in **feet**, stored as signed 8-bit deltas relative to the previous sample;
convert with 1 ft = 0.3048 m. The running depth starts at 0.

### Solution dive

| Offset | Type | Meaning |
|--------|------|---------|
| `0x00..0x02` | 3 bytes | not decoded (no date is known to be stored) |
| `0x03..` | samples | until the `80` byte |
| marker + 1 | u8 | extra minutes to add to the dive time |

Sample bytes, one per 3-minute interval:

| Value | Meaning |
|-------|---------|
| `00..7D`, `83..FF` | depth delta, `i8`, feet |
| `7D` or `83` | as above, and the **next** byte is a second `i8` delta to add (ascents/descents above 124 ft) |
| `7E` | deco / ascent warning ("ASC") |
| `7F` | ceiling violation ("ERR") |
| `81` | slow-down (ascent rate) warning |
| `80` | end of samples |
| `82` | end-of-profile (never inside a dive) |

Note that `7D` and `83` are ordinary deltas of +125 / −125 ft that additionally pull in an
extension byte.

Dive time in seconds = `(number_of_depth_samples × 3 + mem[marker + 1]) × 60`.
Gas: always air (21 % O2). No temperature, no tank pressure.

### Eon dive

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| `0x03` | u8 | s | sample interval |
| `0x04` | u8 | flags | bit 7 set: nitrox dive (Solution Nitrox / Vario); other bits not decoded |
| `0x05` | u8 | see right | nitrox dive: O2 percent. Otherwise: tank start pressure, 2 bar per unit |
| `0x06..0x0A` | 5 × `bcd8` | | year (2 digits), month, day, hour, minute |
| `0x0B..` | samples | | until the `80` byte |
| marker + 1 | u8 | °C + 40 | minimum (bottom) temperature: subtract 40 |
| marker + 2 | u8 | 2 bar | tank end pressure (not meaningful for nitrox dives) |

Bytes `0x00..0x02` are not decoded. Two-digit years below 85 are 20xx, otherwise 19xx. Seconds
are not stored. The time is local time of the device with no time zone.

Gas mix: O2 21 % unless the nitrox flag is set, then the value at `0x05`. A start and end
pressure of 0 means no tank pressure information.

Sample bytes, one per sample interval:

| Value | Meaning |
|-------|---------|
| `00..7C`, `83..FF` | depth delta, `i8`, feet |
| `7D` | surfaced event |
| `7E` | deco / ascent ("ASC") warning |
| `7F` | ceiling violation ("ERR") |
| `81` | slow (ascent rate) warning |
| `80` | end of samples |

Event bytes do not advance time: a time step is only taken when a depth byte arrives, so a run
of events followed by a depth delta all belong to the same sample. Dive time = number of depth
bytes × sample interval. Implementations typically add an artificial 0 ft point at time 0 and
another 0 ft point one interval after the last sample.

Confidence: **likely**.

### Spyder variant of the Eon dive

The Spyder (a Vyper-family device, see [suunto-vyper.md](suunto-vyper.md)) stores dives in the
Eon layout above with these differences:

- the date bytes at `0x06..0x0A` are plain binary, not BCD; two-digit years below 90 are 20xx;
- the temperature at marker + 1 is a signed `i8` in °C (no +40 offset);
- the nitrox flag is not used; `0x05` is always start pressure.

Confidence: **likely**.

## Model-specific variations

- Solution: 256-byte memory, per-byte handshake, backwards-stored profile, 3-minute fixed
  sample interval, no date.
- Eon / Solution Alpha / Solution Nitrox: 2304-byte memory in one reply with `sum8`, forward
  profile, configurable sample interval, BCD date.

## Open questions

1. Solution dives carry no date or time in the decoded bytes; whether bytes `0x00..0x02` of a
   Solution dive hold anything useful (e.g. surface interval) is unknown.
2. The Solution ring walk (boundary when the byte two ahead is `80`) has not been confirmed
   with a real memory image.
3. The meaning of the remaining bits of byte `0x04` and of bytes `0x00..0x02` in Eon dives is
   unknown.
4. Whether the Eon needs DTR asserted (to power the interface) is not stated by the reference;
   the Solution path asserts it explicitly.
5. The 'N' and 'T' configuration writes are not acknowledged in the reference; whether the
   device sends anything back is unknown.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
