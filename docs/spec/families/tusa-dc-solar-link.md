# Tusa DC Solar Link and Scubapro Z1

The Tusa DC Solar Link (IQ1204) and the Scubapro Z1 share a Bluetooth Low Energy protocol
that is unrelated to the Uwatec Smart family. The host sends short ASCII commands terminated
by a carriage return: one to fetch a dive index, one per dive to fetch a dive. The device
answers with a line of printable characters that carries binary data in a 6-bit-per-character
encoding. There is no checksum.

**Source status.** This protocol is known from a single implementation observation, and parts
of the dive record were not fully recovered. No public vendor documentation and no
open-source implementation are known. Treat everything here as provisional until confirmed
with a device.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Tusa | DC Solar Link (IQ1204) | none on the wire | BLE | 5-byte index entries |
| Scubapro | Z1 | none on the wire | BLE | 8-byte index entries |

Identification: advertised BLE name starting with `IQ1204`, `SPROXX` or `Z1`. The part of
the name after `-` carries the serial number / firmware string. The two models are told apart
only by the size of their index entries (see [Download procedure](#download-procedure)).

Confidence: **uncertain**.

## Transport parameters

| Role | UUID |
|------|------|
| Service | `c4acde54-23a2-4008-99ae-8c4297647a12` |
| D→H notify characteristic | `c4acde54-23a2-4008-99ae-8c4297647a13` |
| H→D command characteristic | `c4acde54-23a2-4008-99ae-8c4297647a14` |
| Control characteristic | `c4acde54-23a2-4008-99ae-8c4297647a15` |

- Enable notifications on `…7a13`. Data may also arrive as notifications from `…7a15`.
- Commands are written to `…7a14`, in chunks of at most 20 bytes (every command fits in one
  write).
- Control bytes are written to `…7a15` as write-without-response.
- No bonding is required. Scan by name; the service UUID is not advertised in a way a scan
  filter can rely on.
- Read timeout: 600 ms per notification wait; a reply is abandoned after 12 s without data.

See [../transports/](../transports/) for general BLE handling.

Confidence: **uncertain** (UUIDs: **likely**, read directly from one implementation).

## Session

```
H→D  …7a15  01                         (write without response: start session)
            wait 100 ms
H→D  …7a14  69 0D 00 00 00             ("i\r" + 3 × 00: request index)
D→H  …7a13  <reply line ending in 0D>
H→D  …7a14  6C <hh> 0D 00 00 00        ("l" + 2 hex digits + "\r" + 3 × 00: request dive)
D→H  …7a13  <reply line ending in 0D>
            ... one request per dive ...
H→D  …7a15  02                         (write without response: end session)
```

Confidence: **uncertain**.

## Commands

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|
| Start | `01` on `…7a15` | – | none | Sent once, then wait 100 ms |
| Index | `69 0D 00 00 00` | – | Encoded reply: dive index | ASCII `i`, CR, 3 padding zeros |
| Dive | `6C h1 h2 0D 00 00 00` | `h1 h2`: dive number, 2 uppercase ASCII hex digits, 1-based | Encoded reply: one dive | ASCII `l` |
| End | `02` on `…7a15` | – | none | |

Examples: dive 1 = `6C 30 31 0D 00 00 00` (`l01\r`); dive 10 = `6C 30 41 0D 00 00 00`
(`l0A\r`).

Whether dive 1 is the oldest or the newest dive is not known; take the dive dates from the
index.

Confidence: **likely** for the bytes, **uncertain** for the numbering.

## Framing and integrity

A reply is a run of notification payloads concatenated until a `0D` byte is received.

| Offset | Size | Meaning |
|--------|------|---------|
| 0 | 1 | unknown (not interpreted) |
| 1 | 4 | ASCII hex digits: `N`, the number of decoded bytes |
| 5 | … | encoded payload, up to the terminating `0D` |
| end | 1 | `0D` |

**6-bit encoding.** Each payload character `c` carries 6 bits: `v = (c − 0x20) & 0x3F`.
(So both `0x20` and `0x60` decode to 0, as in uuencode.) Every 4 characters give 3 bytes,
most significant bits first:

```
byte0 = v0 << 2 | v1 >> 4
byte1 = (v1 & 0x0F) << 4 | v2 >> 2
byte2 = (v2 & 0x03) << 6 | v3
```

Keep the first `N` decoded bytes. There is no checksum; a host can only check `N` against
the number of characters received.

Worked example. Decoded payload `01 18 0B 18 0A 1E` (6 bytes: one dive, 2024-11-24 10:30)
encodes as:

```
?  30 30 30 36  20 31 40 2B 26 20 48 3E  0D
   "0006"       " 1@+& H>"
```

`?` is the uninterpreted first byte.

Confidence: **likely** for the encoding, **uncertain** for byte 0.

## Memory layout

There is no addressed memory access. The device provides a dive index and individual dives
by number.

## Download procedure

1. Start the session.
2. Request the index and decode it.
   - Decoded byte 0: number of dives `D`.
   - If `N / D ≥ 8`, the device is a Z1 with 8-byte entries; otherwise it is a DC Solar Link
     with 5-byte entries.
   - Each entry begins with `YY MM DD hh mm` (year = 2000 + YY).
   - Where the entries start (byte 1, right after the count, is the natural assumption) is not
     confirmed.
3. For each entry newer than the newest known dive (or all entries for a full download),
   request dive `n` (its 1-based position in the index) and decode the reply.
4. End the session.

The incremental download compares the index date with the stored dive date; there is no
separate fingerprint. A host should use the 5 date bytes of the entry as the fingerprint.

Confidence: **uncertain**.

## Dive data format

### Header

The decoded dive reply. For the Z1, skip 8 leading bytes first; the offsets below then apply
to both models.

| Offset | Type | Raw unit | Conversion | Meaning | Confidence |
|--------|------|----------|------------|---------|------------|
| 0 | u8 | year | 2000 + raw | Year | likely |
| 1 | u8 | – | `raw & 0x0F` | Month | likely |
| 2 | u8 | – | `raw & 0x1F` | Day | likely |
| 3 | u8 | – | `raw & 0x1F` | Hour | likely |
| 4 | u8 | – | `raw & 0x3F` | Minute | likely |
| 5, 6 | u8, u8 | hours, minutes | `b5 × 60 + b6` min | Dive time | uncertain |
| 8 | u16be | 0.1 m | `raw / 10` m | Maximum depth | uncertain |
| 11 bit 1, 12 | 9-bit | 0.1 °C, offset | `(bit × 256 + b12 + 56) / 10` °C | Minimum temperature | uncertain |
| 13 | u16be | 0.1 m | `raw / 10` m | Average depth | uncertain |
| 15 | u8 | s | – | Sample interval | likely |
| 18..21 | 4 × u8 | % | – | O2 fraction of gases 1..4 | likely |
| 24 | – | – | – | First sample | likely |

The temperature formula's offset of 56 means a raw 9-bit value of 0 equals 5.6 °C. Whether
"bit 1 of byte 11" is really the ninth bit is not confirmed.

### Samples

Samples are 4 bytes each, one per sample interval, starting at offset 24. Let `W` be the
u16be at sample offset 0.

| Field | Bits | Conversion |
|-------|------|------------|
| Depth | `W` bits 9..0 | probably 0.1 m (`raw / 10` m). For the first sample only bits 7..0 are used. |
| Gas | `W` bits 11..10 | gas number = value − 1 (values 0 and 1 both mean gas 0), indexing the O2 list |
| Temperature | `W` bit 13 as bit 8, plus sample byte 2 as bits 7..0 | `(raw + 56) / 10` °C; raw values 0 and 1 mean "no temperature" |
| – | sample byte 3 | unused |

Samples whose `W & 0xFF00` equals `0x5000` are markers of unknown meaning. One implementation
still treats them as ordinary samples; a host should probably skip or flag them.

Confidence: **uncertain**.

### Date and time

Dive start is given as year/month/day/hour/minute in local device time, without a time zone
and without seconds.

## Model-specific variations

- Z1: 8-byte index entries and 8 extra leading bytes in each dive reply.
- DC Solar Link: 5-byte index entries.

## Open questions

1. Meaning of byte 0 of each reply.
2. Content of the extra 3 bytes in Z1 index entries and of the 8 extra bytes before the Z1
   dive record.
3. Dive numbering: does 1 mean oldest or newest?
4. Exact header offsets 5..17, in particular dive time, depth, temperature and any tank
   pressure or deco data.
5. Meaning of the `0x50xx` marker samples and of sample byte 3.
6. Are notifications from `…7a15` ever used for data?
7. What do the `01` and `02` control writes do (wake/stop, or link-layer flow control)?

## References

- Bluetooth Core Specification (GATT notifications, write without response),
  <https://www.bluetooth.com/specifications/specs/>.
- No public protocol documentation or open-source implementation is known.
