# Pelagic VT Pro protocol (Oceanic, Aeris, Sherwood)

The VT Pro protocol is used by early Pelagic air-integrated computers: Oceanic VT Pro, Versa Pro
and Pro Plus 2; Aeris Atmos 2, Atmos AI, Elite and 500 AI; Sherwood Wisdom. They connect through a
serial interface cable at 9600 baud. The cable contains a microcontroller that must be initialised
before the dive computer answers. The host reads memory in 16-byte pages, up to four pages per
command. Each page carries its own 8-bit checksum. Short control replies use a nibble-sum
checksum. Memory layout and download algorithm follow
[pelagic-common.md](pelagic-common.md). The Aeris 500 AI uses a variant with a different cable
mode and a dedicated logbook command.

## Devices

| Vendor | Model | Model code | Transport | Notes |
|--------|-------|------------|-----------|-------|
| Aeris | 500 AI | `4151` | serial | "INTR" cable mode, layout L-500AI, no version string |
| Oceanic | Versa Pro | `4155` | serial | layout L-32A |
| Aeris | Atmos 2 | `4158` | serial | layout L-32A |
| Oceanic | Pro Plus 2 | `4159` | serial | layout L-32A |
| Aeris | Atmos AI | `4244` | serial | layout L-32A |
| Oceanic | VT Pro | `4245` | serial | layout L-32A |
| Sherwood | Wisdom | `4246` | serial | layout L-32W |
| Aeris | Elite | `424F` | serial | layout L-32A |

Identification: by the 16-byte version string; see the
[model table](pelagic-common.md#devices-and-model-codes). The Aeris 500 AI returns no version
string, so the host must be told the model in advance.

Confidence: **likely**.

## Transport parameters

| Parameter | Value |
|-----------|-------|
| Baud rate | 9600 |
| Framing | 8N1, no flow control |
| Read timeout | 3000 ms (9000 ms for the calibration command) |
| DTR | high for the whole session |
| RTS | pulsed low to reset the cable, see below |

See [../transports/serial.md](../transports/serial.md) for serial details.

Confidence: **likely**.

## Session

1. Open the port at 9600 8N1. Set DTR high.
2. Set RTS low, wait 100 ms, set RTS high. This resets the cable's microcontroller.
3. Wait 100 ms (1000 ms for the 500 AI). Discard all buffered data.
4. **Cable init.** Send the cable-mode command. The cable answers with a 13-byte ASCII string,
   with no ACK byte before it:

   | Mode | Models | H→D | D→H |
   |------|--------|-----|-----|
   | MOD | all except 500 AI | `AA 00` | `MOD--OK_V2.00` = `4D 4F 44 2D 2D 4F 4B 5F 56 32 2E 30 30` |
   | INTR | 500 AI | `20 00` | `INTR-OK_V1.11` = `49 4E 54 52 2D 4F 4B 5F 56 31 2E 31 31` |

5. **Download mode.** Send `88 00`. The reply is `5A`, 8 bytes, and a nibble-sum checksum. The
   8 bytes are the first half of the version string and can be ignored. The device must be in PC
   mode (selected by the user on the device).
6. **Version string** (MOD mode only). Send `72 03 00 00`, then `72 03 10 00`. Each reply is `5A`,
   8 bytes, a nibble-sum checksum, and the end byte `51`. The two 8-byte halves form the 16-byte
   version string. In INTR mode the version string is treated as 16 zero bytes.
7. **Calibration** (recommended). Send `18 00` with the read timeout raised to 9 s. The device
   needs about 6 s. The reply is `5A` and two bytes; the second byte must be `00`. Afterwards,
   memory reads are considerably faster.
8. Select the layout by [matching the version string](pelagic-common.md#version-string-matching)
   (L-32A if nothing matches). For the 500 AI, use layout L-500AI.
9. Download (see below).
10. **Quit.** Send `6A 05 A5 00`. The reply is `5A 51`. The device returns to surface mode.

Variant: another implementation expects a `5A` before the 13-byte cable reply. It does not send the
calibration command or the quit command.

Confidence: **likely** for the order and bytes; **uncertain** whether a `5A` precedes the cable
init reply.

## Commands

| Command | H→D bytes | D→H response | Notes |
|---------|-----------|--------------|-------|
| Cable init (MOD) | `AA 00` | 13 ASCII bytes | no ACK |
| Cable init (INTR) | `20 00` | 13 ASCII bytes | 500 AI |
| Download mode / ident | `88 00` | `5A` + 8 bytes + nibble-sum | |
| Version half | `72 03 XX 00`, XX = `00` or `10` | `5A` + 8 bytes + nibble-sum + `51` | MOD only |
| Calibrate | `18 00` | `5A` + 2 bytes (second = `00`) | ~6 s |
| Read pages | `34 FH FL LH LL 00` | `5A` + n × (16 bytes + sum8) | F = first page, L = last page, u16be page numbers (address / 16), n = L − F + 1 ≤ 4 |
| Read logbook index | `52 BB EE 00` | `5A` + k × (8 bytes + nibble-sum) | 500 AI only. BB = first entry index, EE = last entry index, k = EE − BB + 1 |
| Keep-alive | `6A 08 00 00` | `5A 51` | |
| Quit | `6A 05 A5 00` | `5A 51` | |

Error handling: if the first reply byte is not `5A` (for example `A5` = NAK), or no reply comes
within the timeout, resend the command. One implementation allows 2 resends.

Confidence: **likely**.

## Framing and integrity

Commands are raw bytes with no framing. Replies:

- `5A` (ACK) first, then the payload.
- Page data: each 16-byte page is followed by its [sum8](../common/checksums.md#sum8) (8-bit sum
  of the 16 bytes).
- 8-byte control replies: followed by a [nibble-sum](../common/checksums.md#nibble-sum) byte. This
  is the 8-bit sum of the high and low nibbles of every byte:
  `cs = Σ ((b >> 4) + (b & 0x0F)) mod 256`.
- Version-half replies end with the byte `51` after the checksum.

Worked example, first version half of a VT Pro:

```
H→D  72 03 00 00
D→H  5A 56 54 50 52 4F 20 20 72 40 51
```

`56 54 50 52 4F 20 20 72` = "VTPRO  r". Nibble sum: 5+6+5+4+5+0+5+2+4+F+2+0+2+0+7+2 = 0x40.

Second half (firmware bytes `31 30`):

```
H→D  72 03 10 00
D→H  5A 31 30 20 20 32 35 36 4B 30 51
```

Worked example, reading two pages at 0x0240 (pages 0x24 and 0x25):

```
H→D  34 00 24 00 25 00
D→H  5A <16 bytes page 0x24> <sum8> <16 bytes page 0x25> <sum8>
```

A page of `12 34 56 78 9A BC DE F0 12 34 56 78 9A BC DE F0` has sum8 `70`.

Confidence: **likely**.

## Memory layout

Layouts L-32A (VT Pro, Versa Pro, Pro Plus 2, Atmos 2, Atmos AI, Elite), L-32W (Wisdom) and
L-500AI. See [pelagic-common.md](pelagic-common.md#memory-layouts).

### Aeris 500 AI

- Memory size 0x20000. Device-info page at 0x0000, serial encoding S2.
- The pointer page is at 0x0110:

  | Offset | Type | Meaning |
  |--------|------|---------|
  | 0x02 | u8 | Index of the first logbook entry |
  | 0x03 | u8 | Index of the last logbook entry (inclusive) |
  | 0x04 | u16le | Profile ring first page (× 16 = address) |
  | 0x06 | u16le | Profile ring last page (× 16 = address) |

- Logbook entries are not read from memory. The host requests them with the index command `52`.
- Profile ring 0x0200..0x20000. Logbook entries use pointer form E1.

Confidence: **likely** (one source).

## Download procedure

For all models except the 500 AI, follow [the common procedure](pelagic-common.md#download-procedure),
reading with command `34` and up to 4 pages per command.

For the 500 AI:

1. Read the pointer page at 0x0110 (command `34`, one page).
2. Send `52 <first> <last> 00`. After the `5A`, read (last − first + 1) entries of 8 bytes, each
   followed by a nibble-sum byte. Verify each checksum.
3. Skip all-`FF` entries. If an entry equals the stored fingerprint, discard everything collected
   so far, because the device returns entries oldest first.
4. Continue with the profile passes of the common procedure, using the collected entries.

Confidence: **likely**.

## Dive data format

A dive record is the 8-byte logbook entry followed by the profile bytes. The minimum record size
is 56 bytes. Layout:

```
0x00..0x07   logbook entry
0x08..0x27   profile header (32 bytes)
0x28..       8-byte samples
last 16      footer
```

### Header

| Field | Location | Type, unit | Conversion |
|-------|----------|------------|------------|
| Start tank pressure | u16le at 0x26, masked `0x0FFF` | 2 psi | × 2 psi. If start and end are both 0, there is no tank |
| Sample interval (all except 500 AI) | (b0x27 >> 4) & 0x07 | index | 0..3 → 2, 15, 30, 60 s; other values → depth-based sampling (see below) |
| Sample interval (500 AI) | b0x27 >> 4 | index | 3..9 → 2, 5, 10, 15, 20, 25, 30 s; other values → depth-based |
| Max depth (all except 500 AI) | u16le at footer +0, masked `0x01FF` | ft | × 0.3048 m |
| Max depth (500 AI) | b(footer + 1) | ft | |
| O2 (all except 500 AI) | b(footer + 3) | % | 0 → 21 % |
| O2 (500 AI) | (u16le at footer +2 & 0x0FF0) >> 4 | % | 0 → 21 % |
| End tank pressure | u16le at footer +5, masked `0x0FFF` | 2 psi | × 2 psi |

There is one gas mix and at most one tank. The dive time is derived from the samples.

### Samples

8-byte samples from offset 0x28 to the footer. Skip samples that are all `00` or all `FF`.

| Field | All except 500 AI | 500 AI |
|-------|-------------------|--------|
| Time stamp | hours = bcd(s1 & 0x0F), minutes = bcd(s0) | same |
| Depth | s3, ft | (u16le@2 & 0x0FF0) >> 4, ft |
| Temperature | s6, °F | (u16le@6 & 0x0FF0) >> 4, °F |
| Deco stop | s5 >> 4, units of 10 ft; 0 = no stop | - |
| Deco / NDL time | u16le@4 & 0x0FFF, minutes (NDL when no stop) | - |

Sample timing: the time stamp has a resolution of one minute, and several samples share it. Let
`m` be the time stamp in minutes, and `i` the 0-based position of the sample among those with the
same stamp.

- Time-based interval `t`: at most 60 / t samples share a stamp. Sample time =
  `m × 60 + (i + 1) × t` seconds. Ignore extra samples. If the stamp jumps by more than one
  minute, or goes backwards, treat it as a data error.
- Depth-based sampling: count the `n` non-empty samples with the same stamp. Sample time =
  `m × 60 + round((i + 1) × 60 / n)` seconds.

### Date and time

From the logbook entry (all models except 500 AI):

| Field | Source |
|-------|--------|
| Minute | bcd(b0) |
| Hour | bcd(b1 & 0x7F), 12-hour; PM = b1 bit 7 |
| Day | bcd(b3 & 0x7F) |
| Month | b4 >> 4 |
| Year | if the record has at least 40 bytes: 2000 + bcd(((b0x23 & 0xC0) >> 2) + (b0x22 >> 4)); else 2000 + bcd(b4 & 0x0F) |

The logbook entry stores only the last year digit; the full year is in the profile header.

500 AI:

| Field | Source |
|-------|--------|
| Minute | bcd(b0) |
| Hour | bcd(b1 & 0x0F) + 10 × (b3 & 0x01), 12-hour; PM = b3 bit 3 |
| Day | (b2 >> 4) \| ((b3 & 0x02) << 3) |
| Month | b3 >> 4 |
| Year | 1999 + (b2 & 0x0F) |

Hours are converted with `hour mod 12`, plus 12 if PM. No seconds, no time zone.

Confidence: **likely** (one implementation).

## Model-specific variations

- Wisdom: layout L-32W, otherwise identical.
- 500 AI: INTR cable mode, no version string, index-based logbook, own pointer page and
  dive-format fields as listed.

## Open questions

- Whether a `5A` precedes the 13-byte cable init reply (the sources disagree).
- The contents of the 8-byte reply to `88 00`, and of the calibration reply's first byte.
- Whether the 500 AI's year base (1999) is correct for all firmware versions.
- Meaning of the 500 AI device-info and header bytes not listed.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used as a
  factual reference; no code copied.
