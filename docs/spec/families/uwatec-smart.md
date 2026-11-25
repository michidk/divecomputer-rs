# Uwatec / Scubapro Smart family

The Smart family covers every Uwatec and Scubapro computer from the Smart Pro (2000) up to
the G2/G3, A1/A2, Luna 2.0 and Aladin Matrix ranges. All of them speak one small
command/response protocol: a single command byte, optional fixed arguments, and an answer
whose length the host already knows. The device keeps its own logbook and, given a
"newer than" timestamp, returns every matching dive as one contiguous blob. The host never
addresses memory.

Four transports carry the same commands. Only the wrapping differs:

| Transport | Devices | Wrapping |
|-----------|---------|----------|
| IrDA (TinyTP over IrLMP) | Smart, Galileo, Aladin Tec/2G/3G, Aladin 2G/Sport/Prime/One | none: the raw command and answer bytes |
| Serial over USB (infrared cradle) | Meridian, Mantis, Mantis 2, Chromis | sync preamble, 32-bit length, XOR checksum, local echo |
| USB HID | Aladin Square, G2, G2 TEK, G2 Console, G2 HUD, G3 | length-prefixed 32-byte OUT / 64-byte IN reports |
| Bluetooth Low Energy | G2 family, G3, HUD, A1, A2, Luna 2.0 (AI), Aladin Sport/H/ONE Matrix | length-prefixed writes; notifications with a 1-byte header |

The dive blob is the same on every transport. Each dive is a fixed-size header followed by a
variable-length, bit-encoded sample stream. The header layout and the sample encoding table
depend on the model group (see [Dive data format](#dive-data-format)).

## Devices

Sorted by model code (the byte returned by command `10`). Several products share one code
and cannot be told apart from the protocol alone.

| Model code | Vendor and models | Transports | Header group | Sample table |
|------------|-------------------|------------|--------------|--------------|
| `0x10` | Uwatec Smart Pro | IrDA | P (92 bytes) | U1 |
| `0x11` | Uwatec Galileo Sol, Galileo Luna, Galileo Terra | IrDA | G (152) or T (84), see [Galileo trimix switch](#galileo-trimix-switch) | B |
| `0x12` | Uwatec Aladin Tec, Aladin Prime, Aladin One | IrDA | A (108) | U2 |
| `0x13` | Uwatec Aladin Tec 2G, Aladin 2G; Subgear XP-10 | IrDA | A2 (116) | U2 |
| `0x14` | Uwatec Smart Com | IrDA | C (100) | U3 |
| `0x15` | Uwatec Aladin 2G, Aladin Tec 3G, Aladin Sport; Subgear XP-3G | IrDA | G (152) | B |
| `0x17` | Scubapro Aladin Sport Matrix, Aladin H Matrix (and, by name, Aladin ONE Matrix) | BLE | T (84) | B |
| `0x18` | Uwatec Smart Tec | IrDA | Z (132) | U4 |
| `0x19` | Uwatec Galileo Trimix | IrDA | T (84) or G (152), see [Galileo trimix switch](#galileo-trimix-switch) | B |
| `0x1C` | Uwatec Smart Z; Subgear XP Air | IrDA | Z (132) | U4 |
| `0x20` | Scubapro Meridian, Mantis (M1) | Serial | G (152) | B |
| `0x22` | Scubapro Aladin Square | USB HID | G (152) | B |
| `0x24` | Scubapro Chromis | Serial | G (152) | B |
| `0x25` | Scubapro Aladin A1 | BLE | T (84) | B |
| `0x26` | Scubapro Mantis 2 (M2) | Serial | G (152) | B |
| `0x28` | Scubapro Aladin A2 | BLE | T (84) | B |
| `0x31` | Scubapro G2 TEK | USB HID, BLE | T (84) | B |
| `0x32` | Scubapro G2, G2 Console | USB HID, BLE | T (84) | B |
| `0x34` | Scubapro G3 (Galileo 3) | USB HID, BLE | T (84) | B |
| `0x42` | Scubapro G2 HUD (Galileo HUD) | USB HID, BLE | T (84) | B |
| `0x50` | Scubapro Luna 2.0 AI | BLE | T (84) | B |
| `0x51` | Scubapro Luna 2.0 | BLE | T (84) | B |

Further products are sold as members of this family but their model codes are not known:
Aladin Tec 3H, Subgear XP-H, Subgear XP-3H (all IrDA). They most likely report one of the
codes above.

### Identification per transport

**IrDA nickname** (from the IrLAP discovery reply; compare case-insensitively and in full):

- `Aladin Smart Com`
- `Aladin Smart Pro`
- `Aladin Smart Tec`
- `Aladin Smart Z`
- `Uwatec Aladin`
- `UWATEC Galileo`
- `UWATEC Galileo Sol`

**USB HID** (VID:PID):

| VID:PID | Device |
|---------|--------|
| `2E6C:3201` | G2, G2 TEK |
| `2E6C:3211` | G2 Console |
| `2E6C:4201` | G2 HUD |
| `C251:2006` | Aladin Square |

The G3 is listed as a USB HID device but no VID:PID is known for it. Expect a `2E6C` product
ID.

**USB serial**: the Meridian, Mantis, Mantis 2 and Chromis cradle uses a Silicon Labs CP210x
bridge, `10C4:EA60`. This is the chip's generic ID and identifies only the chip.

**BLE advertised name**. Two name lists are in use:

- Exact match (case-insensitive) against: `G2`, `G2 TEK`, `Galileo 3`, `HUD`, `A1`, `A2`,
  `Aladin`, `Luna 2.0`, `Luna 2.0 AI`.
- Prefix match against: `G2`, `Galileo 3`, `A1`, `A2`, `Luna`, `HUD`, `Aladin`
  (`Aladin H` is covered by `Aladin`).

Prefix matching is the safer choice when the name carries a suffix. The model code must still
be read with command `10` after connecting.

Confidence: **likely** for codes, USB IDs and IrDA nicknames. **uncertain** for the BLE name
rules, because the two lists differ.

## Transport parameters

### IrDA

The IrDA stack (discovery, IrLAP, IrLMP, IAS, TinyTP, USB dongles) is described in
[../transports/irda.md](../transports/irda.md). Only the Uwatec-specific parts follow.

- Discover devices and pick the one whose nickname matches the list above.
- Open a TinyTP connection (a `SOCK_STREAM` IrDA socket on operating-system stacks) to
  **remote LSAP selector 1**. The host does **not** query IAS for the remote LSAP; it uses
  selector 1 directly. On Windows this is addressed as the service name `LSAP-SEL1`.
- Every application byte is carried in TinyTP data PDUs. There is no application-level
  length or checksum; the IrLAP FCS protects each frame.
- Answers may arrive in several TinyTP PDUs. Read until the expected number of bytes has
  arrived.
- Application read timeout: 5 s.

Flow control (one implementation observation): when it runs its own TinyTP layer, the host
sends a data-less PDU granting 2 credits after every 2 data PDUs during the bulk transfer, and
each command answer seems to arrive as two PDUs, one of them empty. Implementations on an
operating-system TinyTP socket do not see this. Confidence for this paragraph: **uncertain**.

Confidence for the rest: **likely**.

### Serial (Meridian, Mantis, Mantis 2, Chromis)

| Parameter | Value |
|-----------|-------|
| Baud | 57600 |
| Format | 8N1 |
| Flow control | none |
| DTR / RTS | not changed |
| Read timeout | 5 s |

Purge both directions after opening. The cradle is an infrared link driven through a USB-serial
bridge (see [../transports/](../transports/) for CP210x setup). It is half-duplex and **echoes
every byte the host sends**.

Confidence: **likely**.

### USB HID (Aladin Square, G2 family)

- OUT report: 32 bytes of payload, no report ID (when the host API requires one, prefix
  `00`, giving 33 bytes on the wire).
- IN report: 64 bytes, no report ID.
- Use the first HID interface and its interrupt IN/OUT endpoints.
- Read timeout: 5 s for the protocol as a whole. One implementation polls with 200 ms reads.

Confidence: **likely**.

### BLE

| Role | UUID |
|------|------|
| Service | `fdcdeaaa-295d-470e-bf15-04217b7aa0a0` |
| H→D write characteristic | `a188b7dd-debb-449a-852d-c243d46b4b1a` |
| D→H notify characteristic | `aa0c68f0-ea9c-493d-8112-62879e72af68` |

- Enable notifications on the notify characteristic (write `01 00` to its CCCD).
- Write with response. Writes are at most 20 bytes; every command fits in one write.
- The default ATT MTU (23) is enough. Notifications carry at most 20 bytes (one header byte
  and up to 19 payload bytes).
- The G2 family, A1/A2, Luna 2.0 and Matrix models must be **bonded** before they answer.
  The HUD reportedly works without bonding. Use an existing bond when one exists.
- One implementation writes `06 00 06 00 00 00 0A 00` (connection interval 7.5 ms, latency
  0, supervision timeout 100 ms) to the GAP characteristic `2A04` after discovery, to speed
  up the transfer. This characteristic is normally read-only, so the write is optional and
  may be rejected.
- Wait about 800 ms after connecting (and after the device name has been read) before the
  first command.

Confidence: **likely** for the UUIDs (one source, consistent with the behaviour of the
framing). **uncertain** for the bonding, connection-parameter and delay details.

## Session

The session is a strict request/response sequence. There is no explicit end command; the
host closes the transport.

### Handshake

On IrDA the host must send two handshake commands before anything else:

```
H→D  1B
D→H  01
H→D  1C 10 27 00 00
D→H  01
```

Any answer other than `01` is an error.

On **BLE** the handshake is not sent.

On **serial** and **USB HID** the sources disagree:

- Variant A: send the handshake as on IrDA.
- Variant B: skip it and start directly with command `10`.

Both appear to work, so a host can try variant B and fall back to A if the first command
gets no answer.

Confidence: **likely** for IrDA and BLE, **uncertain** for serial and HID.

### Identity

```
H→D  10            model code      → 1 byte
H→D  11            hardware version → 1 byte
H→D  13            software version → 1 byte, BCD (0x14 = version 14)
H→D  14            serial number   → u32le
H→D  1A            device clock    → u32le, half-second ticks
```

`11` and `13` are optional; one implementation never sends them.

## Commands

All commands are shown as their logical bytes (command byte followed by arguments), before
transport wrapping.

| Command | Logical H→D bytes | Answer | Purpose |
|---------|-------------------|--------|---------|
| Handshake 1 | `1B` | 1 byte, `01` = OK | Wake up / start session (IrDA, maybe serial/HID) |
| Handshake 2 | `1C 10 27 00 00` | 1 byte, `01` = OK | Second handshake stage |
| Model | `10` | 1 byte model code | See [Devices](#devices) |
| Hardware version | `11` | 1 byte | Meaning not documented |
| Software version | `13` | 1 byte BCD | Firmware version |
| Serial number | `14` | u32le | Shown in decimal |
| Device clock | `1A` | u32le | Half-second ticks, see [Date and time](#date-and-time) |
| Data size | `C6 <since:4> 10 27 00 00` | u32le `L` | Bytes of dive data newer than `since` |
| Data | `C4 <since:4> 10 27 00 00` | u32le `L+4`, then `L` bytes | The dive blob |

- `<since:4>` is a u32le timestamp in the same unit as the dive timestamps (half-seconds since
  2000-01-01, see [Download procedure](#download-procedure)). 0 means "everything".
- The trailing `10 27 00 00` is the constant 10000 as u32le. Its meaning is unknown; always
  send it.
- On serial the device acknowledges each host frame with `11`. The value `66` is believed to
  be the corresponding negative acknowledgement. The host never sends either.

Confidence: **likely** (both sources agree on all command bytes and arguments). The meaning
of `11` and `66` outside the serial ACK is **uncertain**.

## Framing and integrity

### IrDA

No framing. Send the logical bytes as one TinyTP write. Read the answer as a byte stream of
the expected size.

```
H→D  10
D→H  32                     (model code 0x32 = G2; illustrative)
```

### Serial

Host frame:

| Offset | Size | Value |
|--------|------|-------|
| 0 | 3 | `FF FF FF` |
| 3 | 4 | `A6 59 BD C2` (sync) |
| 7 | 4 | u32le `n` = number of logical bytes (command byte + arguments) |
| 11 | n | logical bytes |
| 11+n | 1 | XOR of bytes 7 .. 10+n (the length field and the logical bytes) |

The checksum is [xor8](../common/checksums.md#xor8) with initial value 0.

After sending, the host reads back:

1. The **echo** of the complete frame (12 + n bytes for n logical bytes). It must match
   what was sent.
2. One **ACK** byte, `11`.

Then the answer arrives as one or more device packets:

| Offset | Size | Value |
|--------|------|-------|
| 0 | 4 | u32le `m` = 1 + number of answer bytes in this packet |
| 4 | 1 | the command byte being answered |
| 5 | m − 1 | answer bytes |
| 4+m | 1 | XOR checksum |

Collect packets until the expected answer length is reached. The bulk data after `C4` is
sent as many packets of this form; 128 answer bytes per packet (`m` = 129, 134 bytes on the
wire) has been observed.

Checksum coverage of device packets — variants:

- Variant A: XOR of the length field, the command byte and the answer bytes (offsets 0 ..
  3+m).
- Variant B: the same, but also including the preceding `11` ACK byte for the first packet
  after a command.

Since `11` is a constant, the two variants differ by `0x11` in the first packet. A host that
accepts either value is robust. Confidence: **uncertain**.

Worked example, model query:

```
H→D  FF FF FF A6 59 BD C2 01 00 00 00 10 11
D→H  FF FF FF A6 59 BD C2 01 00 00 00 10 11      (echo)
D→H  11                                          (ACK)
D→H  02 00 00 00 10 32 20                        (m = 2, command 10, model 0x32; XOR = 20 under variant A)
```

The model byte 0x32 is illustrative. The XOR over `01 00 00 00 10` is `11`.

Data size request with `since` = `0x1A2B3C4D`:

```
H→D  FF FF FF A6 59 BD C2 09 00 00 00 C6 4D 3C 2B 1A 10 27 00 00 B8
```

The 9 logical bytes are `C6`, the four timestamp bytes and `10 27 00 00`. XOR of
`09 00 00 00 C6 4D 3C 2B 1A 10 27 00 00` is `B8`.

Confidence: **likely** for the host frame (both sources agree byte for byte), **uncertain**
for the device checksum coverage.

### USB HID

OUT report (32 bytes):

| Offset | Size | Value |
|--------|------|-------|
| 0 | 1 | `n` = number of logical bytes |
| 1 | n | logical bytes |
| 1+n | 31−n | `00` padding |

IN report (64 bytes):

| Offset | Size | Value |
|--------|------|-------|
| 0 | 1 | `k` = number of valid payload bytes (at most 63) |
| 1 | k | payload |
| 1+k | … | ignored |

Concatenate the payload of successive IN reports until the expected answer length is
reached. There is no checksum (USB CRCs protect each transfer).

```
H→D  01 10 00 00 … 00                     (32 bytes)
D→H  01 32 xx xx … xx                     (64 bytes; k = 1, model 0x32)
H→D  09 C6 00 00 00 00 10 27 00 00 00 … 00
D→H  04 L0 L1 L2 L3 xx …
```

Confidence: **likely**.

### BLE

Write: `n` followed by the `n` logical bytes, not padded.

```
H→D  01 10
H→D  09 C6 4D 3C 2B 1A 10 27 00 00
```

Notifications: the first byte is a header, the rest is payload. Concatenate payloads until
the expected answer length is reached. No checksum.

The header byte is interpreted in two ways:

- In older firmware, and for any reply that fits one notification, it equals the payload
  length.
- In newer firmware (a change seen between G2 firmware 1.2 and 1.4), multi-notification
  replies carry a running value in that byte instead: `F7`, `14`, `27`, `3A`, … increasing by
  19 (`0x13`) modulo 256, with `E5` followed by `F7` again. It behaves like a sequence
  counter, not a length.

A robust host therefore takes the payload length from the ATT notification length (notification
length − 1) and ignores the header byte. A host that trusts the header byte as a length will
break on newer firmware. Confidence: **likely** for the robust rule, **uncertain** for the
meaning of the sequence values.

## Memory layout

The host has no access to device memory. The device answers `C4` with a blob of all dives
newer than `since`, concatenated. The data size answer `L` gives its length, and `C4` first
returns `L + 4` as a u32le prefix, which the host must check before reading the `L` blob
bytes.

Inside the blob, each dive starts with the marker `A5 A5 5A 5A`:

| Offset | Size | Meaning |
|--------|------|---------|
| 0x00 | 4 | `A5 A5 5A 5A` |
| 0x04 | 4 | u32le total length of this dive, marker included |
| 0x08 | 4 | u32le timestamp (also the fingerprint) |
| 0x0C | … | rest of the header, then samples |

Dives are stored oldest first, so the last dive in the blob is the newest.

Confidence: **likely**.

## Download procedure

```
open transport
if handshake required: send 1B, expect 01; send 1C 10 27 00 00, expect 01
model   := cmd(10) -> 1 byte
serial  := cmd(14) -> u32le
clock   := cmd(1A) -> u32le;  host_time := now()
since   := fingerprint of newest known dive (u32le), or 0 for a full download
L       := cmd(C6, since, 10 27 00 00) -> u32le
if L == 0: done, nothing new
T       := cmd(C4, since, 10 27 00 00) -> u32le
require T == L + 4
blob    := read L more bytes
split blob into dives, newest first:
    pos := len(blob) - 4
    end := len(blob)
    while pos > 0:
        pos := pos - 1
        if blob[pos .. pos+4] == A5 A5 5A 5A:
            n := u32le(blob[pos+4 ..])
            require pos + n <= end
            emit dive blob[pos .. pos+n], fingerprint blob[pos+8 .. pos+12]
            end := pos
            pos := pos - 4 (not below 0)
```

Scanning forwards from offset 0 and skipping `n` bytes at each marker works too.

### Choosing `since`

- Preferred: use the 4-byte timestamp of the newest dive already downloaded (the fingerprint)
  as `since`. The device then returns only newer dives. Whether a dive with a timestamp equal
  to `since` is returned is not known. Hosts should drop any dive whose fingerprint they
  already have.
- From a wall-clock time `t` (seconds since 1970, UTC): `since = (t − 946684800) × 2`. One
  implementation adds 60 s to the newest known dive time before converting.
- For the Smart Pro, Smart Com, Smart Tec and Smart Z (`0x10`, `0x14`, `0x18`, `0x1C`) one
  implementation converts through the device clock instead:
  `since = clock − (host_time − t) × 2`. See the date-and-time open question.

Confidence: **likely** for the procedure, **uncertain** for the boundary behaviour.

## Dive data format

### Header

All values are little-endian. Offsets are from the start of the dive (the `A5` marker).
The header length depends on the group; samples start right after it.

Groups:

- **P**: Smart Pro (`0x10`), 92 bytes
- **C**: Smart Com (`0x14`), 100 bytes
- **Z**: Smart Tec, Smart Z (`0x18`, `0x1C`), 132 bytes
- **A**: Aladin Tec (`0x12`), 108 bytes
- **A2**: Aladin Tec 2G (`0x13`), 116 bytes
- **G**: Galileo without trimix, Aladin 2G/Tec 3G/Sport, Meridian, Mantis, Mantis 2, Chromis,
  Aladin Square (`0x11`, `0x15`, `0x20`, `0x22`, `0x24`, `0x26`), 152 bytes
- **T**: Galileo Trimix and everything newer: Aladin Matrix, A1, A2, G2, G2 TEK, G2 HUD,
  G3, Luna 2.0 (AI) (`0x17`, `0x19`, `0x25`, `0x28`, `0x31`, `0x32`, `0x34`, `0x42`, `0x50`,
  `0x51`), 84 bytes

Offsets per field (`–` = not present):

| Field | Type | P | C | Z | A | A2 | G | T |
|-------|------|---|---|---|---|----|---|---|
| Timestamp | u32le | 8 | 8 | 8 | 8 | 8 | 8 | 8 |
| UTC offset | i8 | – | – | – | 16 | 16 | 16 | 16 |
| Max depth | u16le | 18 | 18 | 18 | 22 | 22 | 22 | 22 |
| Dive time | u16le | 20 | 20 | 20 | 24 | 26 | 26 | 26 |
| Min temperature | i16le | 22 | 22 | 22 | 26 | 30 | 30 | 30 |
| Max temperature | i16le | – | – | – | 28 | 28 | 28 | 28 |
| Surface temperature | i16le | – | – | – | 32 | 32 | 32 | 32 |
| Gas mix O2 table | see below | 24 (1 mix) | 24 (1 mix) | 28 (3 mixes) | 30 (1 mix) | 34 (3 mixes) | 44 (3 mixes) | – |
| Tank pressures | see below | – | 30 | 34 | – | – | 50 | – |
| Settings | u32le | – | – | – | 52 | 60 | 92 | 68 |

Conversions:

- **Timestamp**: half-seconds since 2000-01-01 00:00:00. See [Date and time](#date-and-time).
- **UTC offset**: signed, units of 15 minutes.
- **Max depth**: raw unit is 1 mbar of hydrostatic pressure. Depth in metres =
  `raw × 100 Pa / (ρ × 10 m/s²)`, i.e. `raw / 100` m in fresh water (ρ = 1000 kg/m³) and
  `raw / 102.5` m in salt water (ρ = 1025 kg/m³). The device's own convention uses
  g = 10 m/s². Salinity comes from the settings word; groups without one are treated as
  fresh water.
- **Dive time**: minutes.
- **Temperatures**: 1/10 °C, signed.
- **Gas mix O2 table**: one entry per mix slot, the O2 percentage. Group A2 stores one
  byte per slot; all other groups store a u16le per slot. A value of 0 means the slot is
  unused. Mixes have no helium in these groups.
- **Tank pressures**: units of 1/128 bar. 0xFFFF in either value, or both values 0, means
  no data for that slot.
  - Groups C and Z: per slot `i`, begin pressure at `base + 4i` and end pressure at
    `base + 4i + 2`.
  - Group G: all end pressures first, then all begin pressures. End pressure of slot `i` at
    `base + 2i`, begin pressure at `base + 2·3 + 2i` (3 slots). Example: slot 0 end at 50,
    begin at 56.
  - Tank pressures are ignored for freedives.
- **Settings** bits:

| Bit mask | Meaning |
|----------|---------|
| `0x00000080` | Freedive (apnea) mode. Freedives also set the gauge bit, so test this first. |
| `0x00001000` | Gauge mode |
| `0x00100000` | Salt water (ρ = 1025 kg/m³); clear = fresh water (ρ = 1000 kg/m³) |

For the Aladin Tec (`0x12`) the freedive bit is not used. For the Aladin Tec 2G (`0x13`)
sources differ: one ignores freedive detection; the other reads it from bit `0x00000200`.

In group T, gas mixes and tank pressures are not in the header. They come from the
`MISC` records in the sample stream.

Further header fields reported by one source only (confidence **uncertain**):

| Field | P | C | Z | A | A2 | G | T | Conversion |
|-------|---|---|---|---|----|---|---|------------|
| Average depth (u16le) | – | – | – | – | 24 | 24 | 24 | as max depth |
| Surface interval (u16le) | 26 | 26 | 24 | 34 | 40 | 38 | 38 | minutes |
| Max ppO2 setting (u16le) | 32 | 44 | 58 | 40 | 46 | 66 | – | 1/1000 bar |
| Heart rate avg, max, min (3 × u8) | – | – | – | – | – | 34 | 34 | bpm |
| Settings word | 48 | 58 | 90 | – | – | – | – | as above |

That source also reads the group A temperature at 28 as the surface temperature, while
the other source reads 28 as the maximum and 32 as the surface temperature.

Confidence: **likely** for the main table, **uncertain** where noted.

#### Galileo trimix switch

Models `0x11` (Galileo) and `0x19` (Galileo Trimix) can store either layout. Byte 43 of the
dive selects it:

- Variant A: if `byte[43] & 0x80` is set, the dive uses group T (84-byte header); otherwise
  group G (152 bytes).
- Variant B: the test mask is `0xC0` instead of `0x80`.

The two agree whenever bit 6 is clear. The trimix layout also changes the third alarm
byte (see [Events](#events)) and the absolute-pressure encoding. At least 44 bytes must be
present to make this decision. Confidence: **uncertain** for the exact mask.

### Samples

The sample stream starts at the end of the header and runs to the end of the dive. It is a
sequence of variable-length records. Each record starts with a **type prefix** made of high
bits. The bits after the prefix in the same byte (if any) are the first value bits, and
`extra` whole bytes follow. The value is assembled most-significant-bit first: the leftover
bits of the prefix byte, then each extra byte. Unless stated otherwise, the value is a
two's-complement number with the total bit count as its width.

Some records **ignore** the leftover bits of their prefix byte (marked "x" below). Their
value is made only from the extra bytes.

Two prefix schemes exist.

#### Unary prefixes (tables U1..U4)

The record type is the number `n` of leading 1 bits, counted across byte boundaries,
terminated by a 0 bit. The prefix uses `n + 1` bits. If those bits fill whole bytes, the
value starts in the next byte; otherwise the remaining bits of the last prefix byte are the
leading value bits.

**U1 — Smart Pro (`0x10`)**, and **U2 — Aladin Tec, Aladin Tec 2G (`0x12`, `0x13`)**:

| n | Bit pattern | Extra | Record | Value bits |
|---|-------------|-------|--------|------------|
| 0 | `0ddddddd` | 0 | Depth delta | 7, signed |
| 1 | `10dddddd` | 0 | Temperature delta | 6, signed |
| 2 | `110ddddd` | 0 | Time (repeat count) | 5 |
| 3 | `1110dddd` | 0 | Alarm byte 0 | 4 |
| 4 | `11110ddd` | 1 | Depth delta | 11, signed |
| 5 | `111110dd` | 1 | Temperature delta | 10, signed |
| 6 | `1111110x` | 2 | Depth absolute | 16 |
| 7 | `11111110` | 2 | Temperature absolute | 16, signed |
| 8 | `11111111 0ddddddd` | 0 | Alarm byte 1 (U2 only) | 7 |

**U3 — Smart Com (`0x14`)**:

| n | Bit pattern | Extra | Record | Value bits |
|---|-------------|-------|--------|------------|
| 0 | `0ddddddd` | 1 | Pressure + depth delta | 7 + 8, see below |
| 1 | `10dddddd` | 0 | Remaining bottom time delta | 6, signed |
| 2 | `110ddddd` | 0 | Temperature delta | 5, signed |
| 3 | `1110dddd` | 1 | Tank pressure delta | 12, signed |
| 4 | `11110ddd` | 1 | Depth delta | 11, signed |
| 5 | `111110dd` | 1 | Temperature delta | 10, signed |
| 6 | `1111110x` | 1 | Alarm byte 0 | 8 |
| 7 | `11111110` | 1 | Time (repeat count) | 8 |
| 8 | `11111111 0xxxxxxx` | 2 | Depth absolute | 16 |
| 9 | `11111111 10xxxxxx` | 2 | Tank pressure absolute, tank 0 | 16 |
| 10 | `11111111 110xxxxx` | 2 | Temperature absolute | 16, signed |
| 11 | `11111111 1110xxxx` | 1 | Remaining bottom time absolute | 8 |

**U4 — Smart Tec, Smart Z (`0x18`, `0x1C`)**: types 0..8 as U3, then:

| n | Bit pattern | Extra | Record | Value bits |
|---|-------------|-------|--------|------------|
| 9 | `11111111 10xxxxxx` | 2 | Temperature absolute | 16, signed |
| 10 | `11111111 110xxxxx` | 2 | Tank pressure absolute, tank 0 | 16 |
| 11 | `11111111 1110xxxx` | 2 | Tank pressure absolute, tank 1 | 16 |
| 12 | `11111111 11110xxx` | 2 | Tank pressure absolute, tank 2 | 16 |
| 13 | `11111111 111110xx` | 1 | Remaining bottom time absolute | 8 |

Any longer prefix is invalid.

#### Byte prefixes (table B)

Used by groups G and T (models `0x11`, `0x15`, `0x17`, `0x19`, `0x20` and above). The type is
taken from the first byte only:

| First byte | Extra | Record | Value bits |
|------------|-------|--------|------------|
| `0ddddddd` (`00`–`7F`) | 0 | Depth delta | 7, signed |
| `100ddddd` (`80`–`9F`) | 0 | Remaining bottom time delta | 5, signed |
| `1010dddd` (`A0`–`AF`) | 0 | Tank pressure delta | 4, signed |
| `1011dddd` (`B0`–`BF`) | 0 | Temperature delta | 4, signed |
| `1100dddd` (`C0`–`CF`) | 0 | Time (repeat count) | 4 |
| `1101dddd` (`D0`–`DF`) | 0 | Heart rate delta | 4, signed |
| `1110dddd` (`E0`–`EF`) | 0 | Alarm byte 0 | 4 |
| `F0` | 1 | Alarm byte 1 | 8 |
| `F1` | 2 | Depth absolute | 16 |
| `F2` | 1 | Remaining bottom time absolute | 8 |
| `F3` | 2 | Temperature absolute | 16, signed |
| `F4` | 2 | Tank pressure absolute, tank 0 (see below) | 16 |
| `F5` | 2 | Tank pressure absolute, tank 1 | 16 |
| `F6` | 2 | Tank pressure absolute, tank 2 | 16 |
| `F7` | 1 | Heart rate absolute | 8 |
| `F8` | 2 | Bearing | 16 |
| `F9` | 1 | Alarm byte 2 | 8 |
| `FA` | 8 (raw) | Apnea block | – |
| `FB` | 1 + (len − 1) | Extended (MISC) record | – |
| `FC`–`FF` | – | invalid | – |

The type number for the byte-prefix scheme can be computed as: 0 for `0xxxxxxx`, 1 for
`100xxxxx`, bits 6..4 for other `1xxxxxxx` bytes below `F0`, and `(byte & 0x0F) + 7` for
`F0`..`FF`.

#### Record semantics

The decoder keeps running state: current depth, temperature, tank pressure and active tank,
remaining bottom time (initial value 99), heart rate, active gas mix, and the alarm bytes.
Delta records add to the state; absolute records replace it.

| Record | Raw unit and conversion |
|--------|-------------------------|
| Depth | 2 mbar per unit. Depth = `(raw − raw₀) × 200 Pa / (ρ × 10 m/s²)`, i.e. `(raw − raw₀) / 50` m in fresh water. `raw₀` is the first absolute depth value in the dive (the surface reference); deltas before it start from 0. |
| Pressure + depth delta (U3/U4 type 0) | The 15 value bits split into a pressure part (the 7 bits from the prefix byte, signed) and a depth part (the extra byte, signed 8-bit). Each is added to the corresponding state. |
| Temperature | 0.4 °C per unit (°C = raw / 2.5), signed. |
| Tank pressure | 0.25 bar per unit (bar = raw / 4). For absolute records the tank is the one named in the table. In group T and trimix Galileo dives, absolute values instead carry the tank number in bits 15..12 and the pressure in bits 11..0. An absolute pressure record also makes that tank's gas mix the active one. |
| Remaining bottom time | minutes |
| Heart rate | beats per minute |
| Bearing | compass heading; unit not confirmed (probably degrees). Reported once, at the next sample. |
| Alarm bytes | Bit fields, see [Events](#events). |
| Time | Repeat count `k`: emit `k` samples with the current state. |
| Apnea block | 8 bytes follow the `FA` byte. Not decoded; one source reads maximum, minimum and average heart rate (bpm) from its last three bytes. |
| MISC | See below. |

**Sample emission.** The sample interval is 4 s, or 1 s when the settings word marks a
freedive. The first sample is at time 0.

- Every depth record (delta or absolute, including the combined pressure + depth record)
  completes **one** sample: it is emitted with the current state, then time advances by one
  interval.
- A time record with value `k` emits `k` samples with unchanged state.
- All other records only update the state; their values appear in the next emitted sample.

**MISC records** (`FB`): the byte after `FB` is a length `len` (at least 1); `len − 1` more
bytes follow. The first of those is a subtype.

| Subtype | Contents (offsets from the subtype byte) | Confidence |
|---------|------------------------------------------|------------|
| `0x20`–`0x29` | Gas mix slot `subtype − 0x20` (0..9): +1 u16le O2 %, +3 u16le He %, +5 u16le begin pressure (1/128 bar), +7 u16le end pressure (1/128 bar). Requires `len ≥ 16`. Mix is unused if O2 and He are both 0; tank data absent if 0xFFFF or both 0. | likely |
| `0x20` only | +9 u16le maximum ppO2, 1/1000 bar | uncertain |
| `0x02` | Skin temperature, `35 − (b & 0x7F)` °C from a following byte | uncertain |
| `0x14` | Freedive sub-dive summary: +3 temperature, +5 depth, +7 duration in s | uncertain |
| `0x12`, `0x17`, `0x18` | Present, meaning unknown | uncertain |

Unknown subtypes must be skipped using `len`. Gas mixes are identified by slot number; the
active-mix values in alarm bytes and absolute pressure records refer to these slot numbers.

#### Events

Alarm bytes are decoded as bit fields. Bits not listed are unknown.

Smart Pro, Smart Com, Smart Tec, Smart Z (alarm byte 0 only):

| Bits | Meaning |
|------|---------|
| 0 | Warning (yellow) |
| 1 | Alarm (red) |
| 2 | Workload warning |
| 3–5 | Workload level |
| 6–7 | unknown |

Aladin Tec (`0x12`) and Aladin Tec 2G (`0x13`):

| Byte | Bits | Meaning |
|------|------|---------|
| 0 | 0 | Warning |
| 0 | 1 | Alarm |
| 0 | 2 | Bookmark (or safety-stop timer started) |
| 0 | 3 | unknown |
| 1 | 0–6 | unknown (Aladin Tec) |
| 1 | 0–2 | unknown (Aladin Tec 2G) |
| 1 | 3–4 | Active gas mix slot (Aladin Tec 2G) |

Byte-prefix models (groups G and T):

| Byte | Bits | Meaning |
|------|------|---------|
| 0 | 0 | Warning |
| 0 | 1 | Alarm |
| 0 | 2 | Workload warning |
| 0 | 3 | Bookmark |
| 1 | 0–2 | Workload level |
| 1 | 3–4 | unknown |
| 1 | 5–6 | Active gas mix slot |
| 1 | 7 | unknown |
| 2 | 0–7 | unknown (group G) |
| 2 | 0–3 | unknown (group T, trimix Galileo) |
| 2 | 4–7 | Active gas mix slot (group T, trimix Galileo) |

Confidence: **likely** for warning/alarm/bookmark/gas bits, **uncertain** for workload.

#### Worked example (table B)

Stream bytes after a fresh-water group G header:

```
F3 00 3C   temperature absolute = 60 → 24.0 °C
F1 01 2C   depth absolute = 300 → surface reference; emits t=0 s, 0.00 m, 24.0 °C
19         depth delta +25 → emits t=4 s, 0.50 m
7F         depth delta −1  → emits t=8 s, 0.48 m
C2         time, repeat 2   → emits t=12 s and t=16 s, 0.48 m
BF         temperature delta −1 → 59 (no sample)
19         depth delta +25 → emits t=20 s, 0.98 m, 23.6 °C
```

#### Worked example (table U1, Smart Pro)

```
FE 00 3C   n=7: temperature absolute = 60 → 24.0 °C
FC 01 2C   n=6: depth absolute (low prefix bit ignored) = 300 → reference; emits t=0 s, 0.00 m
05         n=0: depth delta +5 → emits t=4 s, 0.10 m
C2         n=2: time, repeat 2 → emits t=8 s and t=12 s, 0.10 m
```

Confidence: **likely**.

### Date and time

The dive timestamp at offset 8 and the device clock (command `1A`) count **half-seconds**.

- Models with a UTC-offset byte (groups A, A2, G, T): the dive's local time is
  `2000-01-01 00:00:00 + timestamp / 2 s + offset × 15 min`, and the time zone of the dive is
  `offset × 15 min`. The timestamp is therefore UTC.
- Smart Pro, Smart Com, Smart Tec, Smart Z (no offset byte) — variants:
  - Variant A: the timestamp is absolute, counted from 2000-01-01 like the newer models;
    interpret it in the host's local time zone.
  - Variant B: the timestamp is only meaningful relative to the device clock. Read the clock
    with `1A` and note the host time; the dive time is
    `host_time − (clock − timestamp) / 2 s`.

  If the device clock was set correctly, both give the same result. Confidence: **uncertain**.

## Model-specific variations

- The model byte selects the header group and sample table. Codes not listed above are
  unknown. One implementation falls back to group A2 (116-byte header, table U2); another
  rejects them.
- Groups T and G differ in where gas mixes and tank pressures come from (MISC records vs.
  header).
- Galileo (`0x11`/`0x19`) switches between G and T per dive.
- Only the Aladin Tec 2G uses one-byte O2 entries.
- Subgear-branded units report Uwatec codes.

## Open questions

1. Is the IrDA-style handshake (`1B`, `1C …`) required, harmful or ignored on serial and USB
   HID?
2. Which bytes does the serial device checksum cover — does the first packet after a command
   include the `11` ACK byte?
3. Does `since` select dives strictly newer than the timestamp, or newer-or-equal?
4. Meaning of the `10 27 00 00` (10000) argument of `1C`, `C4` and `C6`.
5. Is the Smart Pro/Com/Tec/Z timestamp absolute or only relative to the device clock?
6. Galileo trimix switch: mask `0x80` or `0xC0` at byte 43?
7. Aladin Tec 2G freedive flag: absent, or bit `0x200`?
8. BLE notification header values in new firmware: is it a sequence counter, and does it ever
   indicate packet loss?
9. Model codes of the Aladin Tec 3H, Subgear XP-H and XP-3H; USB VID:PID of the G3.
10. Exact layout of the apnea block and of MISC subtypes other than `0x20`–`0x29`.
11. Units of the bearing record.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual
  reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>, used
  as a factual reference; no code copied.
- Bluetooth SIG Assigned Numbers (GAP characteristic `2A04`),
  <https://www.bluetooth.com/specifications/assigned-numbers/>.
- Infrared Data Association, IrLMP and Tiny TP specifications (for LSAP and TinyTP terms).
