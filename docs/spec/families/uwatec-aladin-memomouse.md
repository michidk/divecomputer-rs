# Uwatec Aladin and Memomouse

The original Uwatec Aladin computers (Aladin Pro, Sport Plus, Air Twin, Air Z and their
nitrox/O2 versions, about 1990–2000) have no host-driven protocol. They transmit their whole
2 KiB memory once, unprompted, when the diver starts a transfer on the unit while it sits in
the interface. The **Memomouse** is a separate Uwatec logbook box that reads an Aladin and
can then be downloaded over a serial port with a small packet protocol.

Both paths are serial, and both send every byte **with its bit order reversed** (bit 7 on the
wire where bit 0 is expected). Both end up with the same per-dive record format, which this
document describes once.

## Devices

| Vendor | Model | Model code | Transport | Gas type |
|--------|-------|------------|-----------|----------|
| Uwatec | Aladin Air Twin | `0x1C` | Serial (Aladin interface) | Air |
| Uwatec | Aladin Sport Plus | `0x3E` | Serial | Air |
| Uwatec | Aladin Pro | `0x3F` | Serial | Air |
| Uwatec | Aladin Air Z | `0x44` | Serial | Air |
| Uwatec | Aladin Air Z O2 | `0xA4` | Serial | O2 series (oxygen-enriched) |
| Uwatec | Aladin Air Z Nitrox | `0xF4` | Serial | Nitrox series |
| Uwatec | Aladin Pro Ultra | `0xFF` | Serial | Nitrox series |
| Uwatec | Memomouse | – | Serial | Reports the model code of the Aladin it read |

The model code is a byte in the Aladin memory. Its high nibble selects the gas variant:
`0xF_` = nitrox series, `0xA_` = O2 series, anything else = air.

There is no USB or wireless identification; the interfaces are plain RS-232 serial cables.

Confidence: **likely**.

## Transport parameters

| Parameter | Aladin (direct) | Memomouse |
|-----------|-----------------|-----------|
| Baud | 19200 | 9600 |
| Format | 8N1 | 8N1 |
| Flow control | none | none |
| DTR | set (high) at open | low at open; raised for the transfer, then lowered |
| RTS | cleared | cleared |
| Read timeout | 3 s | 1 s |

The interfaces draw power or a "host ready" signal from DTR. For the Memomouse, wait 500 ms
before raising DTR so that the box notices a DTR change from a previous session.

**Bit reversal.** Every byte the devices send arrives with its bits mirrored (bit 0 ↔ bit 7,
1 ↔ 6, 2 ↔ 5, 3 ↔ 4). Reverse each received byte before interpreting it. Memomouse command
packets must be reversed the same way before sending. The single-byte Memomouse ACK and NAK
are given below as wire values.

Confidence: **likely**.

## Session

### Aladin (direct)

There is no host command. The host opens the port and waits. When the diver starts a
transfer on the unit, it sends a 4-byte preamble followed by the memory image.

1. Read one byte at a time. Wire bytes `55 55 55 00` form the preamble (`AA AA AA 00` after
   bit reversal). On a mismatch or a read timeout, start matching from the first byte again
   and keep waiting (the user may not have started the transfer yet).
2. Note the host time as soon as the preamble is complete (used for clock calibration).
3. Read the remaining 2046 bytes: 2044 bytes of memory and 2 checksum bytes.
4. Reverse the bits of all 2050 bytes.

### Memomouse

1. Raise DTR (after the 500 ms pause).
2. **Wait for the greeting.** Poll the port for data with 300 ms timeouts. Each time nothing
   arrives, purge the input buffer and send a NAK (wire byte `A8`). Stop when data starts to
   arrive.
3. Read one **inner packet** (see framing). It contains an identification string, which the
   host may discard.
4. Wait 50 ms, then send the **download command** (below). If the reply is NAK (`A8`), purge
   the input and send the command again. The reply must be ACK (`60`).
5. Wait for data, polling every 100 ms. The box may wait for the user, so there is no fixed
   timeout here.
6. Note the host time, then read one inner packet: the data.
7. Lower DTR.

Confidence: **likely**.

## Commands

The Memomouse has one command. The Aladin has none.

| Command | Logical bytes (before bit reversal) | Reply |
|---------|-------------------------------------|-------|
| Download dives newer than `since` | `07 05 00 55 <since:4 LE> <xor8>` | ACK, then a data inner packet |

`since` is a device-clock timestamp in half-second ticks (the same field as the dive
timestamp, see [Date and time](#date-and-time)); 0 requests everything. The layout follows
the framing rules below: outer length `07`, then an inner length `05 00`, the command byte
`55`, the 4-byte argument and the outer XOR. The inner packet of a command has no inner
checksum.

Worked example, `since` = 0:

```
logical  07 05 00 55 00 00 00 00 57
H→D      E0 A0 00 AA 00 00 00 00 EA     (bit-reversed)
D→H      60                              (ACK)
```

With `since` = `0x1A2B3C4D`:

```
logical  07 05 00 55 4D 3C 2B 1A 17
H→D      E0 A0 00 AA B2 3C D4 58 E8
```

Confidence: **likely**.

## Framing and integrity

### Aladin

| Offset (after bit reversal) | Size | Meaning |
|-----------------------------|------|---------|
| 0 | 4 | `AA AA AA 00` preamble |
| 4 | 2044 | memory image |
| 2048 | 2 | u16le checksum |

The checksum is the 16-bit sum of bytes 0..2047, **preamble included**, truncated to 16 bits
([sum16](../common/checksums.md#sum16), init 0). Example: if the image were all zero, the
checksum would be `0xAA × 3 = 0x01FE`, sent as `FE 01` (after reversal).

### Memomouse outer packets

All bytes on the wire are bit-reversed; the table shows the reversed (logical) values.

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 1 | `n` = payload length, at most 126 |
| 1 | n | payload |
| 1+n | 1 | XOR of the length byte and the payload ([xor8](../common/checksums.md#xor8)) |

The host answers every received outer packet:

- good checksum: send ACK, wire byte `60`;
- bad checksum or length > 126: purge the input buffer and send NAK, wire byte `A8`; the box
  repeats the packet.

`60` and `A8` are the bit-reversed forms of the ASCII control characters ACK (`06`) and NAK
(`15`).

### Memomouse inner packets

An inner packet is carried in the payloads of one or more consecutive outer packets (126
bytes each, the last one shorter). Concatenated payloads form:

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 2 | u16le `m` = body length |
| 2 | m | body |
| 2+m | 1 | XOR of bytes 0 .. 1+m ([xor8](../common/checksums.md#xor8)) |

So the inner packet is `m + 3` bytes. The first outer packet must hold at least the 2 length
bytes.

Confidence: **likely**.

## Memory layout

### Aladin memory image

Offsets are into the 2044-byte image (add 4 for offsets into the received buffer including
the preamble).

| Offset | Size | Meaning |
|--------|------|---------|
| `0x000`–`0x5FF` | 1536 | Profile ring buffer |
| `0x600`–`0x7BB` | 444 | Logbook: 37 entries × 12 bytes, a ring |
| `0x7BC` | 1 | Model code |
| `0x7ED` | 3 | Serial number, u24be |
| `0x7F2` | 2 | Total number of dives, u16be (only the last 37 are in the logbook) |
| `0x7F4` | 1 | Position of the newest logbook entry, 1..37 |
| `0x7F6` | 1 | End-of-profile pointer, bits 7..0 |
| `0x7F7` | 1 | Bits 3..1 are bits 10..8 of the end-of-profile pointer |
| `0x7F8` | 4 | Device clock at transfer time, u32be, half-second ticks |

The end-of-profile pointer addresses the **last byte** of the newest profile. Each profile in
the ring starts with a `FF` marker byte. The profile ring is smaller than 37 dives' worth of
profiles, so older logbook entries may have no profile left.

Logbook entry `k` (0-based) lives at `0x600 + 12k`.

### Memomouse data body

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 1 | unknown |
| 1 | 4 | u32le device clock at transfer time, half-second ticks |
| 5 | … | dive records, back to back |

The box sends the dives **twice**: first oldest to newest, then the same records again
newest to oldest (the second series always has 37 records, not all with profiles; it is
probably the connected Aladin's own logbook converted to this format). Stop parsing at the
first record whose 18-byte header equals that of the previous record.

The box can hold dives from several computers, but only the dives of the connected computer
are transferred.

Confidence: **likely**.

## Download procedure

### Aladin: building dive records from the image

```
n      := min(u16be(img[0x7F2]), 37)
newest := (img[0x7F4] + 36) mod 37                  -- 0-based logbook slot
eop    := img[0x7F6] + ((img[0x7F7] & 0x0F) >> 1) * 256
eop    := (eop + 1) wrapped into 0x000..0x5FF        -- one past the newest profile
prev   := eop; cur := eop; profiles_left := true
for i in 0 .. n-1:
    slot  := (newest - i) mod 37
    entry := img[0x600 + 12*slot .. +12]
    rec   := serial(3, from 0x7ED) ++ model(1, from 0x7BC) ++ entry(12) ++ u16le 0
    reverse the byte order of rec[11..15]           -- timestamp to little-endian
    len := 0
    if profiles_left:
        walk cur backwards through the ring (wrapping) until img[cur] == FF
            or until it returns to eop
        if a marker was found:
            len := ring distance from cur to prev, minus 1 (the marker)
            profile := len bytes after the marker (wrapping); prev := cur
        if cur reached eop: profiles_left := false
    rec[16..18] := u16le len; append profile
    if fingerprint known and u32le(rec[11..15]) <= fingerprint: stop
    emit rec, fingerprint rec[11..15]
```

Dives come out newest first. The record format is the one defined below.

### Memomouse

1. Run the session with `since` = the newest known dive's timestamp (or 0).
2. Walk the data body from offset 5: each record is `18 + u16le(record[16])` bytes. Stop at
   the end of data or at the first duplicate 18-byte header.
3. Report the records newest first (the reverse of their order in the first series).

The device serial number and model are those of the first record.

Confidence: **likely**.

## Dive data format

The per-dive record is the same for both paths.

### Header

| Offset | Type | Raw unit | Conversion | Meaning |
|--------|------|----------|------------|---------|
| 0 | u24be | – | – | Serial number of the Aladin |
| 3 | u8 | – | – | Model code |
| 4 | u8 | flag | bit 2 set: add 100 min | Dive time hundreds |
| 5 | bcd8 | minutes | – | Dive time (0–99, plus 100 if flagged) |
| 6 | u16be | bits 15..6: 10/64 m | `(v >> 6) × 10 / 64` m | Maximum depth (6 low bits unused) |
| 10 | u8 | bar, or 20 psi for model `0x1C` | `raw` bar; `raw × 20 psi` for the Air Twin | Tank begin pressure (0 = no tank data) |
| 11 | u32le | half-second ticks | see [Date and time](#date-and-time) | Timestamp; also the 4-byte fingerprint |
| 15 | i8 | 1/4 °C | `raw / 4` °C | Minimum temperature |
| 16 | u16le | bytes | – | Profile length `p` |
| 18 | `p` bytes | – | – | Profile |

The 12-byte logbook entry from the Aladin image occupies offsets 4..15 (with the timestamp
converted from big- to little-endian).

### Samples

The profile starts with a fixed block, then the sample words:

| Variant | Fixed block size |
|---------|------------------|
| Air | 22 bytes |
| Nitrox series (`0xF_`) | 24 bytes |
| O2 series (`0xA_`) | 25 bytes |

Profile byte 23 (record offset 41) holds the oxygen setting:

- O2 series: O2 percentage.
- Nitrox series: low nibble `k`; O2 = `20 + 2k` %, or 21 % when `k` = 0.
- Air: always 21 %.

If the profile is too short to contain the fixed block, assume air.

After the fixed block come samples every **20 s**, starting at t = 20 s:

| Field | Type | Conversion |
|-------|------|------------|
| Depth | bits 15..6 of a u16be word | `raw × 10 / 64` m (0.15625 m steps) |
| Warnings | bits 5..0 of the same word | see below |

After the sample word of every full minute (t mod 60 = 0) there is one extra byte of
decompression information, and for O2-series models a second extra byte with the O2
percentage. Their exact encoding is unknown.

Warning bits:

| Bit | Meaning |
|-----|---------|
| 0 | Decompression stop required (clear = within no-decompression limits) |
| 1 | Remaining bottom time too short (Air models only) |
| 2 | Ascent too fast |
| 3 | Deco ceiling violated |
| 4 | Workload too high (Air models only) |
| 5 | Tank pressure transmission error (always set on models without a transmitter) |

Confidence: **likely**.

### Date and time

The timestamp counts **half-seconds of the device clock**, which has no defined epoch. Convert
it with the clock value read during the same transfer:

```
dive_time = host_time_at_transfer + (timestamp − device_clock) / 2   seconds
```

Use the Aladin clock at image offset `0x7F8` or the Memomouse clock at body offset 1, and the
host time noted when the data began to arrive. The result is in the host's local time zone;
the device stores no time zone.

Confidence: **likely**.

## Model-specific variations

- Air Twin (`0x1C`) stores tank pressure in units of 20 psi.
- Nitrox and O2 series have longer fixed profile blocks and an O2 setting.
- O2-series profiles carry an extra byte per minute.

## Open questions

1. Meaning of record offsets 8..9 (inside the logbook entry) and of the fixed profile block.
2. Encoding of the per-minute decompression byte.
3. Meaning of byte 0 of the Memomouse data body and of the identification string.
4. Does the Aladin end-of-profile pointer really use bits 3..1 of `0x7F7`, or is that an
   artefact of a packed field?
5. Is the Memomouse `since` filter applied by the box, and against which clock?

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual
  reference; no code copied.
