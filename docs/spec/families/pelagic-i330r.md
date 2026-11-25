# Pelagic i330R protocol (Aqualung i330R, Apeks DSX)

The newest Pelagic generation, the Aqualung i330R (wrist and console) and the Apeks DSX, has a
Bluetooth LE interface and stores dives in a large serial flash. The host pairs once with a
6-digit PIN shown on the device and receives a 16-byte access code. It presents that code at the
start of every later session. Communication uses small packets with a start byte, a command byte,
an 8-bit CRC and a length. Memory is read by absolute flash address and length. Ring-buffer
pointers come from a "flash map" record instead of a pointer page. The logbook/profile model and
the dive record layout follow the rest of the Pelagic family; see
[pelagic-common.md](pelagic-common.md).

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Apeks | DSX | `4741` ("GA") | BLE | layout L-DSX, multi-gas, CCR mode |
| Aqualung | i330R | `4744` ("GD") | BLE | layout L-I330R |
| Aqualung | i330R Console | `474D` ("GM") | BLE | layout L-I330R |

Identification: the advertised BLE name starts with the two model-code letters, followed by
digits (see [BLE identification](pelagic-common.md#ble-identification)). After the wake-up command,
the device reports its model code directly.

Confidence: **likely**.

## Transport parameters

BLE only. See [../transports/ble.md](../transports/ble.md).

| Item | Value |
|------|-------|
| Service | `ca7b0001-f785-4c38-b599-c7c5fbadb034` |
| Notify characteristic (device → host) | `ca7b0002-f785-4c38-b599-c7c5fbadb034` |
| Write characteristic (host → device) | `ca7b0003-f785-4c38-b599-c7c5fbadb034` |
| Write type | write with response |
| Response timeout | 3000 ms |

One packet can carry up to 255 payload bytes plus a 5-byte header. One implementation expects
every packet to arrive in a single notification. It therefore needs an ATT MTU of at least
`5 + payload + 3`. Another implementation requests an MTU of 263 in one code path. Devices seem to
send data packets with at most 96 payload bytes (101-byte notifications).

Confidence: **uncertain** for UUIDs (single source; the notify/write UUIDs are given only as a
shared prefix with the service), MTU and packet sizes; **likely** for the timeout.

## Framing and integrity

### Packet

Both directions use the same packet layout:

| Byte | Content |
|------|---------|
| 0 | `CD` start byte |
| 1 | Flags (see below) |
| 2 | Command code. A device packet echoes the command it answers |
| 3 | CRC-8 over the whole packet, computed with this byte set to `00` |
| 4 | Payload length n (0..255) |
| 5..5+n−1 | Payload |

Flags:

| Value | Direction | Meaning |
|-------|-----------|---------|
| `40` | H→D | Request (command phase) |
| `80` | H→D | Data phase: carries a parameter block such as an access code or PIN |
| `00` | H→D | Flash read request |
| any value with (flags AND `C0`) ≠ `C0` | D→H | Data packet; append the payload to the response |
| `C0` (bits 7 and 6 both set) | D→H | Final status packet; payload byte 0 is the response code. It carries no response data |

Response codes in the final status packet:

| Code | Meaning |
|------|---------|
| 1 | Ready: the device expects the data phase or the next step |
| 2 | Done: the command completed |
| 13 | Access code invalid |

A response is: zero or more data packets, followed by exactly one final status packet. The host
MUST check, for each received packet, that byte 0 is `CD`, byte 2 equals the command sent, the
length fits the received bytes, and the CRC matches. The total data length must equal the length
the command implies.

### CRC-8

| Parameter | Value |
|-----------|-------|
| Width | 8 |
| Polynomial | `0x12` (x⁸ + x⁴ + x), non-reflected |
| Init | `0x00` |
| RefIn / RefOut | false / false |
| XorOut | `0x00` |
| Check ("123456789") | `0xEA` |
| Coverage | all packet bytes, with byte 3 set to `00` |

This polynomial is not in the RevEng catalogue. See
[../common/checksums.md#crc-8-msb-0x12](../common/checksums.md#crc-8-msb-0x12). A byte-wise form
that gives the same result: `x = crc XOR byte; t = x XOR (x >> 4) XOR (x >> 7);
crc = ((t << 4) XOR (t << 1)) AND 0xFF`.

### Data CRC-16

The hardware-calibration and flash-map records (256 bytes each) end with a 16-bit CRC over the
first 254 bytes, stored **big-endian** in the last two bytes:
[CRC-16/IBM-3740](../common/checksums.md#crc-16-ibm-3740) (poly `0x1021`, init `0xFFFF`, no
reflection, xorout `0x0000`, check `0x29B1`). Example: 254 zero bytes give `0x592D`, so the
record would end with `59 2D`. Flash reads (`0D`) carry no extra CRC beyond the per-packet CRC-8.

### Worked frames

All CRCs were computed with the CRC-8 above:

```
H→D  CD 40 FA F6 09 00 00 00 00 00 00 00 00 00        access request, command phase
H→D  CD 80 FA 92 10 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
                                                       access request, data phase (all-zero code)
H→D  CD 40 FB 8E 09 00 00 00 00 00 00 00 00 00        PIN request, command phase
H→D  CD 80 FB F4 06 01 02 03 04 05 06                 PIN "123456", data phase
H→D  CD 40 22 4A 09 00 00 00 00 0C 00 00 00 00        wake-up, read/write
H→D  CD 40 21 C2 09 00 00 00 00 0C 00 00 00 00        wake-up, read-only
H→D  CD 40 97 12 09 37 30 31 55 00 00 00 00 00        authentication, i330R
H→D  CD 40 97 6E 09 FF FF FF FF 00 00 00 00 00        authentication, DSX
H→D  CD 40 27 32 09 00 00 00 00 00 01 00 00 00        read hardware calibration
H→D  CD 40 2F C2 09 00 00 00 00 00 00 00 00 00        read flash map
H→D  CD 00 0D F0 09 00 20 10 00 40 00 00 00 00        read 0x40 bytes at 0x00102000
D→H  CD C0 FA 94 02 01 00                              final status "ready" (illustrative)
```

Another implementation independently lists the CRC bytes `F6` (access request), `4A` (wake-up), `12`
(authentication) and `C2` (flash map) for the same packets.

Confidence: **likely**.

## Commands

| Command | Code | Flags | H→D payload | D→H data | Final code |
|---------|------|-------|-------------|----------|------------|
| Access request | `FA` | `40` | 9 × `00` | none | 1 |
| Access code | `FA` | `80` | 16-byte access code | none | 2 (13 = invalid) |
| PIN request | `FB` | `40` | 9 × `00` | none | 1 |
| PIN | `FB` | `80` | 6 bytes: PIN digits as values 0..9, right-aligned, zero-padded on the left | 16-byte access code | 2 |
| Wake-up, read-only | `21` | `40` | `00 00 00 00 0C 00 00 00 00` | 16-byte identity | 2 |
| Wake-up, read/write | `22` | `40` | `00 00 00 00 0C 00 00 00 00` | 16-byte identity | 2 |
| Authentication | `97` | `40` | i330R: `37 30 31 55 00 00 00 00 00`; DSX: `FF FF FF FF 00 00 00 00 00` | none | 1 |
| Read A/D | `25` | | not used | | |
| Read hardware calibration | `27` | `40` | `00 00 00 00 00 01 00 00 00` | 256 bytes incl. CRC-16 | 2 |
| Read general settings | `29` | | not used | | |
| Read flash map | `2F` | `40` | 9 × `00` | 256 bytes incl. CRC-16 | 2 |
| Read device record | `31` | | not used | | |
| Read flash | `0D` | `00` | u32le address, u32le length, `00` | `length` bytes | 2 |

The codes `25`, `29` and `31` are known names only. Their parameters have not been documented.

Identity record (16 bytes, reply to `21`/`22`): bytes 12..13 = model code, u16be.

Hardware-calibration record: bytes 12, 13, 14 are `bcd8`. Serial number = bcd(b14) × 10000 +
bcd(b13) × 100 + bcd(b12).

Variant: another implementation derives the serial number from bytes 12..14 of the 16-byte access code,
with the same formula. It does not send command `27`.

Confidence: **likely** for the command bytes and payloads; **uncertain** for the serial source.

## Session

```
code = stored access code for this device, or 16 zero bytes
attempts = 0
loop:
    send FA/40 (9 zero bytes)          expect final code 1
    send FA/80 (code)                  expect final code 2
    if final code was 2 and code is not all zero: break
    if final code was 13 or code is all zero:
        attempts += 1; if attempts > 3: fail "no access"
        ask the user for the 6-digit PIN shown on the device
        send FB/40 (9 zero bytes)      expect final code 1
        send FB/80 (PIN digits)        receive 16-byte code, expect final code 2
        store code for this device
        continue loop
send 22/40 (wake-up, read/write)       receive 16-byte identity → model code
send 97/40 (authentication; payload by model code)   expect final code 1
send 27/40                             receive hardware calibration, verify CRC-16 → serial
send 2F/40                             receive flash map, verify CRC-16 → pointers
download (below)
```

The access code persists across sessions. A host SHOULD store it per device (for example keyed by
BLE address or name). The user then enters the PIN only once.

No quit command is defined. The host disconnects.

Variants in another implementation:

- Without a stored code, it first sends `FA/80` with an empty payload. It always runs the PIN flow,
  then sends `FA/80` with the new code.
- It ends the session with the atom2-style wrapped frame `CD 40 <seq> 02 A8 99`. The effect on
  these devices is unknown.

Confidence: **likely**.

## Memory layout

Layouts L-I330R and L-DSX in [pelagic-common.md](pelagic-common.md#memory-layouts):

| | i330R / i330R Console | DSX |
|--|----------------------|-----|
| Flash size | 0x400000 | 0x2000000 |
| Logbook ring | 0x102000..0x106000 | 0x800000..0x880000 |
| Logbook entry | 64 bytes | 512 bytes |
| Logbook read direction | forwards | backwards |
| Profile ring | 0x10A000..0x400000 | 0x1000000..0x2000000 |
| Profile pointers in entry | form E4: u32le at +8 (first), u32le at +12 (end, exclusive) | same |

### Flash map

The 256-byte flash map returned by `2F` holds the ring descriptors as u32le values:

| Offset | Meaning |
|--------|---------|
| 0x50 | Logbook ring lowest address |
| 0x54 | Logbook ring highest address (inclusive) |
| 0x58 | Logbook first entry |
| 0x5C | Logbook last entry |
| 0x70 | Profile ring lowest address |
| 0x74 | Profile ring highest address (inclusive) |
| 0x78 | Profile first byte |
| 0x7C | Profile last byte |

Rules:

- If the lowest and highest addresses are both non-zero, the ring end is highest + 1.
- A host SHOULD check that the ring bounds equal the layout values above, and treat a mismatch as
  an unknown layout.
- The logbook span for the download:
  - DSX: [first, last).
  - i330R: [lowest, last + 1). The whole range from the ring start is read.

Variant: another implementation decodes the logbook pointers the same way. It does not mention the
profile descriptors.

Confidence: **likely**.

## Download procedure

Follow [the common procedure](pelagic-common.md#download-procedure), with these differences:

- Reads use command `0D` with an absolute address and length. Requests may be large (one
  implementation reads up to 4096 bytes per request). Read the logbook and profile spans in a few
  large blocks.
- If a read fails with a protocol error or timeout, wait 1000 ms, discard pending notifications and
  retry. One implementation retries up to 3 times. Reads are idempotent.
- For the i330R the logbook is read **forwards** from the span start. The entry read first is
  treated as the newest (see the open questions).
- The profile size check against the remaining ring capacity is applied as for the other
  layouts. One source skips it for these models.

Confidence: **likely**.

## Dive data format

A dive record is `[logbook entry (L bytes)][profile]` with L = 64 (i330R) or 512 (DSX). Sizes:

| | i330R | DSX |
|--|-------|-----|
| Logbook entry L | 64 | 512 |
| Profile header | 80 | 256 |
| Samples start (Hdr) | 144 | 768 |
| Footer | 48 | 64 |
| Sample size | 16 | 32 |

### Header

| Field | i330R | DSX | Type, unit | Conversion |
|-------|-------|-----|------------|------------|
| Dive mode | b2 | b45 | u8 | see below |
| Sample interval | b(L + 0x24) | b(L + 0x24) | u8, seconds | |
| Gas O2 | b(L + 0x10 + i), i = 0..2 | b(L + 0x89 + 16·mode + i), i = 0..5 | u8, % | 0 → 21 %. i330R: 20 also means air (→ 21 %); 21 is 21 % nitrox |
| Gas He | - | see variants | u8, % | |
| Settings word | u32le at L + 0x0C | - | bit field | bit 16 set = fresh water, clear = salt water |
| GF low / GF high | - | b(L + 0x70), b(L + 0x71) | u8, % | Bühlmann gradient factors |
| Dive time | from samples | u16le at footer + 8 | seconds | |
| Max depth | u16le at footer + 10 | same | 1/10 ft | m = v / 10 × 0.3048 |
| Average depth | u16le at footer + 12 | same | 1/10 ft | |
| Min temperature | - | u16le at footer + 16 | 1/10 °F | °C = (v/10 − 32) × 5/9 |
| Max temperature | - | u16le at footer + 18 | 1/10 °F | |

Dive modes:

- i330R: 0 = normal (open circuit), 1 = gauge, 2 = freedive, as in the atom2 protocol.
- DSX: 0 = closed circuit (CCR), 1 = open circuit, 2 = sidemount (open circuit),
  3 = sidemount gauge, 4 = gauge. In modes 3 and 4 no gas mixes are defined. Modes 0..2 each have
  their own set of 6 mixes, 16 bytes apart.

Variants:

- DSX helium: mainline places the He bytes at `0xB9 + 16·mode + i`, counted from the record start.
  The Subsurface fork counts from the profile header: `L + 0xB9 + 16·mode + i`. The fork's
  placement matches the O2 table, which is also relative to L. Treat it as the likely correct one.
- The Subsurface fork does not decode the DSX dive time, min/max temperature, GF, CNS or TTS,
  and reads the DSX ppO2 from s9 (1/100 bar) like the i330R. Mainline does as tabled below.
- Water flag polarity: another implementation reads bit 16 set as salt water.

### Samples

Samples run from Hdr to the footer. Skip samples that are all `00` or all `FF`. Each sample is
one interval after the previous one.

| Field | Offset | Type, unit | Models | Conversion |
|-------|--------|------------|--------|------------|
| Active gas / tank | s0 >> 4 | 1-based index | DSX | gas switch when it changes; tank number for pressure |
| Depth | u16le@2 | 1/10 ft | both | m = v / 10 × 0.3048 |
| NDL | u16le@4 | minutes | both | used when the stop depth is 0 |
| Deco time | u16le@6 | minutes | both | used when the stop depth is not 0 |
| Stop depth | s8 | ft | both | 0 = no stop |
| ppO2 | s9 | 1/100 bar | i330R | |
| Temperature | u16le@10 | 1/10 °F | both | °C = (v/10 − 32) × 5/9 |
| ppO2 | u16le@12 | 1/100 bar | DSX | |
| Tank pressure | u16le@14 | psi | DSX | bar = psi × 0.0689476 |
| CNS | s16 | % | DSX | |
| Time to surface | s19 | minutes | DSX | |

The i330R records no tank pressure.

### Date and time

From the logbook entry, all binary, 24-hour clock:

| Field | Source |
|-------|--------|
| Hour | b3 |
| Minute | b4 |
| Day | b5 |
| Month | b6 |
| Year | 2000 + b7 |

No seconds, no time zone. Example: `xx xx xx 0E 1E 0C 06 18` → 2024-06-12 14:30.

Confidence: **likely** for fields shared by both sources (date, depth, interval, O2, temperature);
**uncertain** for DSX-only fields (one implementation) and the water flag.

## Model-specific variations

- i330R vs DSX: authentication payload, logbook direction, logbook span rule, entry size, dive
  record sizes, sample fields.
- i330R Console behaves like the i330R.

## Open questions

- Exact BLE characteristic UUIDs and the required MTU.
- Purpose of the authentication command `97` and of its payload ("701U" for i330R). It is not known
  whether other values are accepted.
- Whether the serial number is in the hardware-calibration record, the access code, or both.
- The i330R logbook is read forwards from the ring start up to `last + 1`. It is not confirmed
  whether newer entries are at lower addresses or whether the ring simply has not wrapped on
  known devices.
- Whether freedive mode on the i330R uses the generic 4-byte freedive samples of the atom2 format.
- The parameters of commands `25`, `29` and `31`.
- Meaning of other response codes besides 1, 2 and 13.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used as a
  factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>, used to
  cross-check variants; no code copied.
- Greg Cook, "Catalogue of parametrised CRC algorithms" (RevEng),
  <https://reveng.sourceforge.io/crc-catalogue/>, for the CRC-16/IBM-3740 parameters.
