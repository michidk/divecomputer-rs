# Halcyon Symbios

The Halcyon Symbios system has a handset dive computer and a heads-up display (HUD). Both
talk Bluetooth Low Energy. The host sends one-byte commands, with an optional payload
protected by a CRC-8. Responses carry the command with bit 7 set, an ACK/NAK byte and a
CRC-8. Large objects (the logbook and each dive log) come as a stream of numbered blocks of
up to 200 bytes. The host acknowledges each block with ACK and asks for a resend with NAK.
A dive log is a sequence of type-length records.

## Devices

| Vendor | Model | Model code | Transports |
|--------|-------|------------|------------|
| Halcyon | Symbios HUD | 1 | BLE |
| Halcyon | Symbios Handset | 7 | BLE |

The model code is byte 5 of the status response and byte 2 of the dive header record.

BLE advertised name, two forms:

- starts with `H` followed by the two-digit model code (`H01…`, `H07…`), case-insensitive;
  or
- at least 10 characters, all decimal digits, with characters 4..5 equal to the two-digit
  model code (for example `xxxx07xxxx` for a handset).

GATT (public open-source BLE table):

| Role | UUID |
|------|------|
| Service | `00000001-8c3b-4f2c-a59e-8c08224f3253` |
| Device → host (notify) | `00000101-8c3b-4f2c-a59e-8c08224f3253` |
| Host → device (write) | `00000201-8c3b-4f2c-a59e-8c08224f3253` |

The device uses a random (private) BLE address.

Confidence: **likely**. The characteristic roles come from one source, which excludes
`…0101` as a write target and `…0201` as a notify source.

## Transport parameters

- One protocol packet per BLE write or notification. Packets are at most 1 + 256 + 1 bytes
  (host) or 2 + 256 + 1 bytes (device). A 200-byte data block needs 205 bytes, so the link
  needs an ATT MTU of at least 208.
- Response timeout: 5000 ms. One reference implementation uses 3000 ms, and a later fork of
  it raised the value to 5000 ms.
- Flush input after connecting.

Confidence: **likely**.

## Session

No handshake. Known hosts first send **get status** (`01`). The response payload (20 to 36
bytes) is:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u32le | serial number |
| 0x04 | u8 | hardware version |
| 0x05 | u8 | model code |
| 0x06 | u8 | Bluetooth firmware major |
| 0x07 | u8 | Bluetooth firmware minor |
| 0x08 | u16le | battery |
| 0x0A | u16le | pressure |
| 0x0C | u32le | error bits |
| 0x10 | 3 × u8 | firmware major, minor, patch |
| 0x13.. | | not decoded |

Confidence: **likely**.

## Commands

| Cmd | Function | Request payload | Response payload |
|-----|----------|-----------------|------------------|
| 01 | get status | — | 20-36 bytes, see above |
| 02 | get settings | — | (not used by known hosts) |
| 03 | set settings | — | (not used by known hosts) |
| 04 | request logbook | — | `u32le` total logbook length; data follows as `08` blocks |
| 05 | request dive log | `u16le` dive id (from the logbook) | `u32le` total length; data follows as `09` blocks |
| 07 | set time | year−2000, month, day, hour, minute, second | empty |
| 08 | logbook data block | (host sends bare `08` to start) | block, see below |
| 09 | dive log data block | (host sends bare `09` to start) | block, see below |
| 06 | ACK (host → device) | — | next block, or nothing after the last block |
| 15 | NAK (host → device) | — | resend of the current block |

NAK error codes (byte 2 of a NAK response):

| Code | Meaning |
|------|---------|
| 0 | CRC error |
| 1 | boundary error |
| 2 | wrong command length |
| 3 | unknown command |
| 4 | timeout |
| 5 | file not found / empty. For request logbook this means "no dives". For request dive log the host skips that dive |
| 6 | unknown error |

Confidence: **likely**. The settings commands are known by number only.

## Framing and integrity

Host → device:

```
cmd                                    (no payload: a single byte, no CRC)
cmd  payload[n]  crc8(payload)         (n ≥ 1)
```

The CRC covers the payload only, not the command byte.

Device → host:

```
(cmd | 0x80)  06  payload[n]  crc8(bytes 1 .. 1+n)      ACK response
(cmd | 0x80)  15  errcode     crc8(bytes 1 .. 2)        NAK response (always 4 bytes)
```

The device CRC covers everything after the command byte: the ACK/NAK byte and the payload.
A response has at least 3 bytes.

CRC: **CRC-8/SMBUS**: width 8, poly 0x07, init 0x00, refin false, refout false, xorout 0x00.
Check value for `"123456789"` = `0xF4`. See
[../common/checksums.md#crc-8-smbus](../common/checksums.md#crc-8-smbus).

### Block transfer

After a request (`04` or `05`) the device answers with ACK and a 4-byte `u32le` total
length. Then:

1. The host sends the bare block command (`08` or `09`).
2. The device sends a block response: `(block cmd | 0x80) 06 <u16le id> <data…> crc8`.
   - `id` bits 0-14: sequence number. It starts at 1 and increments by 1 per block,
     wrapping at 0x7FFF.
   - `id` bit 15: set on the last block.
   - Up to 200 data bytes per block.
3. After a good block the host sends `06` (ACK). After the last block the ACK finishes the
   transfer.
4. On a CRC error or a timeout the host sends `15` (NAK), and the device repeats the block.
   Known hosts retry up to 3 times. On a timeout, wait briefly (about 300 ms) and flush input
   before sending NAK, so that a late copy of the block is not taken as the resend.
5. The total data length MUST equal the announced length.

### Worked examples

```
H→D  07 18 03 0F 0A 14 1E 7B     set time 2024-03-15 10:20:30; CRC-8 of the 6 bytes = 0x7B
D→H  87 06 12                    ACK, empty payload; CRC-8(06) = 0x12
H→D  05 05 00 41                 request dive log id 5 (u16le 0x0005), CRC-8(05 00) = 0x41
D→H  85 06 L0 L1 L2 L3 <crc>     total length
H→D  09                          start block stream
D→H  89 06 01 00 <≤200 bytes> <crc>
D→H  84 15 05 0D                 example NAK to request logbook: error 5 (no dives), CRC-8(15 05) = 0x0D
H→D  06                          ACK …
```

Confidence: **likely**.

## Memory layout

No raw memory access. The logbook is a flat array of 32-byte entries, oldest first. Dive
logs are fetched by dive id.

Logbook entry (32 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x10 | u16le | dive id (argument for command 05) |
| 0x14 | 4 bytes | fingerprint |
| other | | not decoded |

## Download procedure

```
status = get_status()
logbook = block_download(04, 08)       # NAK code 5 → no dives, done
entries = split logbook into 32-byte entries
walk entries from the LAST to the first:
    if entry[0x14..0x17] == known_fingerprint: stop
    log = block_download(05 with entry[0x10..0x11], 09)
          # NAK code 5 → skip this entry
    hand log to the parser with fingerprint entry[0x14..0x17]
```

Confidence: **likely**.

## Dive data format

The dive log is a sequence of records:

```
u8 type   u8 length (whole record, including these 2 bytes; ≥ 2)   payload…
```

Unknown types are skipped by their length. For known types the length MUST match the table.

| Type | Length | Content (offsets from record start) |
|------|--------|-------------------------------------|
| 0x01 header | 64 | see [Header](#header) |
| 0x02 gas switch | 4 | +2 O2 %, +3 He % |
| 0x03 depth | 4 | +2 u16le depth, cm |
| 0x04 temperature | 4 | +2 u16le temperature, 0.1 °C |
| 0x05 OC/CC switch | 4 | +2 u8 mode (not decoded further) |
| 0x06 gas transmitter | 12 | +2 u8 gas id, +4 u16le transmitter battery, +6 u16le tank pressure 0.1 bar, +8 u16le transmitter id |
| 0x07 compartments | 68 | +4 16 × u16le N2 loadings, +36 16 × u16le He loadings |
| 0x08 GPS | 12 | +4 i32le longitude, +8 i32le latitude, 1e-6 degrees |
| 0x09 ppO2 (board) | 8 | +2..+4 3 × u8 ppO2, 0.01 bar; +6 u16le board serial |
| 0x0A deco | 16 | +2 u8 NDL min, +3 u8 ceiling m, +4 u8 CNS %, +5 u8 safety stop, +6 u16le maximum ceiling, +8 u16le time to surface, +10 u16le OTU |
| 0x0B gradient factors | 4 | +2 GF low %, +3 GF high %. Only the first record counts |
| 0x0C footer | 16 | +2 u8 CNS %, +3 u8 violations, +4 u16le OTU, +6 u16le battery, +8 u32le end time (same epoch as start), +12 u32le desaturation time |
| 0x0D ppO2 (rebreather) | 12 | +2..+4 3 × u8 ppO2 0.01 bar, +8 u16le oxygen tank pressure 0.1 bar, +10 u16le transmitter serial |
| 0x0E compass | 4 | +2 u16le heading, degrees |
| 0x0F log version | 4 | +2 major, +3 minor |
| 0x10 trim | 4 | +2 i8 trim angle |
| 0x11 gas configuration | 8 | +2 gas id, +3 O2 %, +4 He %, +5 enabled flag |
| 0x12 tank transmitter | 8 | +2 u8 transmitter id, +4 u16le battery, +6 u16le pressure 0.01 bar |
| 0x13 GF info (old) | 6 | +2 u16le current GF, +4 u16le surfacing GF |
| 0x14 gas consumption | 4 | +2 u16le SGC |
| 0x15 GF data | 8 | as 0x13, plus +6 u8 leading-tissue current GF, +7 u8 leading-tissue surfacing GF |

Known firmware bug: before log version 1.12, some firmware wrote type 0x15 records with the
old 6-byte length. Treat a type-0x15 record of length 6 as type 0x13.

### Header

Record type 0x01, 64 bytes:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| +2 | u8 | | model code |
| +3, +4 | u8, u8 | | hardware major, minor |
| +5..+7 | 3 × u8 | | firmware major, minor, patch |
| +8, +9 | u8, u8 | | decompression algorithm version |
| +10 | u8 | s | sample interval |
| +11 | u8 | | dive detection setting |
| +12 | u8 | | no-fly time |
| +13 | u8 | | dive mode: 0 OC, 1 CCR, 2 CCR with fixed setpoint, 3 sidemount (OC), 4 gauge |
| +14 | i8 | h | UTC offset |
| +16 | u16le | mbar | surface pressure |
| +18 | u16le | | dive number |
| +20 | u16le | | battery |
| +22 | u16le | min | surface time inside the dive (see [Open questions](#open-questions)) |
| +24 | u32le | s | start time, seconds since 2021-01-01 00:00:00 |
| +28 | u32le | | serial number |

### Samples

Records appear in time order. The header, footer and log-version records are configuration
records and carry no time. Time advances by the sample interval at each depth record, except
the first. The first non-configuration record sets the first sample time (one interval).
Maximum depth is the largest depth record value.

Gas mixes:

- Gas configuration (0x11) defines gases by id. A slot with O2 = He = 0 is empty. Gas
  ids 0-4 are open-circuit gas slots, and ids 5 and up are diluents.
- Gas switch (0x02) gives an active gas by its mixture.
- The gas id in a gas-transmitter record (0x06) also names the active gas.
- Hosts differ on gases that are configured but disabled: one includes every non-empty
  slot, another includes only enabled slots and diluents (see
  [Open questions](#open-questions)).

Tank pressures come from record types 0x06 (0.1 bar), 0x0D (0.1 bar, oxygen bottle) and 0x12
(0.01 bar). Tanks are told apart by transmitter id. Ids from type-0x12 records should be
kept in a separate namespace from type-0x06 ids. Begin pressure is the first reading and end
pressure is the last.

Dive time is end time (footer) − start time (header). One implementation also subtracts
the surface time at header +22. If either time stamp is missing, use the last sample time.

### Date and time

Start time is a `u32le` count of seconds since 2021-01-01 00:00:00 (Unix 1609459200).

- Log version 1.9 or later: the count is UTC. Local time = count + (header +14) × 3600 s.
- Earlier logs: no zone information. Known hosts convert the count as UTC to the **host's**
  current time zone. This is a host convention, not a device fact.

Confidence: **likely** for the record layout. **Uncertain** for the time-to-surface unit, the
deco ceiling unit (assumed metres), and the surface-time subtraction.

## Model-specific variations

- The HUD (model 1) and handset (model 7) share the protocol. Which record types each one
  writes is not documented.
- Log versions: 1.9 introduces the UTC offset and the 0x15 GF data record. 1.12 fixes the
  0x15 length bug.

## Open questions

1. **Surface time.** One implementation subtracts header +22 (minutes) from end − start to
   get the dive time. Another ignores it. Which is right?
2. **Disabled gas slots.** A gas-transmitter record can reference a slot whose
   configuration record had the enabled flag cleared. Implementations differ on whether such
   gases appear in the gas list.
3. The unit of the time to surface (deco record +8): seconds or minutes.
4. The unit of the ceiling (deco record +3): assumed whole metres.
5. Get/set settings payload formats.
6. Whether the ATT MTU is negotiated by the device or must be requested by the host.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, and the Subsurface fork
  of libdivecomputer, <https://github.com/subsurface/libdc>, used as factual references; no
  code copied.
- Subsurface BLE transport (GPL-2.0), UUID table, <https://github.com/subsurface/subsurface>,
  used as a factual reference; no code copied.
- Halcyon Symbios product information, <https://www.halcyon.net/>.
