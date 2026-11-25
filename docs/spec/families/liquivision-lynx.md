# Liquivision family (Xen, Xeo, Lynx, Kaon)

Liquivision computers with a USB-serial interface. The host wakes the device with a long
run of `AA` bytes. Commands are 12 ASCII characters inside a fixed 2-byte prefix and 2-byte
suffix. Replies start with `C5` and end with a CRC-16. Memory is 2 MiB and is read in
1 KiB segments. Dive headers live in a logbook area of 4 KiB pages. Profiles live in a
large ring buffer. Every dive header carries a CRC-32 that is seeded with a per-firmware
key word.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Liquivision | Xen | 0 | serial | 80-byte dive header. Gauge only |
| Liquivision | Xeo | 1 | serial | 96-byte dive header |
| Liquivision | Lynx | 2 | serial | 96-byte dive header. Gas list command |
| Liquivision | Kaon | 3 | serial | 96-byte dive header |

The model code is the first u16 of the INFO reply. The serial number is the first u32 of
the MOREINFO reply. One reference prints it as four decimal groups, most significant byte
first: `%d%02d%02d%02d`.

Identification: USB-serial cable. No VID:PID is established by public sources.

### Firmware key words

The INFO reply also holds a 32-bit word that identifies the firmware/data version. It seeds
the dive header CRC and selects some format details.

| Key | Model | Remarks |
|-----|-------|---------|
| 0x83321485 | Xen | Earliest version. Reported as unsupported |
| 0x83321502 | Xen | |
| 0x83328401 | Xen | |
| 0x17485623 | Xeo | "A" variant. Header CRC covers 6 fewer bytes |
| 0x27485623 | Xeo | "B" variant. Header CRC covers 6 fewer bytes |
| 0x17488401 | Xeo | "A" variant |
| 0x27488401 | Xeo | "B" variant |
| 0x17488402 | Xeo | "A" variant |
| 0x27488402 | Xeo | "B" variant |
| 0x67488403 | Lynx | Short tank-pressure event |
| 0x67488404 | Lynx | |
| 0x67488405 | Lynx | Reportedly the SNUBA variant |
| 0x37488402 | Kaon | "A"-type sample size |
| 0x47488402 | Kaon | "A"-type sample size |

"A" variants (0x17485623, 0x17488401, 0x17488402, 0x37488402, 0x47488402) store 14 extra
bytes per sample.

Another reference uses fixed keys for some models instead of the INFO word: Kaon
0x47488402, Xeo 0x27488402. It uses fallback keys when INFO returns 0: Xen 0x83328402,
Lynx 0x67488404.

Confidence: **likely**.

## Transport parameters

- Serial, 9600 baud, 8 data bits, no parity, 1 stop bit, no flow control.
- DTR and RTS de-asserted (one reference). Another does not touch them.
- Read timeout: 3000 ms in one reference, 600 ms in another.
- One reference writes command bytes one at a time with 10 ms between bytes. Another writes
  the 16-byte command in one go.

Confidence: **likely** for the line settings. **uncertain** for the pacing.

## Session

1. Wait 100 to 1000 ms, then discard pending input and output.
2. **Wake-up:** send the byte `AA` repeatedly. One reference sends 6000 bytes back to back.
   Another sends 5000 bytes with about 1 ms between bytes.
3. **Identify.** The references differ in order:
   - A: INFO, then MOREINFO.
   - B: MOREINFO twice (the first reply is discarded, up to 9 attempts with 200 ms
     between), then INFO.
4. Download (see below).
5. **Shutdown:** send FINISH. There is no reply.

INFO reply payload (6 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u16le | Model code |
| 2 | u32le | Firmware key word |

MOREINFO reply payload (12 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u32le | Serial number |
| 4 | u32le | Highest dive ID + 1 (one reference). Headers with a dive ID at or above this value are ignored |
| 8 | 4 bytes | Unknown |

Confidence: **likely**.

## Commands

Every command is 16 bytes:

```
H→D  00 B1 <12 ASCII characters> 0B 0E
```

| Command | ASCII text | Reply payload | Notes |
|---------|------------|---------------|-------|
| Info | `INFOINFOINFO` | 6 bytes | |
| More info | `MOREINFOMORE` | 12 bytes | |
| Read segment | `PAGE` + `ppp` + `ppp` + `s` + `s` | 1024 bytes | `ppp` = 3-digit page number (repeated), `s` = 1-digit segment 0..3 (repeated) |
| Gas list (Lynx) | `DOWNLOADGASX` | up to 1024 bytes | One reference only |
| Finish | `FINISHFINISH` | none | |

Page and segment: `address = page × 4096 + segment × 1024`. Addresses must be multiples
of 1024.

Byte forms:

```
INFO          00 B1 49 4E 46 4F 49 4E 46 4F 49 4E 46 4F 0B 0E
MOREINFO      00 B1 4D 4F 52 45 49 4E 46 4F 4D 4F 52 45 0B 0E
FINISH        00 B1 46 49 4E 49 53 48 46 49 4E 49 53 48 0B 0E
DOWNLOADGASX  00 B1 44 4F 57 4E 4C 4F 41 44 47 41 53 58 0B 0E
PAGE 0, seg 0 00 B1 50 41 47 45 30 30 30 30 30 30 30 30 0B 0E    "PAGE00000000"
PAGE 25,seg 2 00 B1 50 41 47 45 30 32 35 30 32 35 32 32 0B 0E    "PAGE02502522"
```

On a timeout or bad reply, wait about 100 ms, discard input and resend. The references allow
2 retries for every command, and 3 attempts in total for segment reads.

Confidence: **likely**.

## Framing and integrity

### Reply frame

```
D→H  C5 <payload:n> <crc:u16be>
```

`n` is fixed by the command (6, 12 or 1024). The first byte must be `C5`.

CRC-16/CCITT-FALSE: width 16, poly 0x1021, init 0xFFFF, refin false, refout false,
xorout 0x0000. It covers the payload only (not `C5`) and is transmitted **big-endian**.
See [../common/checksums.md#crc-16-ibm-3740](../common/checksums.md#crc-16-ibm-3740).

Worked example: INFO reply for a Lynx with key 0x67488404:

```
D→H  C5 02 00 04 84 48 67 30 1F
         ^^^^^ model 2
               ^^^^^^^^^^^ key 0x67488404 (LE)
                           ^^^^^ CRC 0x301F
```

### Dive header CRC-32

CRC-32/BZIP2: width 32, poly 0x04C11DB7, init 0xFFFFFFFF, refin false, refout false,
xorout 0xFFFFFFFF. See
[../common/checksums.md#crc-32-bzip2](../common/checksums.md#crc-32-bzip2) (check value
0xFC891918).

The CRC is stored as u32le at header offset 0. To verify it, compute the CRC over:

1. the 4 bytes of the firmware key word, little-endian (as received in INFO), then
2. header bytes `4 .. H − u`, where H is the header size (80 or 96).

| Variant | u (excluded tail bytes) | Bytes covered after the key |
|---------|-------------------------|-----------------------------|
| Xeo keys 0x17485623, 0x27485623 | 6 | H − 10 |
| All others | 2 | H − 6 (74 for Xen, 90 for 96-byte headers) |

Another reference covers 76 bytes after the key for the Xen (that is, `u = 0`). See open
questions.

Confidence: **likely** for the reply CRC. **uncertain** for the Xen header CRC range.

## Memory layout

Memory is 2 MiB (0x000000..0x1FFFFF), read in 1024-byte segments. A *page* is 4096 bytes.

| Address range | Content |
|---------------|---------|
| 0x000000..0x0003FF | Configuration segment |
| 0x001000..0x018FFF | Logbook area: pages 1..24 |
| 0x019000..0x1F3FFF | Profile ring buffer: pages 25..499 |

Configuration segment:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x46 | u16le | Logbook begin, as an entry index |
| 0x48 | u16le | Logbook end, as an entry index (one past the newest) |
| 0x4A | u32le | Profile ring begin address |
| 0x4E | u32le | Profile ring end address (one past the newest data) |

Logbook packing: dive headers are packed from the start of each logbook page. Only whole
headers fit, and the remainder of the page is padding.

| Header size | Headers per page | Padding at end of page |
|-------------|------------------|------------------------|
| 80 (Xen) | 51 | 16 bytes |
| 96 (others) | 42 | 64 bytes |

Index *i* maps to address `0x1000 + (i div per_page) × 4096 + (i mod per_page) × H`.

The logbook is itself a ring over pages 1..24. The begin index only covers dives whose
profile still exists, so a host may ignore it and scan the whole logbook.

Confidence: **likely**.

## Download procedure

Two equivalent strategies are documented.

**Strategy A (pointer-driven, newest first):**

1. Read the configuration segment. Compute the logbook end address from the end index.
   Validate both profile pointers (inside 0x19000..0x1F4000).
2. Walk the logbook backwards from the end address, wrapping from 0x1000 to 0x19000. At
   each page boundary, skip that page's padding. Read headers one by one:
   - stop at an all-`0xFF` header;
   - verify the CRC-32;
   - incremental download: the fingerprint is the 4-byte CRC at offset 0. Stop when it
     matches the stored fingerprint.
3. For each accepted header, newest first, the profile runs from its pointer (offset 16)
   up to the next newer dive's pointer. For the newest dive it runs to the profile ring end
   pointer. Length = `(next − cur) mod ring_size` (0 means the full ring). Stop allocating
   once the cumulative length exceeds the ring size.
4. Read the profile bytes backwards through the ring using segment reads.
5. Output the dive as the header followed by the profile bytes. One reference replaces
   header bytes 0..3 (the CRC) with the firmware key word, so that the parser knows the
   format variant.

**Strategy B (scan):** read logbook segments from page 1 upwards until a segment is all
`0xFF`. Accept a header when its CRC-32 matches and its dive ID (offset 24) is below the
MOREINFO limit. Filter by date against the newest stored dive. For each dive, read only the
segments its profile occupies:
`segment = 25 × 4 + (address − 0x19000) / 1024`. Bytes needed are approximately
`samples × sample_size + events × event_size`.

On the Lynx, strategy B also sends DOWNLOADGASX first. The reply is a count byte, an unknown
byte, then `count` × 4-byte entries (O2 %, He %, then two bytes of unknown meaning).

Finish with FINISH.

Confidence: **likely**.

## Dive data format

### Header

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0 | u32le | – | Header CRC-32 (fingerprint) |
| 4 | u32le | s | Dive time |
| 8 | u32le | count | Number of sample records |
| 12 | u32le | count | Number of event records |
| 16 | u32le | address | Profile start in the ring |
| 20 | u32le | s | Surface interval before the dive (one reference) |
| 24 | u16le | – | Dive ID |
| 26 | u16le | mbar | Atmospheric pressure (÷1000 → bar) |
| 28 | u16le | cm | Maximum depth |
| 30 | u16le | cm | Average depth |
| 32 | u16le | – | Dive number within the session (Kaon and Xeo "B" keys, one reference) |
| 34 | i16le | 1/10 °C | Minimum temperature |
| 36 | i16le | 1/10 °C | Maximum temperature (one reference calls it surface temperature) |
| 38 | u8 | – | Water type: 0 fresh (1000 kg/m³), 1 brackish (1015), 2 salt (1025) |
| 39 | u8 | – | Sample interval index: 0→1 s, 1→2 s, 2→5 s, 3→10 s, 4→30 s, 5→60 s |
| 44 | u16le | s | Second |
| 46 | u16le | min | Minute |
| 48 | u16le | h | Hour |
| 52 | u16le | – | Day − 1 (zero-based) |
| 56 | u16le | – | Month − 1 (zero-based) |
| 58 | u16le | – | Year |
| 92 | u8 | – | 96-byte headers only. Low nibble = dive mode: 0 deco, 1 gauge, 2 technical, 3 recreational (0, 2 and 3 are open circuit). High nibble = air-integration mode. Values ≥ 5 indicate sidemount (one reference) |
| 93 | u8 | – | 96-byte headers only. Decompression model: 0 Bühlmann ZHL-16 with gradient factors, 1 RGBM |

The Xen has no mode byte and is always a gauge.

### Samples and events

The profile is a stream of u16le-headed records. Parsing stops when the declared numbers of
samples and events have been consumed, when the data ends, or (one reference) at a word
equal to `0xFFFF`.

**Sample record** (bit 15 of the first word clear):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u16le | Depth, cm (bits 0..14) |
| 2 | i16le | Temperature, 1/10 °C |
| 4 | 14 bytes | Only for "A" key variants. Content unknown |

Each sample advances time by the interval. The first sample is at 1 × interval.

**Event record** (bit 15 set):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u16le | `0x8000 | type` |
| 2 | 4 bytes | Probably a timestamp. Not used |
| 6 | variable | Payload, by type |

| Type | Meaning | Payload |
|------|---------|---------|
| 0 | Normal / no-op | none |
| 1 | Bookmark | none |
| 2 | Depth alarm | none |
| 3 | Time alarm | none |
| 4 | Ascent-rate alarm | none |
| 5 | Deco stop | u16le stop depth, cm. 0 means no-deco |
| 6 | Deco stop violated | none |
| 7 | Gas switch | u8 O2 %, u8 He % |
| 8 | Setpoint | u8, 1/10 bar |
| 9 | Bailout on | none |
| 10 | Bailout off | none |
| 11 | Emergency on | none |
| 12 | Emergency off | none |
| 13 | Gas lost | none |
| 14 | Safety stop | none |
| 15 | Tank pressure | u16le transmitter ID, u16le pressure (1/100 bar), then 2 bytes of unknown meaning. The 2 extra bytes are absent with key 0x67488403 |
| 16 | Transmitter table | 11 × u16le transmitter IDs (index = tank number) |

Unknown types have no payload. Event values apply to the next sample. Distinct transmitter
IDs define tanks (up to 11). Distinct O2/He pairs define gas mixes (up to 11).

### Date and time

Separate u16 fields (see the header). Day and month are zero-based. Local time, no time zone.

Confidence: **likely** for the fields listed by both references (time, counts, pointer,
depths, temperatures, interval, date, event sizes for types 7, 15 and 16). **uncertain**
for the other fields.

## Model-specific variations

- **Xen:** 80-byte header (51 per page). No mode or deco model byte.
- **Xeo, Lynx, Kaon:** 96-byte header (42 per page).
- **"A" key variants** (some Xeo, all Kaon): 16-byte sample records.
- **Lynx key 0x67488403:** tank pressure events carry 4 payload bytes instead of 6.
- **Lynx:** optional DOWNLOADGASX gas list.

## Open questions

1. Xen header CRC coverage: 74 or 76 bytes after the key?
2. Does the device require per-byte pacing of commands, or a minimum number of `AA`
   wake-up bytes?
3. What are the 14 extra sample bytes in "A" variants? Both references disagree about the
   sample size for Kaon and Xeo.
4. What does the 4-byte event prefix mean?
5. Which header bytes are left unexplained (40..43, 50, 54, 60..91)? What are the last
   bytes of MOREINFO?
6. What is the role of the begin index at 0x46, and of the dive ID limit from MOREINFO?
7. What is the format of the DOWNLOADGASX reply, and is it supported on non-Lynx models?
8. Key 0x83328402 is used as a Xen fallback by one reference but is not listed by the other.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference. No code copied.
- Greg Cook, "Catalogue of parametrised CRC algorithms" (RevEng),
  https://reveng.sourceforge.io/crc-catalogue/
