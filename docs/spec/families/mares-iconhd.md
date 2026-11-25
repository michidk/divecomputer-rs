# Mares Icon HD family (binary command protocol, memory dump)

Covers Mares computers from the Icon HD (2010) through the Quad, Quad Air, Smart and Smart
Air. They speak a binary command protocol over a serial link (USB cable, USB CDC on the
Icon HD) or over Bluetooth Low Energy through the Mares "BlueLink Pro" adapter. Every
command is two bytes, `cmd` followed by `cmd XOR A5`; every response is framed by an
`AA` lead byte and an `EA` trailer. Dives are read from a flash memory ring buffer with a
"read memory" command and walked backwards from an end pointer.

The newer Genius-generation computers use the same framing but read dives as objects; see
[mares-genius.md](mares-genius.md). Older Nemo/Puck/Darwin computers use a different
ASCII protocol; see [mares-nemo-puck-darwin.md](mares-nemo-puck-darwin.md).

## Devices

The model code is not read from memory; it is derived from the product name in the
version response (see [Session](#session)). The numbers below are the values a host
conventionally uses; the Genius-generation values also match what those devices report in
their model object.

| Vendor | Model | Product name in version reply | Model code | Transports | Notes |
|--------|-------|-------------------------------|------------|------------|-------|
| Mares | Matrix | `Matrix` | 0x0F (15) | serial | |
| Mares | Smart | `Smart` | 0x10 (16) | serial, BLE (BlueLink Pro) | |
| Mares | Smart Apnea | `Smart Apnea` | 0x10 (16); hosts distinguish it as 0x010010 | serial, BLE (BlueLink Pro) | |
| Mares | Icon HD, Icon HD Black Edition | `Icon HD` | 0x14 (20) | USB (CDC) | |
| Mares | Icon HD Net Ready | `Icon AIR` | 0x15 (21) | USB (CDC) | air integrated |
| Mares | Puck Pro, Puck Pro + | `Puck Pro` | 0x18 (24) | serial, BLE (BlueLink Pro) | |
| Mares | Nemo Wide 2 | `Nemo Wide 2` | 0x19 (25) | serial | |
| Mares | Puck 2, Puck Wrist 2 | `Puck 2` | 0x1F (31) | serial | |
| Mares | Quad Air | `Quad Air` | 0x23 (35) | serial, BLE (BlueLink Pro) | air integrated |
| Mares | Smart Air | `Smart Air` | 0x24 (36) | serial, BLE (BlueLink Pro) | air integrated |
| Mares | Quad | `Quad` | 0x29 (41) | serial, BLE (BlueLink Pro) | flash size query |

Genius-generation codes (0x1C, 0x2C, 0x2D, 0x2F, 0x31, 0x32, 0x33, 0x35) are listed in
[mares-genius.md](mares-genius.md#devices).

Identification:

- **USB CDC (Icon HD):** one source reports VID:PID `FFFF:0005` for the Icon HD's built-in
  CDC-ACM interface (and `2047:082B` as a second ID of unknown owner).
- **USB-serial cable:** the Mares USB interface cable is a Silicon Labs CP210x bridge
  (`10C4:EA60`) according to one source; see `../transports/`.
- **BLE:** the BlueLink Pro adapter advertises a name starting with `Mares bluelink pro`.

Confidence: **likely** (names, codes); USB IDs **uncertain** (one source).

## Transport parameters

### Serial / USB

| Parameter | Value |
|-----------|-------|
| Baud | 115200 |
| Framing | 8 data bits, **even parity**, 1 stop bit, no flow control |
| DTR, RTS | both cleared (0) |
| Read timeout | 3000 ms (one source: 1000 ms) |
| After open | purge input and output |

Icon HD over USB CDC: one source configures the CDC line coding to **256000 baud, 8N1**
for `FFFF:0005` (115200 otherwise), selects USB configuration 2, and pulses
SET_CONTROL_LINE_STATE `0x0002` (RTS on, DTR off) for 20 ms, then `0x0000`, before the
first command. The reference implementation instead opens the CDC port as a normal
serial port at 115200 8E1. A CDC device usually ignores the line coding, so both are
likely to work; see open questions.

### BLE

<a id="ble"></a>

The BlueLink Pro adapter (and the Genius-generation computers) expose one vendor service
that carries the serial byte stream:

| Item | UUID |
|------|------|
| Service | `544e326b-5b72-c6b0-1c46-41c1bc448118` |
| Notify characteristic (D→H) | `1d1aae28-d2a8-91a1-1242-9d2973fbe571` |
| Write characteristic (H→D) | `99a91ebd-b21f-1689-bb43-681f1f55e966` |
| Optional characteristic | `a87988b9-694c-479c-900e-95dfa6c00a24` — one host writes `01` once (write with response) before the first command and waits 100 ms; purpose unknown |

- Enable notifications on the notify characteristic (CCCD `01 00`).
- Writes of up to **20 bytes** each, write-with-response, waiting for each write to
  complete. No MTU exchange is required.
- Notifications arrive in packets of up to 244 bytes (MTU-dependent); the host
  reassembles them into a byte stream.
- The serial line parameters do not apply.
- One host removes any existing bond with the adapter before connecting. Whether
  pairing is needed is not known.

Packet size for memory reads over BlueLink Pro: see [Download procedure](#download-procedure).

Confidence: **likely** for UUIDs, write size and procedure (one source each,
consistent with the reference implementation's 20-byte write split and 244-byte read
packets); **uncertain** for the optional characteristic.

## Session

1. Open the transport, configure it as above, purge.
2. **Version:** send `C2 67`. The device replies `AA`, **140 bytes**, `EA`.
   - Bytes `0x46`–`0x55` of the 140-byte payload hold the product name, ASCII,
     NUL-padded to 16 bytes. Compare the whole 16-byte field with the names in the
     [Devices](#devices) table (an exact match, so `Quad` and `Quad Air` do not collide).
   - The rest of the payload (firmware and hardware information) is not decoded.
3. **Quad only, flash size:** send `B3 16`. The device replies `AA`, u32le flash size in
   bytes, `EA`. If the size exceeds `0x40000` the 1 MiB layout is used, otherwise the
   256 KiB layout. A failure here is not fatal (assume 256 KiB).
4. One source sends the version command twice to the Matrix and discards the first reply.

There is no explicit logout; the session ends when the link closes.

Confidence: **likely**.

## Commands

All commands start with the two-byte header `cmd`, `cmd XOR A5`.

| Command | Header bytes | Payload (H→D) | Response payload (between `AA` and `EA`) | Notes |
|---------|--------------|---------------|------------------------------------------|-------|
| Version | `C2 67` | none | 140 bytes | product name at `0x46` |
| Flash size | `B3 16` | none | 4 bytes, u32le | Quad |
| Read memory | `E7 42` | 8 bytes: u32le address, u32le length | `length` bytes | |
| Set time | `B0 15` | 4 bytes: u32le local time as seconds since 1970-01-01 00:00 (local clock treated as UTC) | none (`AA EA`) | optional |
| Object init | `BF 1A` | 16 bytes | 16 bytes | Genius generation, see [mares-genius.md](mares-genius.md) |
| Object segment (toggle 0) | `AC 09` | none | variable | Genius generation |
| Object segment (toggle 1) | `FE 5B` | none | variable | Genius generation |

Set-time example: 2024-01-01 12:00:00 local → `1704110400` = `0x6592A940` →
`B0 15 40 A9 92 65`.

Confidence: **likely** (Set time: one source only).

## Framing and integrity

There is no length field, checksum or CRC; integrity relies on the lead and trailer
bytes and on known response lengths.

### Serial order

```
H→D  cmd  cmd^A5
D→H  AA                       (ACK; the host skips any other bytes until AA)
H→D  <payload>                (only if the command has one)
D→H  <response payload, known length>
D→H  EA                       (END)
```

On serial the payload is sent **after** the `AA`. A trailer other than `EA` is a
protocol error.

### BLE order (BlueLink Pro, Genius)

The payload is sent immediately after the two header bytes, without waiting for `AA`;
the response is then `AA <payload> EA` as on serial. (The Genius-generation "variable"
BLE mode differs slightly, see [mares-genius.md](mares-genius.md#ble-variable-mode).)

### Retries

On a timeout or framing error, wait ~1 s, discard input and repeat the whole command,
up to 4 retries. (Another host waits 30 ms and allows 5 to 20 attempts.)

### Worked example

Read 256 bytes at `0x0000A000`:

```
H→D  E7 42
D→H  AA
H→D  00 A0 00 00 00 01 00 00
D→H  <256 bytes>
D→H  EA
```

Confidence: **likely**.

## Memory layout

| Address | Size | Meaning |
|---------|------|---------|
| `0x000C` | u32le | serial number (one source: at `0x0004` for Quad, Quad Air, Smart, Smart Apnea, Smart Air) |
| `0x2001` | u32le | end-of-profile pointer (address just past the newest dive) |
| `0x3001` | u32le | fallback end-of-profile pointer, used when the first is out of range |

If neither pointer is inside the profile ring and the first one read is `FFFFFFFF`, the
log is empty.

| Layout | Models | Memory size | Profile ring `[begin, end)` | Read packet (serial) |
|--------|--------|-------------|-----------------------------|----------------------|
| A | Icon HD, unknown names | `0x100000` | `0x00A000`–`0x100000` | 4096 |
| B | Icon HD Net Ready | `0x100000` | `0x00E000`–`0x100000` | 4096 |
| C | Quad Air, Smart Air | `0x100000` | `0x00E000`–`0x100000` | 256 |
| D | Matrix | `0x40000` | `0x0A000`–`0x3E000` | 256 |
| E | Puck Pro, Puck 2, Nemo Wide 2, Smart, Smart Apnea | `0x40000` | `0x0A000`–`0x40000` | 256 |
| E or A | Quad | flash ≤ `0x40000`: E; larger: A (ring end `0x100000`) | | 256 |

One source places the Smart Air ring at `0xA000`; see open questions.

Over BLE (BlueLink Pro) the read packet SHOULD be limited to 128 bytes; larger reads
are reported to fail through the adapter. (One host uses 128 only for the Puck Pro and
256 otherwise.)

Each dive in the ring is stored contiguously (wrapping at the ring end):

```
[u32le total length] [samples ...] [header]
```

The header ends where the next newer dive starts. For all models except the Smart
family, the header **begins** with u16le type and u16le sample count. For the Smart
family the header **ends** with u16le sample count followed by u16le type (Smart Apnea:
u16le count, u16le type, 2 more bytes).

Confidence: **likely**.

## Download procedure

```
read serial number (4 bytes at 0x000C)
eop = u32le at 0x2001; if not in ring: eop = u32le at 0x3001
if eop not in ring: if eop == FFFFFFFF -> no dives, else error
pos = eop       (reading backwards, wrapping at ring begin -> ring end)
remaining = ring size
loop:
    read the last K bytes before pos, where K = 0x5C (default), 0x80 (Icon HD Net),
        0x84 (Quad Air), 4 (Smart, Smart Air), 6 (Smart Apnea)
    from these get type and n (positions per the layout rule above)
    stop if type == FFFF or n == FFFF
    mode = type AND 3        (0 air, 1 gauge, 2 nitrox, 3 freedive)
    choose header size H, sample size S and fingerprint offset F (table below)
    total = 4 + H + n*S
        + (n div 4) * 8          for Icon HD Net, Quad Air, Smart Air (non-freedive)
        + divetime * rate * 2    for Smart Apnea (see its header)
    stop if total > remaining
    read the dive's bytes; the u32le at its start MUST equal total, otherwise stop
    fingerprint = 10 bytes at (dive start + total - H + F)
    stop if fingerprint equals the last known one
    emit the dive (total bytes); pos -= total; remaining -= total
```

| Model / mode | H | S | F |
|--------------|---|---|---|
| default (Icon HD, Matrix, Puck Pro, Puck 2, Nemo Wide 2, Quad) | 0x5C (92) | 8 | 6 |
| Icon HD Net Ready | 0x80 (128) | 12 | 6 |
| Quad Air | 0x84 (132) | 12 | 6 |
| Smart, modes 0–2 | 0x5C | 8 | 2 |
| Smart, freedive (mode 3) | 0x2E (46) | 6 | 0x20 |
| Smart Air, modes 0–2 | 0x84 | 12 | 2 |
| Smart Air, freedive | 0x30 (48) | 6 | 0x22 |
| Smart Apnea | 0x50 (80) | 14 | 0x40 |

The 10-byte fingerprint is the dive's date/time block. Reading in packet-aligned blocks
from the ring end downward is efficient; a host may also dump the whole memory with
repeated read commands and walk it offline.

One source instead compares the header date with the time of the last download and
stops at the first older dive; that is equivalent in practice.

Confidence: **likely**.

## Dive data format

Notation: `h` = offset of the first header byte (dive start + total − H). For the
non-Smart models the fields are listed at `h + 4 + x`, i.e. after the type/count words;
for the Smart family directly at `h + x`. The tables give the absolute `h`-relative
offset.

### Header, Icon HD / Matrix / Puck Pro / Puck 2 / Nemo Wide 2 / Quad / Icon HD Net / Quad Air

| Offset from h | Type | Raw unit | Meaning |
|---------------|------|----------|---------|
| 0x00 | u16le | | type; bits 0–1 dive mode: 0 air, 1 gauge, 2 nitrox, 3 freedive |
| 0x02 | u16le | | number of samples `n` |
| 0x04 | u16le | 0.1 m | maximum depth |
| 0x06 | u16le | | hour |
| 0x08 | u16le | | minute |
| 0x0A | u16le | | day |
| 0x0C | u16le | | month, **0-based** (0 = January) |
| 0x0E | u16le | | year − 1900 |
| 0x10 | u16le | | settings (below) |
| 0x14 + 4i | u8 | % | gas i O2 (i = 0..2) |
| 0x15 + 4i | u8 | | bit 7 set = gas i disabled; enabled gases come first |
| 0x16 | u16le | 0.001 bar | maximum ppO2 setting (one source only) |
| 0x26 | u16le | 1/8 mbar | atmospheric pressure; bar = raw / 8000 |
| 0x2A | u16le | 0.1 m | average depth |
| 0x30 | u16le | | surface interval (one source only; unit unknown) |
| 0x46 | i16le | 0.1 °C | minimum temperature |
| 0x48 | i16le | 0.1 °C | maximum temperature |
| 0x4E | u16le | | bit 4: dive required decompression (one source only) |
| T + 4i | u16le | 0.01 bar | tank i start pressure (air-integrated models) |
| T + 2 + 4i | u16le | 0.01 bar | tank i end pressure |
| T + 12 + 8i | u16le | L or cu ft | tank i volume |
| T + 14 + 8i | u16le | bar or psi | tank i working pressure |

`T` = 0x5C for Icon HD Net Ready, 0x60 for Quad Air; up to 3 tanks; a tank whose start
pressure is 0 and end pressure is 0 or 36000 ends the list.

Settings word:

| Bits | Meaning |
|------|---------|
| 0–1 | altitude class 0..3 (one source only) |
| 4 | 1 = fresh water, 0 = salt water (one source: bits 4–5 on Quad) |
| 8 | 1 = metric tank units (litres, bar); 0 = imperial (cu ft, psi) |
| 10–11 | sample interval index: 0 → 1 s, 1 → 5 s, 2 → 10 s, 3 → 20 s |

Gas mixes: air mode → one gas, 21 % O2; nitrox mode → the enabled gases; gauge and
freedive → none.

Dive time = `n` × interval − 180 s: the profile includes 3 minutes of surface samples
after the dive.

Tank volume conversion, imperial: water volume in litres =
`raw × 28.316846592 / (wp × 0.0680460)` with `wp` in psi
(0.0680460 = psi→atm), working pressure in bar = `wp × 0.06894757`. A zero working
pressure with imperial units is invalid.

### Header, Smart and Smart Air (modes 0–2)

| Offset from h | Type | Meaning |
|---------------|------|---------|
| 0x00 | u16le | maximum depth, 0.1 m |
| 0x02 … 0x0A | 5 × u16le | hour, minute, day, month (0-based), year − 1900 |
| 0x0C | u16le | settings (as above) |
| 0x10 + 4i | u8, u8 | gas i O2 % and enable flags (as above). One source masks the O2 field as u16le & 0x7FFF and forces 21 % in air mode |
| 0x22 | u16le | atmospheric, 1/8 mbar |
| 0x26 | u16le | average depth, 0.1 m |
| 0x2C | u16le | surface interval (one source only) |
| 0x42 | i16le | minimum temperature, 0.1 °C |
| 0x44 | i16le | maximum temperature, 0.1 °C |
| 0x5C … | | Smart Air: tank block at T = 0x5C (layout as above) |
| H − 4 | u16le | number of samples `n` |
| H − 2 | u16le | type (bits 0–1 mode; one source lists `F0` air, `F1` gauge, `F2` nitrox, `F3` freedive) |

### Header, Smart / Smart Air freedive session (mode 3)

One record holds a whole session; each "sample" summarises one freedive.

| Offset from h (Smart / Smart Air) | Type | Meaning |
|-----------------------------------|------|---------|
| 0x08 / 0x08 | u16le | settings; bit 4 fresh water |
| 0x0C / 0x0E | u16le | total dive time, s |
| 0x18 / 0x1A | u16le | atmospheric, 0.001 bar (mbar) |
| 0x1A / 0x1C | u16le | maximum depth, 0.1 m |
| 0x1C / 0x20 | i16le | minimum temperature, 0.1 °C |
| 0x1E / 0x1E | i16le | maximum temperature, 0.1 °C |
| 0x20 / 0x22 | 5 × u16le | hour, minute, day, month (0-based), year − 1900 |
| H − 4, H − 2 | u16le, u16le | number of freedives, type |

Per-freedive record (6 bytes): u16le maximum depth (0.1 m), u16le dive time (s), u16le
surface time before the dive (s).

### Header, Smart Apnea (H = 0x50)

| Offset from h | Type | Meaning |
|---------------|------|---------|
| 0x1C | u16le | settings: bits 0–5 salinity (0 = fresh; otherwise density = 1000 + value kg/m³); bits 9–10 sample-rate index `k` (rate = 2^k samples per second). One source reads the salinity bits from the u16le at 0x1E instead |
| 0x24 | u32le | total dive time, s |
| 0x38 | u16le | atmospheric, mbar |
| 0x3A | u16le | maximum depth, 0.1 m |
| 0x3C | i16le | maximum temperature, 0.1 °C |
| 0x3E | i16le | minimum temperature, 0.1 °C |
| 0x40 | 5 × u16le | hour, minute, day, month (0-based), year − 1900 |
| H − 6 | u16le | number of freedives |
| H − 4 | u16le | type |

Profile: for each freedive, a 14-byte summary (u16le maximum depth 0.1 m, u16le dive time
s, u16le surface time before the dive s; one source adds i16le temperatures at +6 and +8
in 0.1 °C; remaining bytes unknown), immediately followed by `dive time × rate` u16le
depth samples in 0.1 m at intervals of `1/rate` s. One source masks depth with `0x07FF`.

### Samples (Icon HD style, modes 0–2)

Samples start at dive start + 4; first sample at time = interval.

| Bytes | Type | Meaning |
|-------|------|---------|
| +0 | u16le | depth, 0.1 m (one source masks with `0x07FF`) |
| +2 | u16le | bits 0–11 temperature, 0.1 °C; bits 12–15 active gas index |
| +4 … | | unknown (one source reads flags from the u16le at +6, bit 9 an unnamed flag, bits 0/1/3 alarms; uncertain) |

Sample size is 8 bytes, or 12 bytes on Icon HD Net Ready, Quad Air and Smart Air.

On Icon HD Net Ready, Quad Air and Smart Air an extra 8-byte record follows every 4th
sample: u16le tank pressure (0.01 bar) of the tank matching the current gas index at +0;
other bytes unknown.

The gas index MUST be smaller than the number of enabled gases when gases are defined.

### Date and time

Local time, no time zone, minute resolution, month 0-based, year − 1900, each field a
u16le.

Confidence: **likely** for fields used by the reference implementation; fields marked
"one source only" are **uncertain**.

## Model-specific variations

- Quad: memory size read with the flash-size command.
- Matrix: ring ends at `0x3E000`.
- Icon HD / Icon HD Net Ready: 4096-byte read packets over USB.
- Smart family: header-end type/count, freedive and apnea record formats.

## Open questions

1. Icon HD over USB CDC: does the device need 256000 baud line coding and the RTS pulse,
   or is the reference implementation's 115200 8E1 enough?
2. Serial number location for Quad, Quad Air and Smart family: `0x000C` or `0x0004`?
3. Smart Air profile ring start: `0xE000` or `0xA000`?
4. Sample bytes +4…+7 (and +8…+11 on 12-byte samples): flags, alarms, NDL?
5. BLE read packet limit for BlueLink Pro: 128 bytes for all models, or 256 for all but
   the Puck Pro?
6. Purpose of characteristic `a87988b9-…` and whether bonding is needed.
7. Meaning of the remaining 140-byte version payload (firmware version location).

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used
  as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1), <https://github.com/subsurface/libdc>,
  used as a factual reference; no code copied.
