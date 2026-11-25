# Shearwater Predator and Petrel family

Shearwater dive computers share one request/response protocol. The host sends small
command packets and the device answers each with one response packet. Packets carry a
4-byte (or, on the newest models, 5-byte) header, are SLIP-framed, and on most Bluetooth
Low Energy links get a 2-byte segmentation prefix. There is no checksum.
The command set has five commands: read a data item by 16-bit identifier, write a data
item, and a three-step bulk upload (start, fetch block, finish). The upload can return
data compressed with a 9-bit zero-run coding followed by a 32-byte XOR delta.

There are two download strategies:

- **Predator**: one uncompressed upload of the whole 0x20080-byte log memory. The host
  then finds dives in a ring buffer of 128-byte blocks.
- **Petrel family** (everything newer): the host reads a *manifest* (a directory of
  32-byte dive entries), then uploads each new dive individually, compressed.

The dive data format is described separately in
[shearwater-log-format.md](shearwater-log-format.md).

## Devices

Model codes are the value of data item `0x8060` (one byte) and of byte 13 of the log's
final record. See [Identification](#identification).

| Vendor | Model | Model code | Transports | Download strategy | Notes |
|--------|-------|-----------:|------------|-------------------|-------|
| Shearwater | Predator | 2 | BT classic | full dump | Predator log format |
| Shearwater | Petrel | 3 | BT classic | manifest | |
| Shearwater | Petrel 2 | 3 | BT classic, BLE | manifest | same code as Petrel |
| Shearwater | NERD | 4 | BT classic | manifest | |
| Shearwater | Perdix | 5 | BT classic, BLE | manifest | |
| Shearwater | Perdix AI | 6 | BLE | manifest | |
| Shearwater | NERD 2 | 7 | BLE | manifest | |
| Shearwater | Teric | 8 | BLE | manifest | clock set in UTC + offset |
| Shearwater | Peregrine | 9 | BLE | manifest | |
| Shearwater | Petrel 3 | 10 | BLE | manifest | |
| Shearwater | Perdix 2 | 11 | BLE | manifest | |
| Shearwater | Tern, Tern TX | 12 | BLE | manifest | |
| Shearwater | Peregrine TX | 13 | BLE | manifest | |
| Shearwater | Perdix 3 | 14 | BLE | manifest | protocol variant 2 (see [Framing](#framing-and-integrity)) |

Libdivecomputer also lists the oldest models (Predator, Petrel, Petrel 2, NERD,
Perdix) on a "serial" transport. In practice that is the Bluetooth classic RFCOMM
link opened as a serial port. No Shearwater USB cable protocol is known.

**Identification by name.** The Bluetooth classic device name and the BLE advertised
name start with one of: `Predator`, `Petrel`, `Petrel 3`, `NERD`, `NERD 2`, `Perdix`,
`Perdix 2`, `Perdix 3`, `Teric`, `Peregrine`, `Peregrine TX`, `Tern`. The name
usually carries a serial-number suffix, so hosts should match on a prefix. The name
alone cannot tell apart models that share a prefix (e.g. `Petrel` vs `Petrel 3`).
Read data item `0x8060` after connecting.

Confidence: **likely** (one source for the model table; name prefixes from two
sources).

## Transport parameters

### Serial / Bluetooth classic (SPP)

- RFCOMM via the Serial Port Profile, UUID `00001101-0000-1000-8000-00805F9B34FB`.
  Pairing without MITM protection ("insecure" RFCOMM) has been used successfully for
  every model except possibly the original Petrel, where a secure socket was used.
- When opened as a serial port: 115200 baud, 8 data bits, no parity, 1 stop bit, no
  flow control. The baud rate has no effect on an RFCOMM link.
- No segmentation prefix. The SLIP stream is sent and received as-is.
- Before the first command, wait 300–800 ms and discard any pending input.
- Response timeout: implementations use between 1 s and 12 s. A timeout of at least
  3 s is recommended, because the device can take a while to answer an upload start.

Confidence: **likely**.

### Bluetooth Low Energy

| Item | Value |
|------|-------|
| Service | `FE25C237-0ECE-443C-B0AA-E02033E7029D` |
| Characteristic (write and notify) | `27B7570B-359E-45A3-91BB-CF7E70049BD2` |
| Characteristic properties | write, write-without-response, notify |
| CCCD | `0x2902`; enable notifications (or indications if offered) |

The same characteristic carries both directions. The host writes first. Until the
first command arrives the device displays "Wait Cmnd" (or similar), and it shows
nothing on its own.

**Write size.** Hosts write in chunks of at most 20 bytes (the default ATT MTU minus 3)
and wait for each write to complete before sending the next. Write-with-response and
write-without-response have both been used. Hosts commonly request a larger ATT MTU
(63 and 80 bytes have been used), but the protocol does not depend on it.

**Segmentation prefix (protocol variant 1 only).** On BLE, every write and every
notification in variant 1 begins with two bytes:

| Byte | Meaning |
|------|---------|
| 0 | total number of BLE packets that make up this SLIP frame |
| 1 | index of this packet, starting at 0 |

The rest of the packet (at most 18 bytes in a 20-byte write) is the next slice of the
SLIP-encoded frame. The receiver strips the two prefix bytes from every notification
and feeds the rest to the SLIP decoder. The SLIP `C0` terminator, not the packet
count, marks the end of a frame. Notifications from the device can be longer than 20
bytes if the MTU allows.

How the packet count is computed differs between implementations. One computes
`ceil(len / 20)` over the SLIP-encoded length including the terminator. Another
computes `1` if the slice is 18 bytes or fewer, else `floor(len / 18) + 1`. Both give
`1` for every short command frame, which covers all commands a downloader sends.
See [Open questions](#open-questions).

Protocol variant 2 (Perdix 3) sends **no** segmentation prefix on BLE.

Confidence: UUIDs and the prefix for single-packet frames **verified** (observed in a
published capture, see the [worked example](#worked-example-frames)). Multi-packet
prefix counting **uncertain**.

## Session

There is no wake-up or authentication sequence. A session is:

1. Connect, wait 300–800 ms, discard pending input.
2. (Petrel family) Read identity items with RDBI: serial `0x8010`, firmware `0x8011`,
   model `0x8060`, optionally hardware `0x8050`, then log-upload descriptor `0x8021`.
   Any command takes the device out of its "waiting for command" screen.
3. Download (see [Download procedure](#download-procedure)).
4. Optionally set the clock (see [Commands](#commands)).
5. End the session by writing data item `0x9020` with the single byte `00`:
   payload `2E 90 20 00`. The host does not wait for a response. Pausing about
   500 ms before and 100 ms after has been used. The device then leaves its
   communication screen.

Confidence: **likely**.

### Data items

| Identifier | Direction | Size | Content |
|-----------:|-----------|------|---------|
| `0x8010` | read | 8 | serial number as 8 ASCII hex digits. Decoded as hex it gives a 4-byte big-endian number |
| `0x8011` | read | ≤ 12 | firmware version as ASCII text. The decimal number starting at the second character is the firmware version (e.g. a leading letter followed by digits) |
| `0x8021` | read | 9 | log-upload descriptor. Bytes 1–4 are a `u32be` base address for dive data (see below) |
| `0x8050` | read | 2 | hardware/firmware identifier, `u16be` (see [Identification](#identification)) |
| `0x8060` | read | 1 | model code (table in [Devices](#devices)) |
| `0x9020` | write | 1 | write `00` to end the communication session |
| `0x9030` | write | 4 | local time, `u32be`, seconds since 1970-01-01 counted on the local wall clock |
| `0x9031` | write | 4 | UTC time, `u32be`, Unix seconds |
| `0x9032` | write | 4 | UTC offset, `i32be`, minutes |
| `0x9033` | write | 4 | daylight-saving flag/offset, `u32be` (writing `0` with the full offset in `0x9032` works) |

**Log-upload base (`0x8021`, bytes 1–4):**

| Value | Meaning | Use as dive base |
|-------|---------|------------------|
| `0x80000000` | native log format with final record | `0x80000000` |
| `0x90000000` | early native format without final record (some firmware) | use `0xC0000000` |
| `0xC0000000` | Predator-like log format | `0xC0000000` |
| `0xDD000000` | Predator memory | use `0xC0000000` |
| other | unknown format | abort |

The mapping in the third column is what libdivecomputer does: reading through
`0xC0000000` returns the Predator-like format, which every Petrel-family firmware
still supports. Another implementation only checks whether byte 1 equals `0x80` and
then switches between `0x80000000` and `0xC0000000`. Both agree for the values seen
in practice.

Confidence: **likely**.

## Commands

All commands go inside the packet header (next section). The first payload byte is the
command code. The response code is the command code plus `0x40`.

| Command | H→D payload | D→H payload | Notes |
|---------|-------------|-------------|-------|
| Read data item (RDBI) | `22 <id:2 be>` | `62 <id:2 be> <data>` | response echoes the identifier |
| Write data item (WDBI) | `2E <id:2 be> <data>` | `6E <id:2 be>` | used for clock and session end |
| Start upload | `35 <flags> 34 <addr:4 be> <size:3 be>` | `75 <n<<4> <maxblock:n be>` | see below |
| Fetch block | variant 1: `36 <seq>`. Variant 2: `36 <seq> 00` | `76 <seq> <data>` | `seq` starts at 1, wraps after `FF` |
| Finish upload | `37` | `77 00` | |
| Negative response | — | `7F <cmd> <error>` | e.g. `7F 22 xx` for an unknown data item |

The command codes, the `+0x40` positive-response rule, the `7F` negative response and
the upload parameter layout match the Unified Diagnostic Services protocol
(ISO 14229-1): ReadDataByIdentifier, WriteDataByIdentifier, RequestUpload,
TransferData and RequestTransferExit. That standard is a useful guide to the field
meanings. For example, error `0x31` is "request out of range" there. The device
implements only the subset listed here.

**Start upload.** `flags` is `10` for a compressed transfer and `00` for raw. In UDS
terms this is the data-format identifier: compression method 1, no encryption. The
byte `34` is the address-and-length format identifier: a 4-byte address and a 3-byte
size. `addr` is a 32-bit big-endian address in the device's upload
address space. `size` is a 24-bit big-endian byte count. `FF FF FF` means "until the
end of the object" and is used for dives.

The response's second byte has a field length `n` in its high nibble (`n` ≤ 4), as in
the UDS length-format identifier. It is
followed by `n` bytes, big-endian: the largest block payload the device will send. A
Perdix 2 answers a compressed dive request with `75 10 92`, so `n` = 1 and the maximum
block is 146 bytes. The host should check that the maximum block fits its buffer
(254 bytes of payload in variant 1, 4096 in variant 2). The low nibble of byte 1 has no
known meaning.

On a Perdix 2, an uncompressed (`flags = 00`) request for a dive address is refused
with `7F 35 31`. Dives MUST be requested compressed. The manifest MUST be requested
uncompressed.

**Fetch block.** The host sends block numbers 1, 2, 3, … The device echoes the
number. Each response carries up to `maxblock` bytes of (possibly compressed) data.
Stop requesting when the decompressor reports end of stream (compressed transfers),
or when the requested `size` has been received (raw transfers). The block number is a
single byte and wraps from `FF` to `00`.

**Upload address space:**

| Address | Size | Compression | Content |
|---------|------|-------------|---------|
| `0xDD000000` | `0x20080` | raw | Predator: complete log memory |
| `0xE0000000` | `0x600` | raw | Petrel family: one manifest page (48 × 32 bytes) |
| base + dive offset | `0xFFFFFF` | compressed | Petrel family: one dive |

Confidence: **likely**. The `75 10 92` response and the NAK for raw dive reads were
**verified** on a Perdix 2 in a published capture.

## Framing and integrity

### Packet header

Two header variants exist. Variant 2 is used only by the Perdix 3 (model code 14).
Every other model uses variant 1.

| Variant | H→D header | D→H header | Length field |
|---------|------------|------------|--------------|
| 1 | `FF 01 <L> 00` | `01 FF <L> 00` | `L = payload length + 1`, one byte. Max payload 254 |
| 2 | `FF 01 00 <len:2 be>` | `01 FF 00 <len:2 be>` | `len = payload length`. Max payload seen 0x0902 (dive) and 0x0202 (manifest) |

The receiver checks the first two bytes (`01 FF`), the zero byte (offset 3 in variant
1, offset 2 in variant 2), and that the header length plus payload length equals the
de-SLIPped frame length.

One implementation reads variant 1's bytes 2–3 as a **little-endian** 16-bit `L`
instead (`L` low byte, then high byte). For payloads under 255 bytes the high byte is
always zero, so the two readings produce identical frames.

### SLIP

After the header is added, the packet is SLIP-encoded (RFC 1055):

| Raw byte | Sent as |
|----------|---------|
| `C0` (END) | `DB DC` |
| `DB` (ESC) | `DB DD` |

One `C0` ends each frame. Senders send **no** leading `C0`. Receivers SHOULD ignore
empty frames (consecutive `C0`s). An `ESC` followed by `C0` or `DB` is a protocol
error. An `ESC` followed by any other byte is either an error or passed through
unchanged, depending on the implementation.

### Integrity

There is no checksum, CRC or sequence number in either direction. Integrity relies on
the link layer (RFCOMM or BLE) and on the header length check.

### Compression

Compressed uploads (`flags = 10`) are encoded in two stages. Decode them in reverse
order:

1. **Zero-run decoding, per block.** The payload of each `76` block (after `76 <seq>`)
   is a string of 9-bit codes, most significant bit first, packed with no gaps. The
   payload length in bits MUST be a multiple of 9. Codes never span two blocks. For
   each code `c`:
   - bit 8 set (`c ≥ 0x100`): output the literal byte `c & 0xFF`;
   - `c == 0`: end of stream. Ignore the rest of this block and stop the transfer;
   - otherwise (`1 ≤ c ≤ 0xFF`): output `c` zero bytes.
2. **XOR delta, once over the whole output.** For every index `i ≥ 32`, in increasing
   order: `out[i] = out[i] XOR out[i − 32]`. The first 32 bytes are unchanged. Each
   32-byte record is therefore stored as its difference from the previous record.

```
decode(blocks):
    out = []
    for each block payload p:
        require len(p) * 8 mod 9 == 0
        for each 9-bit code c in p (MSB first):
            if c & 0x100: append (c & 0xFF)
            elif c == 0: goto done
            else: append c zero bytes
    done:
    for i from 32 to len(out) - 1: out[i] ^= out[i - 32]
    return out
```

Confidence: **likely**. The algorithm comes from one reference implementation and
matches the stream structure seen on a real Perdix 2.

### Worked example frames

RDBI of the serial number (`0x8010`), variant 1, serial/RFCOMM:

```
payload   22 80 10
header    FF 01 04 00                (L = 3 + 1)
SLIP      FF 01 04 00 22 80 10 C0
```

The same frame on BLE, with the segmentation prefix `01 00` (1 packet, index 0):

```
H→D  01 00 FF 01 04 00 22 80 10 C0
```

This exact byte string, and the session-end frame `01 00 FF 01 05 00 2E 90 20 00 C0`,
appear in published BLE logs from real devices.

Variant 2 (Perdix 3) RDBI and first block fetch:

```
H→D  FF 01 00 00 03 22 80 10 C0
H→D  FF 01 00 00 03 36 01 00 C0
```

An upload start that needs escaping: compressed, address `0x8000C0DB`, size `FFFFFF`:

```
payload   35 10 34 80 00 C0 DB FF FF FF
header    FF 01 0B 00
SLIP      FF 01 0B 00 35 10 34 80 00 DB DC DB DD FF FF FF C0
BLE       01 00 FF 01 0B 00 35 10 34 80 00 DB DC DB DD FF FF FF C0   (17 + 2 bytes, one write)
```

### Worked decompression example

A block payload of 9 bytes holds exactly eight 9-bit codes:

```
block payload   D2 87 E0 20 30 00 00 00 00
9-bit codes     1A5  01F  101  003  000  000  000  000
meaning         lit A5, 31 zeros, lit 01, 3 zeros, END (rest ignored)
after stage 1   A5 00×31 01 00 00 00                        (36 bytes)
after stage 2   A5 00×31 A4 00 00 00                        (byte 32 = 01 XOR A5)
```

## Memory layout

### Predator (full dump, 0x20080 bytes)

| Range | Content |
|-------|---------|
| `0x00000`–`0x1F5FF` | dive log ring buffer, 1004 blocks of 128 bytes |
| `0x1F600`–`0x1FFFF` | unknown (not used for dive extraction) |
| `0x20000`–`0x2007F` | identity block, same layout as a log final block |

Identity block: `0x20002` serial (4 bytes, see [Open questions](#open-questions) on
byte order), `0x2000A` firmware version (BCD), `0x2000D` model code.

Ring-buffer blocks are classified by their first two bytes:

| First bytes | Meaning |
|-------------|---------|
| `FF FF` | dive opening block. Bytes 2–3 are the dive number, `u16be` |
| `FF FE` | dive closing block. Bytes 2–3 repeat the dive number |
| 128 × `FF` | empty (erased) block. Must be checked before the `FF FF` test |
| anything else | sample block |

The dive counter increases with every dive, so the opening block with the highest
dive number belongs to the newest dive.

A Petrel (model code 3) read through the same full-dump path returns the ring already
rotated so that the newest dive comes first. No search for the ring end is needed.

### Petrel family manifest

Each manifest page is 0x600 bytes: 48 entries of 32 bytes, newest dive first.

| Offset | Type | Meaning |
|-------:|------|---------|
| 0x00 | `u16be` | entry marker: `A5C4` = valid dive, `5A23` = deleted dive, anything else = end of list |
| 0x04 | `u32be` | dive start time (same value as the dive's fingerprint, see below) |
| 0x14 | `u32be` | dive offset. Add the log-upload base from `0x8021` |
| others | — | unknown |

If a page holds 48 entries (valid plus deleted), the host uploads `0xE0000000` again
and the device returns the next page. A page with fewer entries ends the manifest.

Confidence: **likely**. The repeated-read paging is inferred from host behaviour and
is not confirmed with a device holding more than 48 dives.

## Download procedure

### Fingerprint

The fingerprint is 4 bytes: the dive start time, `u32be`. It sits at offset 12 of the
dive data (opening block or opening record 0) and at offset 4 of the manifest entry.

### Petrel family

```
identify: rdbi 0x8010, 0x8011, 0x8060, (0x8050), 0x8021
base = map(0x8021 bytes 1..4)              (see table above)
entries = []
loop:
    page = upload(0xE0000000, 0x600, raw)
    n = 0
    for each 32-byte entry e in page:
        if e.marker == 5A23: n += 1; continue      (deleted)
        if e.marker != A5C4: break
        if e[4..8] == known fingerprint: stop_all = true; break
        entries.append(e); n += 1
    if stop_all or n < 48: break
for e in entries:                           (newest first)
    dive = upload(base + e.offset, 0xFFFFFF, compressed)
    deliver dive; stop if the consumer says so
end session: send 2E 90 20 00, no reply
```

Each `upload` is: start (`35`), fetch blocks (`36`, numbered 1, 2, …) until the end
condition, finish (`37`, expect `77 00`).

### Predator

1. Upload `0xDD000000`, size `0x20080`, raw.
2. Scan the 128-byte blocks of the ring backwards from offset `0x1F600`, wrapping at 0.
   Pair each closing block with the opening block before it. The dive numbers MUST
   match. The newest dive is the pair with the highest dive number. The ring's logical
   end is just after that dive's closing block.
3. Rotate the ring so it ends there. Walk backwards again and cut out each dive from
   its opening block to its closing block (inclusive). Stop at an empty block.
4. Append the 128-byte identity block (`0x20000`) to each dive as its final block. The
   log parser expects this.
5. Deliver newest first. Stop when the dive's fingerprint (offset 12) matches the
   stored one.

Dives on the Predator are not compressed.

### Clock setting

- Teric (model 8): write `0x9031` (UTC seconds), `0x9032` (offset in minutes), and
  `0x9033` = 0.
- All other Petrel-family models and the Predator: write `0x9030` with the local wall
  clock time, encoded as if it were UTC.

Confidence: **likely**.

## Identification

The primary model identifier is the model code from data item `0x8060`, or byte 13 of
the log's final record. Petrel and Petrel 2 share code 3.

Data item `0x8050` returns a 16-bit hardware/firmware identifier. It can tell apart
models that share a model code, but it can change with firmware updates and is not
stored in logs. One host matches it exactly first, and then after masking with
`0x0F0F`:

| `0x8050` value | Match | Model |
|----------------|-------|-------|
| `0x0009` | masked | Peregrine |
| `0x0505`, `0x0805`, `0x0808`, `0x0B0C` | masked | Petrel 2 |
| `0x0707` | masked | Perdix |
| `0x0909`, `0x0B0B` | masked | Petrel |
| `0x0A0A` | masked | NERD |
| `0x0C0D` | masked | Perdix AI |
| `0x0E0D` | masked | NERD 2 |
| `0x0F0A`, `0x0F0F` | masked | Teric |
| `0xB407` | exact | Petrel 3 |
| `0xC407` | exact | Perdix 2 |

Confidence: **uncertain** (one source; the mapping is incomplete for newer models).

## Model-specific variations

- **Perdix 3 (14):** packet header variant 2, 3-byte block fetch `36 <seq> 00`, no BLE
  segmentation prefix, larger blocks.
- **Teric (8):** clock is set as UTC plus offset. Its logs from version 9 on carry the
  UTC offset.
- **Predator (2):** full dump only, no manifest, no compression.
- **Petrel (3)** via the full-dump path: ring pre-rotated, newest dive first.
- Some firmware reports log base `0x90000000` (an early native format without the
  final record). Download those dives through `0xC0000000` instead.

## Open questions

1. **BLE packet count for multi-packet frames.** Implementations compute the count
   byte differently: over the whole SLIP frame divided by 20, or per slice of at most
   32 bytes divided by 18. Whether the device checks this byte is unknown. Every
   command a downloader needs fits in one 20-byte write, so the question only matters
   for long WDBI writes.
2. **Predator serial byte order.** The serial at dump offset `0x20002` is read as
   big-endian by one implementation and as little-endian by another.
3. **Perdix 2 model code.** A third-party probe reports model `0x11` for a Perdix 2.
   The reference table gives 11 (`0x0B`). This is probably decimal 11 written with a
   hex prefix, but it is unconfirmed.
4. **Start-upload response.** Byte 1 is read here as a UDS-style length nibble. One
   reading treats `0x10` as an echo of the compression flag instead. Both readings
   agree for every observed response (`75 10 xx`).
5. **Manifest paging.** Is the next page really returned by re-reading `0xE0000000`?
   How do deleted entries count toward the 48-entry "page full" test? Two
   implementations count them differently.
6. Meaning of manifest bytes 2–3 and 8–19, and of Predator memory `0x1F600`–`0x1FFFF`.
7. Response timeouts: values from 1 s to 12 s are in use. The worst-case delay of the
   device on a `35` start is unknown.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>,
  used as a factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1),
  <https://github.com/subsurface/libdc>, used as a factual reference; no code copied.
- shearwater-rs (MIT/Apache-2.0), protocol notes and Perdix 2 observations,
  <https://github.com/kenthejr/shearwater-rs/blob/main/docs/protocol.md>.
- ISO 14229-1, *Road vehicles — Unified diagnostic services (UDS)*, for the general
  shape of the command set (no Shearwater-specific content).
- RFC 1055, *A Nonstandard for Transmission of IP Datagrams over Serial Lines (SLIP)*.
- Bluetooth SIG Assigned Numbers (SPP UUID, CCCD).
