# Pelagic atom2 protocol (Oceanic, Aeris, Aqualung, Sherwood, Hollis, Tusa, Subgear, Genesis, Beuchat)

This protocol is used by most Pelagic-built computers from about 2007 on: Oceanic Atom, Geo, Veo
2-4, VT3/VT4/VTX, OC1/OCi/OCS, Pro Plus 2.1-4/X; Aeris Epic, Manta, Elite T3, A300, F10/F11;
Aqualung i100-i770R; Sherwood Wisdom 2-4, Insight 2, Amphos, Vision, Sage, Beacon; Hollis DG03 and
TX1; Tusa Element II, Zen, Zen Air, Talis; Subgear XP-Air; Genesis React Pro White; Beuchat
Voyager 2G and Mundial. It is a byte-oriented command/response protocol over a serial link
(USB-serial cable at 38400 or 115200 baud). Bluetooth LE models use the same commands, carried in a
small packet wrapper, and add a handshake derived from the device name. The host reads 16-, 128- or
256-byte memory blocks addressed by page number. Every reply starts with an ACK byte and ends with
an additive checksum. Memory layout, ring buffers and download algorithm are shared with the other
Pelagic protocols; see [pelagic-common.md](pelagic-common.md).

## Devices

All models with family "atom2" in the [model table](pelagic-common.md#devices-and-model-codes).
Models that also support BLE: Pro Plus X (`ER`), i750TC (`EZ`), Sage (`FG`), i300C (`FH`), i200C
(`FI`, `GI`), i770R (`FQ`), i550C (`FR`), Geo 4.0 (`FS`), Veo 4.0 (`FT`), Wisdom 4 (`FU`), Pro Plus 4
(`FV`), Beacon (`GB`), i470TC (`GC`), Geo Air (`GK`). The VTX (`EW`) also has a Bluetooth link, but
the sources describe it differently; see [BLE transport](#ble).

Identification:

| Transport | Identification |
|-----------|----------------|
| USB serial | Pelagic's download cable uses an FTDI USB-serial chip. The FTDI product ID `0403:F460` is registered as "Oceanic dive instrument" in the Linux FTDI driver. Other generic USB-serial chips also work, because the protocol only needs a serial port. |
| USB CDC | One source reports that the A300CS-generation cable can appear as a USB CDC device `2047:082B`, or `FFFF:0005`. |
| BLE | Advertised name = two ASCII letters of the model code + decimal serial, e.g. `FH012345`. See [BLE identification](pelagic-common.md#ble-identification). |

Confidence: **likely** for the BLE name rule and `0403:F460` (public ID list plus one other source); **uncertain** for the USB CDC IDs (single source).

## Transport parameters

### Serial

| Parameter | Value |
|-----------|-------|
| Baud rate | 38400. 115200 for VTX (`4557`), i750TC (`455A`), Pro Plus X (`4552`), i770R (`4651`) |
| Framing | 8 data bits, no parity, 1 stop bit, no flow control |
| Read timeout | 1000 ms (one implementation uses 600 ms) |
| Line control | see the session start below |

See [../transports/serial.md](../transports/serial.md) for USB-serial chip handling.

### BLE

Service and characteristic UUIDs (one source only):

| Devices | Service | Notify (device → host) | Write (host → device) |
|---------|---------|------------------------|-----------------------|
| All BLE atom2 models (names `ER`, `EZ`, `FH`...`GK`) | `cb3c4555-d670-4670-bc20-b61dbc851e9a` | `a60b8e5c-b267-44d7-9764-837caf96489e` | `6606ab42-89d5-4a00-a8ce-4eb5e1414ee0` |
| VTX (name `EW`) | `da2b84f1-6279-48de-bdc0-afbea0226079` | `18cda784-4bd3-4370-85bb-bfed91ec86af` | `bf03260c-7205-4c25-af43-93b1c299d159` |

- Enable notifications on the notify characteristic before sending commands.
- Writes use "write with response".
- The default ATT MTU (23) is sufficient; every packet is at most 20 bytes. No MTU exchange is
  needed.
- See [../transports/ble.md](../transports/ble.md) for general BLE handling.

Confidence: **likely** for the packet size; **uncertain** for the UUIDs (single source).

## Session

### Serial session start

1. Configure the port (baud rate as above, 8N1).
2. Set DTR high.
3. Set RTS low, wait 100 ms, set RTS high. The cable contains a microcontroller that is reset by
   RTS low. It answers commands only after a clean reset.
4. Wait 100 ms, then discard all buffered input and output.
5. Send the version command `84` and read the 16-byte version string (see [Commands](#commands)).
   The device must be in PC/download mode (most models enter it automatically when the cable is
   attached).
6. Select the model code and memory layout by
   [matching the version string](pelagic-common.md#version-string-matching).

Variant: another implementation sets DTR, waits 500 ms, pulses RTS low for 100 ms, and sends the
version command as two bytes `84 00`.

### BLE session start

1. Connect, enable notifications.
2. Send the version command `84` inside the [BLE wrapper](#ble-wrapper). Read the 16-byte version
   string.
3. Send the access handshake `E5` built from the advertised name (below). If the device answers
   with `A5` instead of `5A`, treat the handshake as not required and continue.
4. Match the version string as on serial.

Access handshake: let the advertised name be `LLdddddd` (at least 8 characters). The command is
10 bytes:

```
E5 d1 d2 d3 d4 d5 d6 00 00 cs
```

- `d1..d6` are the six characters after the two letters, each converted to its digit value
  (`'0'` → 0x00 ... `'9'` → 0x09).
- `cs` is the [sum8](../common/checksums.md#sum8) of the 8 bytes `d1 .. 00 00`.
- The reply is a bare ACK `5A` with no data.

Example for name `FQ001124`: `E5 00 00 01 01 02 04 00 00 08`.

Pro Plus 4 over BLE: before every 50th memory-block read, repeat the version command and the
access handshake. Without this, the device ends the session.

Variants in another implementation:

- After the handshake, read the 16-byte device-info page at address 0. Take the model code from
  bytes 8..9 (u16be) instead of matching the version string.
- The i750TC (`EZ`) acknowledges the handshake with `A5`.
- The VTX (`EW`) is driven without the wrapper and without the handshake. Commands are sent as
  raw bytes over the GATT characteristic.

### Session end

Send `6A 05 A5 00`. The device answers with the single byte `A5` and returns to surface mode.

Variant: another implementation sends no quit command on serial (it only drops DTR). On BLE it sends
`A8 99` (wrapped) as the last command.

Confidence: **likely**.

## Commands

All commands are bytes as written. On BLE they are carried in the wrapper. `PH PL` is a 16-bit
big-endian block number.

| Command | H→D bytes | D→H response | Notes |
|---------|-----------|--------------|-------|
| Version | `84` | `5A` + 16 bytes + sum8 | ASCII version string |
| Read 16 bytes | `B1 PH PL` | `5A` + 16 bytes + sum8 | Block number = address / 16 |
| Read 128 bytes | `B4 PH PL` | `5A` + 128 bytes + sum8 (Pro Plus 4: sum16) | Block number = address / 16 (multiple of 8) |
| Read 256 bytes | `B8 PH PL` | `5A` + 256 bytes + sum16 | Block number = address / 16 (multiple of 16) |
| Read 256 bytes, high memory | `F6 PH PL` | `5A` + 256 bytes + sum16 | Block number = (address − high-memory base) / 256 |
| Write 16 bytes, step 1 | `B2 PH PL` | `5A` | Block number = address / 16 |
| Write 16 bytes, step 2 | 16 data bytes + sum8 | `5A` | Not needed for download |
| Keep-alive | `91 05 A5` | `5A` | |
| BLE access handshake | `E5` + 8 bytes + sum8 | `5A` (or `A5`, see above) | BLE only |
| Quit | `6A 05 A5 00` | `A5` | |
| End (one source) | `A8 99` | unknown | Another implementation, BLE only |

Choice of read command per layout (see "Read block" in the
[layout table](pelagic-common.md#memory-layouts)):

- `B1` for all layouts with read block 16.
- `B4` for L-64P (Pro Plus 3, Pro Plus 4) and L-128F (F11).
- `B8` for L-256C (A300CS, VTX, i750TC, Sage, Beacon), L-HX and L-HR below the high-memory base.
- `F6` at and above the high-memory base (L-HX, L-HR).

Variant: another implementation uses `B4` for F11 (`4549`, `4554`) and i300C (`4648`), and not for Pro
Plus 3/4. It uses `B8` only on the USB connection for the VTX.

Confidence: **likely**.

## Framing and integrity

### Serial

Each exchange is: host writes the command; device answers with one status byte, then (for read
commands) the data and checksum.

- `5A` = ACK. The data follows.
- `A5` = NAK. The command was refused or is not supported. The quit command is the exception: its
  normal answer is `A5`.

Checksums:

- 1-byte: [sum8](../common/checksums.md#sum8), the 8-bit sum of the data bytes (not including the
  ACK byte), initial value 0.
- 2-byte: [sum16](../common/checksums.md#sum16), the 16-bit sum of the data bytes, initial value 0,
  sent little-endian.

A host SHOULD verify the checksum. On a timeout, a wrong first byte or a checksum mismatch, wait
100 ms, discard buffered input and resend. One implementation tries 3 times in total. It also
inserts a pause before every later command, which grows by 1 ms per retry up to 16 ms.

Worked example (serial, read the pointer page at address 0x0040 = block 4):

```
H→D  B1 00 04
D→H  5A 00 00 00 00 40 02 58 02 40 0A 50 3C 00 00 00 00 72
```

The checksum is 0x72 = (0x40+0x02+0x58+0x02+0x40+0x0A+0x50+0x3C) mod 256. Decoded: logbook
pointers 0x0240 and 0x0258 (mode G0: four 8-byte entries, 0x0240..0x025F). Profile ring pointers
0x0A40 and 0x3C50.

Example of a 2-byte checksum: a 256-byte block containing the bytes `00 01 02 ... FF` ends with
`80 7F` (sum 0x7F80).

### BLE wrapper

Every command is sent as one or more packets of at most 20 bytes:

| Byte | Content |
|------|---------|
| 0 | `CD` |
| 1 | Status: bit 7 = direction (0 host→device, 1 device→host); bit 6 = 1; bit 5 = more packets follow; bits 4..0 = packet index within this message (0, 1, 2, ...) |
| 2 | Message sequence number: 0 for the first command of the connection, incremented by 1 after each successfully completed command |
| 3 | Payload length, 1..16 |
| 4.. | Payload |

A host message is the command bytes. Commands longer than 16 bytes are split. A device message is
the serial reply (ACK byte, data, checksum), split into 16-byte payloads. The host checks for each
received packet that:

- byte 0 is `CD`,
- byte 1 is `C0 | index | (more ? 20 : 00)`,
- byte 2 equals the sequence number of the command,
- byte 3 does not exceed the packet length minus 4.

It collects payloads until a packet with bit 5 clear arrives.

The Pro Plus X and the i770R may return more data than requested on BLE. A host SHOULD accept
replies of up to 256 data bytes plus checksum and ignore the excess.

Worked example (BLE, Geo 4.0, sequence numbers as they occur in a session):

```
H→D  CD 40 00 01 84                                         version, message 0
D→H  CD E0 00 10 5A 4F 43 45 47 45 4F 34 30 20 31 30 20 35 31 32
D→H  CD C1 00 02 4B 9A                                       "OCEGEO40 10 512K", sum8 9A
H→D  CD 40 01 0A E5 00 00 01 01 02 04 00 00 08             handshake for name "FS001124"
D→H  CD C0 01 01 5A
H→D  CD 40 02 03 B1 00 00                                   read device-info page
D→H  ...
H→D  CD 40 03 03 B1 00 04                                   read pointer page
D→H  CD E0 03 10 5A 00 00 00 00 40 02 58 02 40 0A 50 3C 00 00 00
D→H  CD C1 03 02 00 72
```

Confidence: **likely** for the wrapper (one implementation, consistent with another implementation, which
describes a 4-byte header per 20-byte notification).

## Memory layout

See [pelagic-common.md](pelagic-common.md#memory-model). The device-info page is at 0x0000 and the
pointer page at 0x0040 for every atom2 layout.

### High memory

Layouts L-HX (Pro Plus X) and L-HR (i770R) have a high-memory base of 0x40000. Addresses below it
are read with `B8` and 16-byte block numbers. For an address `a` ≥ 0x40000, the host uses `F6`
with block number `(a − 0x40000) / 256`. The logbook ring is in low memory; the profile ring is
entirely in high memory. Profile pointers in logbook entries are then 256-byte page numbers
relative to the base (see [profile pointers](pelagic-common.md#profile-pointers-inside-a-logbook-entry)).
The last profile page is padding. The logbook entry field at +12 gives the number of profile
pages that hold data; see the download pseudocode in the common document.

Confidence: **likely**.

## Download procedure

Follow [the common procedure](pelagic-common.md#download-procedure) with the read commands above.
Read the device-info page (16 bytes at 0x0000), then the pointer page (16 bytes at 0x0040), then
the logbook and profile rings. Reads are aligned to the read block; cache the last block.

For layout L-EMPTY (Veo 1.0, XR-1 NX) there are no rings, and the host reports zero dives.

Confidence: **likely**.

## Dive data format

A dive record is `[logbook entry][profile]`, with offsets counted from the start of the logbook
entry. Four regions:

```
0 .. Hdr−1           logbook entry + profile header ("Hdr" bytes in total)
Hdr .. len−Ftr−1     samples, fixed size ("Sample" bytes) except type BB
len−Ftr .. len−1     footer ("Ftr" bytes, normally 16)
```

The last 8 bytes of the header, at offset `h = Hdr − 8`, form a "header sample" with start-of-dive
values. For VT4, VT 4.1, A300 AI, Vision and XP-Air, `h` = 0x30 instead.

The Genesis React Pro White (`4354`) downloads with this protocol, but its dive records use the
Veo 250 format; see [pelagic-veo250.md](pelagic-veo250.md#dive-data-format).

### Per-model geometry

Columns:

- Hdr / Ftr: header and footer sizes in bytes.
- Date: [date format](#date-and-time).
- Mode: the dive-mode source. `-` means always "normal".
- Interval: the sample-interval source.
- Gas O2: number of gas mixes and the offset of the first O2 byte. Consecutive mixes follow at
  +1, or at +2 where "step 2" is shown.
- Sample: sample size in bytes for normal (non-freedive) dives.

| Code | Model | Hdr | Ftr | Date | Mode | Interval | Gas O2 | Sample |
|---|---|---|---|---|---|---|---|---|
| 4250 | Oceanic Atom 1.0 | 40 | 16 | A | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 4257 | Aeris Epic (A) | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4258 | Oceanic VT3 | 72 | 16 | B | (b2 >> 6) & 3 | b0x17 &3 | 3 @ 0x44 | 8 |
| 4259 | Aeris Elite T3 (A) | 72 | 16 | B | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4342 | Oceanic Atom 2.0 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4344 | Oceanic Geo | 56 | 16 | A | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4345 | Aeris Manta | 56 | 16 | A | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4346 | Aeris XR-1 NX | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4347 | Oceanic Datamask | 56 | 16 | B | - | b0x17 &3 | 1 @ 0x33 | 8 |
| 4348 | Aeris Compumask | 56 | 16 | B | - | b0x17 &3 | 1 @ 0x33 | 8 |
| 434D | Aeris F10 | 48 | 0 | E | always freedive | 1 s | none | 2 |
| 434E | Oceanic OC1 (A) | 72 | 16 | C | - | b0x17 &3 | 3 @ 0x44 | 16 |
| 4350 | Sherwood Wisdom 2 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4353 | Sherwood Insight 2 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4357 | Tusa Element II | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4358 | Oceanic Veo 1.0 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4359 | Oceanic Veo 2.0 | 56 | 16 | B | (b1 >> 5) & 3 | b0x17 &3 | 3 @ 0x34 | 8 |
| 435A | Oceanic Veo 3.0 | 56 | 16 | B | (b1 >> 5) & 3 | b0x17 &3 | 3 @ 0x34 | 8 |
| 4441 | Tusa Zen | 56 | 16 | A | - | b0x17 &3 | 2 @ 0x34 | 8 |
| 4442 | Tusa Zen Air | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4443 | Aeris Atmos AI 2 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4444 | Oceanic Pro Plus 2.1 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4446 | Oceanic Geo 2.0 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4447 | Oceanic VT4 | 88 | 16 | C | - | b0x17 &3 | 4 @ 0x34 | 8 |
| 4449 | Oceanic OC1 (B) | 72 | 16 | C | - | b0x17 &3 | 3 @ 0x44 | 16 |
| 444B | Beuchat Voyager 2G | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 444C | Oceanic Atom 3.0 | 72 | 16 | C | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 444D | Hollis DG03 | 72 | 16 | B | (b2 >> 6) & 3 | b0x17 &3 | 3 @ 0x44 | 8 |
| 4450 | Oceanic OCS | 56 | 16 | C | (b1 >> 5) & 3 | b0x17 &3 | 3 @ 0x34 | 8 |
| 4451 | Oceanic OC1 (C) | 72 | 16 | C | - | b0x17 &3 | 3 @ 0x44 | 16 |
| 4452 | Oceanic VT 4.1 | 88 | 16 | C | - | b0x17 &3 | 4 @ 0x34 | 8 |
| 4453 | Aeris Epic (B) | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4455 | Aeris Elite T3 (B) | 72 | 16 | B | (b2 >> 6) & 3 | b0x17 &3 | 3 @ 0x44 | 8 |
| 4456 | Oceanic Atom 3.1 | 72 | 16 | C | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4457 | Aeris A300 AI | 72 | 16 | C | - | b0x17 &3 | 4 @ 0x34 | 8 |
| 4458 | Sherwood Wisdom 3 | 72 | 16 | A | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 445A | Aeris A300 | 56 | 16 | A | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4542 | Hollis TX1 | 104 | 16 | F | - | b0x17 &3 | 6 @ 0x3E, He 0x48 | 16 |
| 4543 | Beuchat Mundial 2 | 48 | 0 | E | always freedive | 1 s | none | 2 |
| 4545 | Sherwood Amphos | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4546 | Sherwood Amphos Air | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4548 | Oceanic Pro Plus 3 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4549 | Aeris F11 | 80 | 0 | E | always freedive | b0x29 &3 | none | 2 |
| 454B | Oceanic OCi | 72 | 16 | C | - | b0x17 &3 | 4 @ 0x28 | 16 |
| 454C | Aeris A300CS | 80 | 16 | H | - | b0x1F &3 | 1-4 @ 0x2A (count from b0x39) | 16 |
| 454E | Tusa Talis | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4550 | Beuchat Mundial 3 | 48 | 0 | E | always freedive | 1 s | none | 2 |
| 4552 | Oceanic Pro Plus X | 48 | 16 | H | - | b0x1F &3 | 4 @ 0x24 | 16 |
| 4553 | Oceanic F10 | 48 | 0 | E | always freedive | 1 s | none | 2 |
| 4554 | Oceanic F11 | 80 | 0 | E | always freedive | b0x29 &3 | none | 2 |
| 4555 | Subgear XP-Air | 72 | 16 | C | - | b0x17 &3 | 4 @ 0x34 | 8 |
| 4556 | Sherwood Vision | 72 | 16 | C | - | b0x17 &3 | 4 @ 0x34 | 8 |
| 4557 | Oceanic VTX | 80 | 16 | H | - | b0x1F &3 | 1-4 @ 0x2A (count from b0x39) | 16 |
| 4559 | Aqualung i300 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 455A | Aqualung i750TC | 80 | 16 | H | - | b0x1F &3 | 1-4 @ 0x2A (count from b0x39) | 16 |
| 4641 | Aqualung i450T | 80 | 16 | H | - | b0x1F &3 | 3 @ 0x30 | 16 |
| 4642 | Aqualung i550 | 56 | 16 | C | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4646 | Aqualung i200 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4647 | Sherwood Sage | 80 | 16 | H | - | b0x1F &3 | 1-4 @ 0x2A (count from b0x39) | 16 |
| 4648 | Aqualung i300C | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4649 | Aqualung i200C | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 464E | Aqualung i100 | 40 | 16 | B | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 4651 | Aqualung i770R | 80 | 16 | H | - | b0x1F &3 | 4 @ 0x30 step 2 | 16 |
| 4652 | Aqualung i550C | 40 | 16 | C | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 4653 | Oceanic Geo 4.0 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4654 | Oceanic Veo 4.0 | 56 | 16 | B | - | b0x17 &3 | 3 @ 0x34 | 8 |
| 4655 | Sherwood Wisdom 4 | 40 | 16 | C | - | b0x17 &3 | 1 @ 0x24 | 8 |
| 4656 | Oceanic Pro Plus 4 | 40 | 16 | B | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 4657 | Sherwood Amphos 2.0 | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4658 | Sherwood Amphos Air 2.0 | 72 | 16 | D | - | b0x17 &3 | 3 @ 0x44 | 8 |
| 4742 | Sherwood Beacon | 80 | 16 | H | - | b0x1F &3 | 1-4 @ 0x2A (count from b0x39) | 16 |
| 4743 | Aqualung i470TC | 56 | 16 | C | - | b0x17 &3 | 3 @ 0x28 step 2 | 16 |
| 4745 | Aqualung i100 (v2) | 40 | 16 | C | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 4749 | Aqualung i200C (v2) | 40 | 16 | C | - | b0x17 &3 | 3 @ 0x24 | 8 |
| 474B | Oceanic Geo Air | 56 | 16 | C | - | b0x17 &3 | 3 @ 0x34 | 16 |

Model code 0 (unknown device) uses the defaults: Hdr 72, Ftr 16, date A, 3 gases at `h+4`,
sample 8, and all "default" columns of the sample table below.

### Header fields

| Field | Location | Type, unit | Conversion / meaning |
|-------|----------|------------|----------------------|
| Dive mode | "Mode" column | 2-bit code | 0 = normal (open circuit), 1 = gauge, 2 = freedive, 3 = undefined |
| Sample interval | "Interval" column | 2-bit index | b0x17 / b0x1F: index 0..3 → 2, 15, 30, 60 s. F11 b0x29: index 0..3 → 0.25, 0.5, 1, 2 s. Freedive (all models) and F10/Mundial: 1 s |
| Gas O2 | "Gas O2" column | u8, % | 0 → 21 % (air). Helium only on TX1 (u8 % at 0x48 + i) |
| Gas count (A300CS group) | b0x39 | bit flags | bit 2 set → 1 mix; else bit 3 → 2; else bit 4 → 3; else 4 |
| Start tank pressure | u16le at `h+2` (A300CS, VTX, i750TC: at `h+16`) | psi | 10000 = no transmitter. Not present on models whose pressure column below is `-` |
| Start temperature | b`h+7` | u8, °F | Initial value for models with delta temperature |
| Water type | b0x18 bit 7 (A300CS, VTX, i750TC, i770R only) | flag | set = fresh water, clear = salt water |
| Max depth | u16le at footer +4, masked `0x0FFF` | 1/16 ft | metres = value / 16 × 0.3048 |
| Max depth (F10, F11, Mundial) | u16le at 4 | 1/16 ft | |
| Dive time (F10, F11, Mundial) | b2, b3 | bcd8 | seconds = bcd(b2) + 60 × bcd(b3) |
| Dive time (i770R) | u16le at footer +0, masked `0x7FFF` | minutes | |
| Minimum temperature (i770R) | b(footer + 6) | u8, °F | |
| Dive time (all others) | derived | | number of samples × interval, including inserted surface samples |

Variants and conflicts:

- Another implementation masks the max depth with `0x1FFF`.
- Another implementation gives these extra mode sources: `b3 & 3` for the A300CS group, i450T, i770R
  and Pro Plus X; `(b1 >> 5) & 3` for Geo 4.0 and Veo 4.0; `(b0x17 >> 3) & 3` for i200 and i200C
  (in freedive mode, b2 & 0x7F is the dive number within the session).
- Another implementation reads the water flag as "set = salt" in one code path and "set = fresh" in
  another.
- i770R: mainline libdivecomputer reads the dive time and minimum temperature from the footer.
  The Subsurface fork derives the dive time from the samples instead, reads the start pressure at
  `h+16`, and decodes `AA` records with the 1-psi form of the A300CS group.
- F-series dive time: another implementation adds hours from b1 (BCD), i.e.
  seconds = bcd(b1)·3600 + bcd(b3)·60 + bcd(b2).
- F11 interval: another implementation reads the same byte (0x29) but maps it to 1/2/4 s.

### Samples

Samples start at offset Hdr and end before the footer. Rules for every sample:

1. Skip a sample that is all `00` (except in freedive mode) or all `FF`.
2. Byte 0 is the sample type in normal dives:
   - `AA`: tank switch / pressure record. Same size as a normal sample. Not a time step.
   - `BB`: surface interval record, always 16 bytes long. Not a time step on its own.
   - anything else: a regular sample. Time advances by one interval.
3. Freedive dives have no type byte. Every sample is a depth sample.

#### Tank switch (`AA`)

| Models | Tank number | Pressure |
|--------|-------------|----------|
| Datamask, Compumask | always 1 | u16le at 6, masked `0x0FFF`, 1 psi |
| A300CS, VTX, i750TC, Sage, Beacon | b1 & 3 | u16le at 6, masked `0x0FFF`, 1 psi |
| Atom 2.0, Epic (both codes) | b1 & 3 | u16be at 3, masked `0x0FFF`, × 2 psi |
| all others | b1 & 3 | u16be at 4, masked `0x0FFF`, × 2 psi |

Tank numbers are 1-based. 0 means "no tank". The pressure becomes the current pressure for the
following samples.

#### Surface interval (`BB`)

Bytes 1 and 2 are `bcd8` minutes and seconds of surface time. The host inserts
floor(surface time / interval) samples at depth 0, each one interval apart, and continues the time
axis after them.

#### Regular samples

Time: each regular sample is one interval after the previous one. Exception: i450T and i470TC
store an absolute time in the sample, b0 = `bcd8` minutes, b1 & 0x0F = `bcd8` hours,
b2 = `bcd8` seconds, counted from the dive start plus any inserted surface time. Skip samples with
a repeated time. Treat time going backwards as a data error.

Field sources per model (`sN` = sample byte N, `u16le@N` = little-endian word at byte N):

| Code | Depth | Temperature (°F) | Tank pressure (psi) | Deco/NDL | RBT (min) |
|---|---|---|---|---|---|
| 4250 | s3, whole ft | s6 | Δ −s1 | - | - |
| 4257 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 4258 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4259 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4342 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 4344 | u16le@2 &0FFF /16 ft | s6 | - | - | - |
| 4345 | u16le@2 &0FFF /16 ft | s6 | - | - | - |
| 4346 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4347 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4348 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 434D | u16le@0 /16 ft | - | - | - | - |
| 434E | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 4350 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 4353 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4357 | u16le@2 &0FFF /16 ft | s6 | - | - | - |
| 4358 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4359 | u16le@4 &0FFF /16 ft | s3 | - | - | - |
| 435A | u16le@4 &0FFF /16 ft | s3 | - | - | - |
| 4441 | u16le@2 &0FFF /16 ft | s6 | - | K2 | - |
| 4442 | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | P5 | - | - |
| 4443 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | K4 | u16le@6 &03FF |
| 4444 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 4446 | u16le@4 &0FFF /16 ft | s3 | - | - | - |
| 4447 | u16le@2 &0FFF /16 ft | T6 | P5 | - | - |
| 4449 | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 444B | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | Δ −s1 | - | - |
| 444C | u16le@2 &0FFF /16 ft | T6 | P5 | - | - |
| 444D | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | K2 | - |
| 4450 | u16le@2 &0FFF /16 ft | s1 | - | - | - |
| 4451 | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 4452 | u16le@2 &0FFF /16 ft | T6 | P5 | - | - |
| 4453 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 4455 | u16le@2 &0FFF /16 ft | Δ, sign ~s0.7 | Δ −s1 | - | - |
| 4456 | u16le@2 &0FFF /16 ft | T6 | P5 | K4 | u16le@6 &01FF |
| 4457 | u16le@2 &0FFF /16 ft | T6 | P5 | - | - |
| 4458 | u16le@2 &0FFF /16 ft | Δ, sign s0.7 | Δ −s1 | - | - |
| 445A | u16le@4 &0FFF /16 ft | s3 | - | - | - |
| 4542 | u16le@2 &0FFF /16 ft | s1 | u16le@4 | K3 | - |
| 4543 | u16le@0 /16 ft | - | - | - | - |
| 4545 | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | Δ −s1 | - | - |
| 4546 | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | P5 | - | - |
| 4548 | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | - | - |
| 4549 | u16le@0 /16 ft | - | - | - | - |
| 454B | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 454C | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | - |
| 454E | u16le@2 &0FFF /16 ft | s7 | - | - | - |
| 4550 | u16le@0 /16 ft | - | - | - | - |
| 4552 | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | u16le@8 &01FF |
| 4553 | u16le@0 /16 ft | - | - | - | - |
| 4554 | u16le@0 /16 ft | - | - | - | - |
| 4555 | u16le@2 &0FFF /16 ft | T6 | P5 | K4 | u16le@6 &03FF |
| 4556 | u16le@2 &0FFF /16 ft | T6 | P5 | K4 | u16le@6 &03FF |
| 4557 | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | - |
| 4559 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 455A | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | - |
| 4641 | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 4642 | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | K4 | u16le@6 &03FF |
| 4646 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4647 | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | - |
| 4648 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4649 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 464E | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4651 | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | u16le@8 &01FF |
| 4652 | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | K4 | u16le@6 &03FF |
| 4653 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4654 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4655 | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | K4 | u16le@6 &03FF |
| 4656 | u16le@2 &0FFF /16 ft | Δ, sign ~s5.2 | P5 | K4 | u16le@6 &03FF |
| 4657 | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | Δ −s1 | - | - |
| 4658 | u16le@2 &0FFF /16 ft | Δ, sign s5.2 | P5 | - | - |
| 4742 | u16le@2 &0FFF /16 ft | s11 | u16le@4 | K1 | - |
| 4743 | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |
| 4745 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 4749 | u16le@4 &0FFF /16 ft | s3 | - | K5 | - |
| 474B | u16le@4 &0FFF /16 ft | s3 | u16le@10 &0FFF | K5 | u16le@8 &01FF |

Legend:

- Depth `/16 ft`: metres = value / 16 × 0.3048.
- Temperature `sN`: u8 in °F.
- Temperature `T6`: an 8-bit °F value assembled from three fields:
  `((s7 >> 4) & 0x0F) | ((s7 & 0x0C) << 2) | ((s5 & 0x0C) << 4)`.
- Temperature `Δ`: a running value. It starts at the header temperature (b`h+7`). Each sample
  changes it by `(s7 & 0x0C) >> 2` degrees (0..3). If the sign bit is 1, subtract; else add. The
  sign bit is named in the cell: `s0.7` = bit 7 of s0, `~s0.7` = inverted bit 7 of s0, `s5.2` =
  bit 2 of s5, `~s5.2` = inverted bit 2 of s5.
- Pressure `P5`: `(((s0 & 0x03) << 8) | s1) × 5` psi.
- Pressure `Δ −s1`: a running value. It starts at the header start pressure (or the last `AA`
  record). Subtract s1 psi on every sample.
- Pressure `u16le@N`: psi. Masked with `0x0FFF` where shown.
- Pressure `-`: the model records no tank pressure.
- A pressure is reported only while the current tank number is not 0 and the start pressure was
  not 10000. The tank number defaults to 1 and changes with `AA` records.
- Deco/NDL forms. "stop" is in units of 10 ft; 0 means no stop, and then the time is the NDL.
  The time is in minutes.

  | Form | Stop | Time |
  |------|------|------|
  | K1 | `(s15 >> 4) & 0x07` | u16le@6 & 0x03FF |
  | K2 | `s5 >> 4` | u16le@4 & 0x0FFF |
  | K3 | s10 | u16le@6 |
  | K4 | `s5 >> 4` | u16le@4 & 0x03FF |
  | K5 | `s7 >> 4` | u16le@6 & 0x0FFF |

- Gas switches: only the TX1 stores the active mix in each sample, as `s0 & 0x07`, 1-based.
- Bookmark: OC1 (all codes), OCi and Geo Air set bit 7 of s12 in a sample that carries a bookmark.

Freedive dives (dive mode 2 on a model that has a mode field, and every dive on F10/F11/Mundial):

- Samples are 4 bytes (2 bytes on F10, F11, Mundial 2/3).
- Depth = u16le at 0, in 1/16 ft. Another implementation masks it with `0x0FFF`.
- No temperature, pressure or deco data. No gas mixes.
- All-`00` samples are not skipped.

Variants in another implementation:

- Depth mask `0x1FFF` instead of `0x0FFF`.
- K5 time mask `0x03FF` instead of `0x0FFF`.
- Another implementation gives no header sizes for `4552`, `4652`, `4655` and `4749`.
- Another implementation gives other header/footer sizes for freedive-mode dives: VTX 32/16 bytes,
  i450T 48/16 bytes.

### Date and time

The date and time of the dive start are in the logbook entry (bytes 0..7). Formats E and F also
use bytes 8..15; these models have entries of at least 16 bytes. `bcd()` means decode `bcd8`.

| Format | Minute | Hour (12 h) | Day | Month | Year − 2000 |
|--------|--------|-------------|-----|-------|-------------|
| A | bcd(b0) | bcd(b1 & 0x1F) | bcd(b3 & 0x3F) | b4 >> 4 | bcd(((b3 & 0xC0) >> 2) + (b4 & 0x0F)) |
| B | bcd(b0) | bcd(b1 & 0x1F) | b3 & 0x1F | b4 >> 4 | ((b3 & 0xE0) >> 1) + (b4 & 0x0F) |
| C | bcd(b0 & 0x7F) | bcd(b1 & 0x1F) | ((b0 & 0x80) >> 3) + (b3 >> 4) | b3 & 0x0F | ((b5 & 0xE0) >> 5) + ((b7 & 0xE0) >> 2) |
| D | bcd(b0) | bcd(b1 & 0x1F) | ((b3 & 0x80) >> 3) + (b5 >> 4) | b7 >> 4 | b3 & 0x1F |
| E | bcd(b12) | bcd(b13 & 0x7F) | bcd(b8) | bcd(b7) | bcd(b6) |
| F | b10 | b11 | bcd(b15) | bcd(b14) | bcd(b13) |
| H | bcd(b0) | bcd(b1 & 0x1F) | b9 | b8 | b10 |

- PM flag: bit 7 of b1, except format E, which uses bit 7 of b13. Convert the hour as
  `hour mod 12`, plus 12 if PM.
- Seconds are always 0. There is no time zone; the time is device-local.
- Year-2010 rule: some models store only the last digit of the year. If the decoded year is below
  2010, assume the most recent year with that last digit that is not in the future (relative to
  the host's current date). Example: on 2026-11-24, a stored year of 2005 becomes 2025, and 2008
  becomes 2018.

Variants:

- Format F (TX1): another implementation applies no PM correction to b11.
- Format H: another implementation masks the day with `0x1F` and the month with `0x0F`.
- Format A: another implementation decodes the day in binary (b3 & 0x1F) for some models with codes
  4259, 4455, 4446, 4653, 4548, 4348, 4347, 4353. In this document those models use format B.

Worked example (format C): bytes 0..7 = `25 89 00 B7 00 20 00 40`.
- minute = bcd(0x25) = 25
- hour = bcd(0x09) = 9, PM bit set → 21
- day = 0 + 0x0B = 11
- month = 7
- year = 2000 + (0x20 >> 5) + (0x40 >> 2) = 2000 + 1 + 16 = 2017

Result: 2017-07-11 21:25.

Confidence: **likely** for the per-model tables (one implementation; partially confirmed by another
implementation). **Uncertain** where variants are
listed.

## Model-specific variations

- **Pro Plus 4**: `B4` replies use a 2-byte sum16. BLE sessions repeat the handshake every 50
  reads.
- **Pro Plus X, i770R**: high memory and `F6`; BLE replies may be longer than requested.
- **i450T**: one source reports a second logbook used for freedives: pointer page 0x00E0,
  logbook 0x0440-0x10C0 with 32-byte entries, profile ring shared with the main log, pointer form
  E2. Date format for these entries: minute bcd(b12 & 0x7F), hour bcd(b13 & 0x1F),
  day bcd(b8 & 0x3F), month b7 & 0x0F, year bcd(b6). Header 64 bytes, no footer.
- **F10, F11, Mundial**: freedive-only, 32-byte logbook entries, no footer.
- **F11 (`4549`, `4554`)**: one source numbers the freedives within a session. The number resets
  after a surface interval longer than 120 (unit unknown, probably seconds).
- **React Pro White**: atom2 protocol, Veo 250 dive format.
- **Veo 1.0, XR-1 NX**: no dive memory accessible.

## Open questions

- BLE UUIDs and the VTX's separate service come from one source only.
- What `A8 99` does at session end. One source sends it; the other uses `6A 05 A5 00`.
- Whether the i750TC really answers the handshake with `A5`. If so, a host that treats `A5` as
  "handshake not supported" still works.
- Which read command the i300C uses (`B1` or `B4`).
- Header sizes of the Pro Plus X (48 bytes) and of i550C, Wisdom 4, i200C v2 and i100 v2 (40 bytes)
  rest on one source.
- Water-type flag polarity.
- The meaning of logbook entry bytes not listed here, and of the footer apart from max depth.
- Units of the RBT (remaining bottom time) values are assumed to be minutes.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used as a
  factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>, used to
  cross-check variants; no code copied.
- Linux kernel FTDI device ID list (GPL-2.0), `drivers/usb/serial/ftdi_sio_ids.h`,
  <https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio_ids.h>, for the
  Oceanic FTDI product ID; consulted for facts only.
