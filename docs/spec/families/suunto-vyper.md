# Suunto Vyper family (Spyder, Stinger, Mosquito, D3, Vyper, Vytec, Cobra, Gekko, Zoop)

Suunto's second-generation wrist computers, sold from the late 1990s until the late 2000s.
They are read through Suunto's serial interface cable at 2400 baud, 8 data bits, **odd
parity**. The protocol has addressed memory reads and writes of up to 32 bytes, plus two
"dive stream" commands that return one dive at a time, newest first, in reversed byte order.
Every packet carries an XOR checksum. The cable is half-duplex and the host switches direction
with RTS.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Suunto | Spyder | `0x01` | serial | older memory layout; Eon-style dive records |
| Suunto | Stinger | `0x03` | serial | |
| Suunto | Mosquito | `0x04` | serial | |
| Suunto | D3 | `0x05` | serial | |
| Suunto | Vyper | `0x0A` | serial | |
| Suunto | Vytec | `0x0B` | serial | multiple nitrox mixes |
| Suunto | Cobra | `0x0C` | serial | air-integrated (tank pressure) |
| Suunto | Gekko | `0x0D` | serial | |
| Suunto | Zoop | `0x16` | serial | |

The model code is a byte in device memory (see Memory layout). It is not reported over a
separate identity command.

Identification: the devices are only reachable through a serial interface cable. Suunto's own
USB cable contains an FTDI USB-serial bridge; one source associates FTDI product ID
`0403:F680` with it (unconfirmed). Generic serial handling is described in
[../transports/](../transports/).

Confidence: **likely** for model codes; **uncertain** for the cable USB ID.

## Transport parameters

| Parameter | Value |
|-----------|-------|
| Baud rate | 2400 |
| Framing | 8 data bits, **odd parity**, 1 stop bit (8O1) |
| Flow control | none |
| DTR | asserted for the whole session: it powers the interface |
| RTS | direction control: asserted while the host transmits, cleared to receive |
| Read timeout | 600 ms to 1000 ms (sources differ) |

### Direction switching and the echo problem

Suunto's own interface does not echo, and its RTS switching makes it impossible to receive
while RTS is asserted. Many third-party clone interfaces **do** echo the transmitted bytes. A
sequence that works with both:

1. Wait before each command (see timing below).
2. Assert RTS.
3. Write the command.
4. Wait 200 ms. The echo, if any, arrives within roughly 40 ms; the device's reply does not
   start before roughly 500–600 ms.
5. Discard all pending input (removes the echo).
6. Clear RTS and read the reply.

Waiting too long with RTS asserted (more than about 500 ms) loses the reply.

Timing values from the two sources:

| Step | libdivecomputer | Independent analysis |
|------|----------|----------|
| Settle time after asserting DTR at open | 100 ms, then purge | 500 ms, then purge |
| Delay before each command | 500 ms | none effective |
| RTS held after write | 200 ms, purge input, then clear | 200 ms, then clear (no purge) |
| Read timeout | 1000 ms | 600 ms |

Confidence: **likely** for line settings and RTS polarity (both sources agree);
**uncertain** for exact delays.

## Session

There is no handshake or version command. After opening the port (DTR asserted, settle, purge),
the host reads the identity block from memory:

```
H→D  05 00 16 14 07                 read 20 bytes from 0x0016
D→H  05 00 16 14 <20 bytes> <xor>
```

That window (`0x16..0x29`) contains the identity data of both memory layouts:

- If the byte at `0x24` is `20`, `30` or `60`, the device is a **Spyder**: the value is
  actually its sample interval in seconds, and the identity block starts at `0x16`.
- Otherwise the identity block starts at `0x24`.

Identity block (at `0x24`, or at `0x16` for the Spyder):

| Offset | Type | Meaning |
|--------|------|---------|
| +0 | u8 | model code |
| +1 | u8 | firmware version |
| +2..+5 | 4 × u8 | serial number: each byte is a value 0..99; concatenate as two decimal digits each (e.g. `01 02 03 04` → 1020304) |

At the end of the session the host clears DTR.

Confidence: **likely**.

## Commands

All multi-byte addresses are big-endian. `<xor>` is the XOR of all preceding bytes of the
same packet (see Framing).

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Read memory | `05 AH AL N <xor>` | address, `N` ≤ 32 | `05 AH AL N <N bytes> <xor>` | reply repeats the 4 request bytes |
| Prepare write | `07 A5 A2` | none | `07 A5 A2` | must precede every write packet |
| Write memory | `06 AH AL N <N bytes> <xor>` | address, `N` ≤ 32, data | `06 AH AL N <xor>` | |
| First dive | `08 A5 AD` | none | stream of dive packets | starts at the newest dive |
| Next dive | `09 A5 AC` | none | stream of dive packets | continues with the next older dive |

The third byte of the prepare-write and the dive commands is the XOR of the first two
(`07 ⊕ A5 = A2`, `08 ⊕ A5 = AD`, `09 ⊕ A5 = AC`).

### Read memory reply validation

The reply is exactly `N + 5` bytes. The host MUST check that the first four bytes equal the
first four request bytes and that the last byte is the XOR of all others. One source retries
a failed read up to 6 times.

### Dive stream

After a dive command the device sends one or more packets:

```
D→H  <cmd> <len> <len bytes> <xor>
```

- `<cmd>` repeats the command byte (`08` or `09`).
- `<len>` is 0..32. Packets are 32 bytes long except possibly the last one of a dive.
- `<xor>` covers `<cmd>`, `<len>` and the data.
- A packet with `<len>` = 0 means the device reached the end of its ring buffer: the dive
  being assembled has been partially overwritten. Discard it and stop downloading.
- The end of a dive is detected by **silence**: the host reads until a read times out with no
  bytes. The first packet after a dive command must arrive; a timeout there is an error.
  (A last packet of exactly 32 bytes cannot be distinguished from a continuing stream, which is
  why a timeout is used.)

The device walks its ring buffer backwards, so the concatenated payload of one dive is in
**reverse byte order**. Reverse it to obtain the dive record described below. An empty result
(no packets with data) means there are no more dives.

Confidence: **likely**.

## Framing and integrity

Checksum: 8-bit XOR of every preceding byte of the packet, initial value `00`. See
[../common/checksums.md#xor8](../common/checksums.md#xor8). Test vector: XOR over ASCII
`123456789` is `31`.

Worked examples:

```
read 6 bytes at 0x0024 (identity block)
H→D  05 00 24 06 27
D→H  05 00 24 06 <model> <fw> <s0> <s1> <s2> <s3> <xor>

read 20 bytes at 0x0016
H→D  05 00 16 14 07

newest dive
H→D  08 A5 AD
D→H  08 20 <32 bytes> <xor>
D→H  08 20 <32 bytes> <xor>
D→H  08 0B <11 bytes> <xor>
     ... silence: dive complete, reverse the 75 bytes
```

Confidence: **likely**.

## Memory layout

Memory size: 8192 bytes (`0x0000..0x1FFF`). A full dump is 256 reads of 32 bytes.

| Item | Vyper layout | Spyder layout |
|------|--------------|---------------|
| Identity block | `0x24` | `0x16` |
| End-of-profile pointer | `0x51`, u16be | `0x1C`, u16be |
| Profile ring buffer | `0x71..0x1FFF` | `0x4C..0x1FFF` |
| Dive trailer length after `80` | 4 bytes | 2 bytes |
| Fingerprint offset in dive | 9 (5 bytes) | 6 (5 bytes) |

The end-of-profile pointer addresses a byte `82` in the ring. Dives are stored in forward
order; each dive's samples end with `80` followed by the trailer bytes, and the next dive
begins immediately after.

Confidence: **likely**. One source instead derives the ring start from the model code (Zoop
`0x77`, Vytec `0x71`, others `0x51`); see open questions.

## Download procedure

Two equivalent methods exist.

### Method 1: dive stream (preferred, incremental)

```
read identity window 0x16..0x29; choose Vyper or Spyder layout
remaining := ring size (0x2000 - ring start)
cmd := 08
loop:
    dive := read dive stream for cmd; cmd := 09
    if dive is empty: stop
    if size(dive) > remaining: error (corrupt)
    if dive[fp_offset .. fp_offset+5] = stored fingerprint: stop
    emit dive
    remaining := remaining - size(dive)
```

### Method 2: full dump

Read all 8192 bytes. Read the end-of-profile pointer, check it lies inside the ring and points
at `82`. Walk backwards from it (wrapping from ring start to `0x1FFF`). Whenever the byte
`trailer + 1` positions before the current one is `80`, a dive starts at the current position
and extends up to the previous boundary. The walk ends at the `82` marker. Apply the same
fingerprint test.

Fingerprint: 5 bytes at dive offset 9 (Vyper) or 6 (Spyder), i.e. the start date and time.

Confidence: **likely**.

## Dive data format

The record below is the dive after byte reversal (stream method) or as stored (dump method).
The Spyder uses the Eon record layout instead, described in
[suunto-solution-eon.md](suunto-solution-eon.md#spyder-variant-of-the-eon-dive).

### Header

| Offset | Type | Raw unit | Conversion | Meaning |
|--------|------|----------|------------|---------|
| `0x00` | u8 | min | | surface interval minutes (one source only) |
| `0x01` | u8 | h | | surface interval hours (one source only) |
| `0x02` | u8 | | | not decoded |
| `0x03` | u8 | s | | sample interval |
| `0x04` | u8 | flags | | bit 6: gauge (bottom timer) mode. Bits 0..3: personal/altitude adjustment setting (one source divides by 3 to obtain a conservatism level) |
| `0x05` | u8 | 2 bar | × 2 | tank start pressure; 0 = none |
| `0x06` | u8 | % | / 100 | O2 of the first mix; 0 means air (21 %) |
| `0x07` | u8 | | | not decoded |
| `0x08` | i8 | °C | | surface (air) temperature |
| `0x09` | u8 | year | yy < 90 → 2000 + yy, else 1900 + yy | start year (binary, not BCD) |
| `0x0A` | u8 | | | month (one source masks the low nibble) |
| `0x0B` | u8 | | | day |
| `0x0C` | u8 | | | hour |
| `0x0D` | u8 | | | minute |
| `0x0E..` | | | | samples, until `80` |

Trailer after the `80` sample terminator (`m` = offset of `80`):

| Offset | Type | Meaning |
|--------|------|---------|
| m + 1 | i8 | minimum (bottom) temperature, °C |
| m + 2 | u8 | not decoded |
| m + 3 | u8 | tank end pressure, 2 bar per unit |
| m + 4 | u8 | not decoded |

A record must have at least `m + 5` bytes.

### Samples

One byte per sample interval, depth in **feet** as a signed delta from the previous depth
(start at 0 ft; 1 ft = 0.3048 m):

| Value | Meaning |
|-------|---------|
| `00..78`, `88..FF` | depth delta, `i8`, feet |
| `79` | event, meaning unknown |
| `7A` | slow down: ascent too fast |
| `7B` | violation (ascent rate / decompression) |
| `7C` | bookmark (user marker) |
| `7D` | surfaced |
| `7E` | decompression stop required |
| `7F` | ceiling broken (decompression violation) |
| `80` | end of samples |
| `81` | safety stop |
| `82..86` | events, meaning unknown |
| `87` | gas change; the **next byte** is the new O2 percentage |

Event bytes do not consume time; time advances by one interval for every depth byte. Dive time
= number of depth bytes × sample interval. At most 3 distinct mixes occur in one dive (the
first mix from `0x06` plus up to two more from `87` events). In gauge mode there is no mix.

Implementations typically add a 0 ft point at time 0 and one interval after the last sample.

### Date and time

Local time of the device, minute resolution, no time zone, two-digit year as above.

Confidence: **likely** for fields decoded by both sources; offsets `0x00`/`0x01` (surface
interval) are **uncertain** (one source).

## Model-specific variations

- Spyder: Spyder memory layout (identity at `0x16`, pointer at `0x1C`, ring from `0x4C`) and
  Eon-style dive records with a 2-byte trailer.
- Cobra: air-integrated, so tank start/end pressure fields are filled.
- Gauge-mode dives (flag bit 6) have no gas mix.

## Open questions

1. One source sets the profile ring start from the model code (`0x77` for Zoop, `0x71` for
   Vytec, `0x51` as a default for others, `0x4C` for Spyder) instead of the fixed `0x71`;
   `0x51` coincides with the address of the end-of-profile pointer. Which is right matters
   only for full-dump parsing.
2. Inter-command delays and settle times differ between sources (see table above); a minimal
   safe timing is not established.
3. Meanings of event bytes `79` and `82..86` are unknown.
4. Bytes `0x02`, `0x07`, trailer `m + 2` and `m + 4` are not decoded.
5. The USB product ID of Suunto's own interface cable is not confirmed.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
