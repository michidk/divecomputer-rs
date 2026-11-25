# Cochran Commander and EMC

Cochran Undersea Technology's Commander and EMC computers share one serial protocol. It
has no packet framing and no checksums. Raw command bytes are sent one at a time, and the
device answers with a block of raw memory of the requested length. Bulk memory reads
start at 9600 baud, and both ends then switch to a model-specific high speed (115200 or
850000 baud) for the data. The device keeps a logbook ring (fixed-size dive summaries)
and a profile ring (variable-length, delta-encoded samples interleaved with event bytes).
Profile data includes "inter-dive" events recorded while the diver was on the surface.

The serial link uses no encryption or obfuscation. All memory is read as plain bytes.

No public vendor documentation is known. The only source is libdivecomputer, so most
sections here are **likely** or **uncertain**.

## Devices

| Vendor | Model | Model code | Model string (ID block 0x3D..0x3F) | Notes |
|--------|-------|------------|------------------------------------|-------|
| Cochran | Commander TM | 0 | `0A 31 32` (`\x0A` "12") | oldest; SRAM; 9600 baud only |
| Cochran | Commander I (serial numbers below 21000) | 1 | `11 32 31` (`\x11` "21") | 115200 high speed |
| Cochran | Commander II (Air/Nitrox) | 2 | `11 32 32` (`\x11` "22") | 115200 high speed |
| Cochran | EMC-14 | 3 | "730", "731" | 850000 high speed |
| Cochran | EMC-16 | 4 | "A30", "A31" | 850000 high speed |
| Cochran | EMC-20H | 5 | "230", "231", `40 33 30` (`\x40` "30") | 850000 high speed |

Transport: RS-232 serial through Cochran's interface cable, commonly through a USB-serial
adapter that can run at 850000 baud. There are no USB IDs, Bluetooth names or IrDA
nicknames.

Confidence: **likely**.

## Transport parameters

| Phase | Baud | Format | Flow control |
|-------|------|--------|--------------|
| wake-up, ID, config, command bytes | 9600 | 8N2 (two stop bits) | none |
| bulk data, Commander I/II | 115200 | 8N2 | none |
| bulk data, EMC-14/16/20H | 850000 | 8N2 | none |
| bulk data, Commander TM | 9600 (no switch) | 8N2 | none |

- Read timeout: 5000 ms.
- Command bytes MUST be sent one byte at a time, 16 ms apart. The device has no input
  buffer and drops a command sent as one burst.
- The baud switch: the host sends the last command byte at 9600, waits 45 ms, reconfigures
  its port to the high rate, and reads the data.
- The host may read in chunks of any size. libdivecomputer reads 1024 bytes at a time
  for progress reporting.

Confidence: **likely**.

## Session

**Wake-up / heartbeat.** This step is done at open and again before every bulk read:

1. Configure 9600 8N2.
2. Assert BREAK for 16 ms, then clear it.
3. Flush the input buffer. The device sends heartbeat bytes, and old ones must be
   discarded.
4. Read 1 byte. It MUST be `AA` (heartbeat). Anything else is a protocol error.

**Identification.** At 9600 baud (no speed switch), read 67 bytes with the low-speed read
command (see Commands) from address `0x00FF9D`:

```
H→D  05 9D FF 00 43 00      (one byte at a time, 16 ms apart)
D→H  <67 bytes>
```

If the block does not begin with ASCII `"(C)"`, the device is a Commander. Repeat the read
from address `0x007FBD`:

```
H→D  05 BD 7F 00 43 00
D→H  <67 bytes>
```

The 3 bytes at offset 0x3D of the block select the model (see Devices). An unknown string
means the model is unsupported.

**Configuration.** Read 512-byte config pages at 9600 baud. Command `96 <page>`
(2 bytes) for pages 0 and 1, which give 1024 bytes in total. The Commander TM has a
single page and takes only the 1-byte command `96`.

There is no explicit shutdown.

Confidence: **likely**.

## Commands

| Command | H→D bytes | Arguments | D→H response | Used by |
|---------|-----------|-----------|--------------|---------|
| Low-speed read | `05 a0 a1 a2 n0 n1` | 24-bit LE address, 16-bit LE length | n bytes at 9600; length 0 returns 32768 bytes | ID block (all); all data on the TM |
| Commander high-speed read | `15 a0 a1 a2 n0 n1 n2 04` | 24-bit LE address, 24-bit LE length, `04` terminator | n bytes at 115200 | Commander I/II |
| EMC high-speed read | `15 a0 a1 a2 a3 n0 n1 n2 n3 05` | 32-bit LE address, 32-bit LE length, `05` terminator | n bytes at 850000 | EMC-14/16/20H |
| Config page | `96 p` (TM: `96`) | page number 0 or 1 | 512 bytes at 9600 | all |

Every bulk read (low- or high-speed) follows the same sequence: wait 550 ms, run the
wake/heartbeat sequence, send the command bytes 16 ms apart, then (high-speed only) wait
45 ms and switch baud, then read exactly n bytes. The reference retries a failed or
timed-out read up to 2 more times.

Example: EMC read of 512 bytes from address 0:

```
H→D  15 00 00 00 00 00 02 00 00 05   (bytes 16 ms apart, at 9600 8N2)
     (host waits 45 ms and switches to 850000 8N2)
D→H  <512 bytes>
```

Confidence: **likely**.

## Framing and integrity

None. Responses are raw memory with no header, length or checksum. The host can detect
errors only by timeouts and by checking that the data is plausible (pointers inside the
ring, dive numbers consistent).

## Memory layout

### Address map per model

Addresses below are device addresses. Pointers stored in the logbook and config are
relative to the *logbook base* (column "base"), which is 0 except on the TM.

| Model | base | Logbook area | Entries × size | Profile ring |
|-------|------|--------------|----------------|--------------|
| Commander TM | 0x010000 | 0x010000–0x01232B | 100 × 90 | 0x01232B–0x018000 |
| Commander I | 0 | 0x000000–0x020000 | 512 × 256 | 0x020000–0x100000 |
| Commander II | 0 | 0x000000–0x020000 | 512 × 256 | 0x020000–0x100000 |
| EMC-14 | 0 | 0x000000–0x020000 | 256 × 512 | 0x022000–0x200000 |
| EMC-16 | 0 | 0x000000–0x080000 | 1024 × 512 | 0x094000–0x800000 |
| EMC-20H | 0 | 0x000000–0x080000 | 1024 × 512 | 0x094000–0x1000000 |

The logbook area is a ring of fixed-size entries; entry k is at base + k × size. The
profile ring follows it directly, so one read starting at the logbook base covers both.
A full dump is the read of [base, profile end).

### Integer encodings

| Model group | Config words | Pointers inside logbook entries |
|-------------|--------------|----------------------------------|
| Commander TM | u16be dive count. 32-bit values are "word big-endian": bytes b0 b1 b2 b3 mean (b2 b3) as the high word and (b0 b1) as the low word, each word big-endian. Value = b2·2²⁴ + b3·2¹⁶ + b0·2⁸ + b1 | u16le |
| Commander I/II | as TM | u32le |
| EMC | u16le / u32le | u32le |

### Config fields (offset into the 1024-byte config; 512 for TM)

| Field | TM | Commander I/II | EMC |
|-------|----|----------------|-----|
| total dive count (u16) | 0x146 | 0x046 | 0x0D2 |
| profile write head: end of the newest profile data, relative to base | 0x158 | 0x06C | 0x13E |
| end of the last inter-dive data (not needed for download) | – | 0x070 | 0x142 |
| serial number (u32) | 0x15C | 0x0AA | 0x1E6 |

On the TM, the reference reads the write head at 0x158 as u16be in one place and as a
word-big-endian u32 in another (see Open questions).

### Pointer fields inside each logbook entry

| Field | TM | Commander I | Commander II | EMC |
|-------|----|-------------|--------------|-----|
| fingerprint (offset, length) | 15, 4 | 12, 4 | 0, 6 | 0, 6 |
| profile start pointer | 0 (u16le) | 0 | 6 | 6 |
| pointer to the pre-dive data start | 0 | 28 | 30 | 30 |
| profile end pointer (`FFFFFFFF` = dive end not recorded) | none; use the next dive's start or the write head | 128 | 128 | 256 |
| dive number (u16le) | 20 | 68 | 70 | 86 |

Confidence: **likely** for the address map and offsets; **uncertain** for the TM
encodings.

## Download procedure

```
open; wake; read ID; pick model layout; read config
count = config.dive_count
if count == 0: done
n_entries = min(count, entries_per_ring)
read n_entries × entry_size bytes starting at the logbook base
if count <= entries_per_ring: head = count  else: head = count mod entries_per_ring
write_head = base + config.profile_write_head

# Work out which dives still have profile data (the profile ring may have wrapped
# past older dives even though their logbook entries survive):
capacity = profile_end − profile_begin
prev = write_head
for i in 0 .. n_entries−1:                  # newest to oldest
    idx = (head − 1 − i) mod entries_per_ring
    if entry[idx].fingerprint == known_fingerprint: stop scanning; newer dives only
    pre = base + entry[idx].pre_pointer
    capacity −= ring_distance(pre, prev); prev = pre
    if capacity < 0: dives idx and older have no profile left

# Read and emit, newest first. One backward-moving read cursor starts at write_head:
cursor_end = write_head
for each dive newer than the fingerprint dive, newest first:
    start = base + entry.profile_start
    end   = base + entry.profile_end  (TM: cursor_end)
    if profile still present:
        size = ring_distance(start, end)          # dive samples
        tail = ring_distance(end, cursor_end)     # surface events after the dive
        read the size + tail bytes ending at cursor_end from the profile ring
        cursor_end = start
    blob = entry bytes ++ first `size` profile bytes   (the tail is dropped)
    emit blob, fingerprint = entry bytes at the fingerprint offset
```

The flash models erase ahead of the write head. When counting capacity, the reference
also subtracts the gap between the newest dive's end pointer and
`(write_head rounded down to 4096) + 0x2000`. On the TM, which uses SRAM, it subtracts
the gap up to the write head itself.

`ring_distance(a, b)` = (b − a) mod (profile_end − profile_begin), with a == b giving 0.
Pointers outside the profile ring make a profile invalid; emit the logbook entry only.

Commander TM only: battery-backed SRAM can scramble when power is lost. Skip an entry
whose start pointer is outside the ring, or whose dive number mod 100 differs from its
ring index.

A dive whose end pointer is `FFFFFFFF` was not closed properly. Take the end as the next
newer dive's start pointer, or the write head for the newest dive.

Incremental download: stop at the entry whose fingerprint bytes equal the stored
fingerprint. Fingerprints: TM and Commander I use the 4-byte dive timestamp; Commander II
and EMC use the 6-byte date/time.

Confidence: **likely** (single source).

## Dive data format

The dive blob is the logbook entry (header) followed by the profile bytes.

### Header

Offsets in bytes into the logbook entry; "–" = not present.

| Field | Type / raw unit | TM | Cmdr I | Cmdr II | EMC |
|-------|-----------------|----|--------|---------|-----|
| start date/time | see Date and time | 15 | 8 | 0 | 0 |
| water conductivity | u8, bits 0–1 | – | 24 | 24 | 24 |
| start depth | u16le; ¼ ft (Cmdr), 1/256 ft (EMC) | – | 54 | 56 | 42 |
| start temperature | u8, °F | 83 | 43 | 45 | 55 |
| dive number | u16le | 20 | 68 | 70 | 86 |
| altitude | u8, 250 ft steps | – | 73 | 73 | 89 |
| sample interval | u8, s | 72 | (1 s fixed) | (1 s fixed) | (1 s fixed) |
| end temperature | u8, °F | – | 153 | 153 | 293 |
| dive time | u16le, min; `FFFF` = n/a | 57 | 166 | 166 | 304 |
| max depth | u16le, ¼ ft; `FFFF` = n/a | 49 | 168 | 168 | 306 |
| average depth | u16le, ¼ ft; `FFFF` = n/a | 51 | 170 | 170 | 310 |
| O₂ of mix 1, 2 (EMC: 1–3) | u16le each, percent × 256 | 74 | 210 | 210 | 144 |
| He of mix 1–3 | u16le each, percent × 256 | – | – | – | 164 |
| minimum temperature | u8; °F = raw/2 + 20; `FF` = n/a | 82 | 232 | 232 | 403 |
| maximum temperature | u8; °F = raw/2 + 20; `FF` = n/a | – | 233 | 233 | 407 |

Conversions:

- feet → m: × 0.3048. °F → °C: (F − 32) / 1.8.
- Gas: fraction = raw / 256 / 100. Example: `00 20` (u16le 0x2000 = 8192) → 32.0 %.
- Water: conductivity bits 0–1 equal to 0 mean fresh water; any other value means salt.
  libdivecomputer assumes density = 1000 + 12.5 × value kg/m³ (value 2 → 1025). This is
  an assumption, not a documented mapping.
- Altitude → surface pressure: h = raw × 250 ft × 0.3048; P bar =
  1.01325 × (1 − 2.25577×10⁻⁵ × h)^5.25588 (standard atmosphere). Raw 4 → 0.9772 bar.

### Samples: Commander TM

The first 2 profile bytes give initial values: temperature in ½ °F and depth in ½ ft. Then
each byte is one of:

| Byte pattern | Meaning |
|--------------|---------|
| `0xxxxxxx` | depth sample: bit 6 = direction (1 = shallower), bits 0–5 = change in ½ ft. The clock advances by the header interval |
| `100xxxxx` | temperature change: bit 4 = direction (1 = colder), bits 0–3 = change in ½ °F. Time does not advance |
| `1xxxxxxx` with bits 5–6 not both 0 | single-byte event (see the event table). Time does not advance |

On the TM, ceiling events `AB`/`AD` are one byte long and carry no duration.

### Samples: Commander I/II and EMC

The sample period is 1 s. Sample record size: 2 bytes (Commander), 3 bytes (EMC). Before
each record, check the first byte. If bit 7 is set, it starts an event (see table) and is
not a sample. Skip the event's length and look again; events do not advance the clock.

| Byte | Meaning |
|------|---------|
| 0 | depth change: bit 6 = direction (1 = shallower), bits 0–5 = change in ¼ ft. Depth = start depth (header) + running sum / 4 ft |
| 1 | even seconds (0, 2, 4, …): ascent rate, bit 7 = sign (set = positive), bits 0–6 in ¼ ft units; odd seconds: temperature, °F = raw/2 + 20 |
| 2 (EMC only) | 24-second cycle; see below |

EMC byte 2, by the sample's position in a 24-sample cycle (sample index mod 24):

| Position | Content |
|----------|---------|
| 0–19 | tissue loading values (not decoded) |
| 20 (low), 21 (high) | value + 1 = minutes: the no-deco limit when not in deco, otherwise the time at the deepest stop |
| 22 (low), 23 (high) | value + 1 = minutes: total deco time (meaningful only in deco) |

"In deco" runs from event `C5` until event `D8`. The initial values come from the
header: start depth, start temperature, mix 1.

### Events

Event codes have bit 7 set. Each code below is 1 byte long, except where a length is
given.

| Code | Meaning |
|------|---------|
| A8 | entered post-dive surface mode (start of surface interval) |
| A9 | left post-dive surface mode |
| AB | ceiling 10 ft deeper. 5 bytes: code, u16le first-stop minutes, u16le total stop minutes (Commander/EMC) |
| AD | ceiling 10 ft shallower. 5 bytes, same layout as AB |
| B5 | less than 5 minutes of air left (deco) |
| BD | back to normal ppO₂ setting |
| BE | ceiling deeper than 60 ft |
| C0 | switched to 21 % FO₂ mode (seen on surfacing) |
| C1 | ascent rate above limit, start |
| C2 | low battery |
| C3 | CNS oxygen toxicity warning |
| C4 | depth beyond user limit |
| C5 | decompression obligation begins |
| C7 | entered gauge mode (lock-out) |
| C8 | ppO₂ too high |
| CC | low cylinder 1 pressure |
| CD | switched to deco blend (treated as mix 2) |
| CE | no-deco limit warning, start |
| CF | oxygen toxicity, start |
| D0 | breathing rate alarm, start |
| D3 | low gas 1 flow rate |
| D6 | depth above ceiling, start |
| D8 | decompression obligation ends |
| E1 | ascent rate warning, end |
| E2 | low SBAT battery |
| E3 | switched to FO₂ mode |
| E5 | switched to ppO₂ mode |
| EE | no-deco limit warning, end |
| EF | switched to blend 2 (mix 2) |
| F0 | breathing rate alarm, end |
| F3 | switched to blend 1 (mix 1) |
| F6 | depth above ceiling, end |

The meaning of each code is libdivecomputer's interpretation. An unknown code with bit 7
set is skipped as 1 byte.

**Surface (inter-dive) records.** After the end of a dive's samples, the profile ring
holds records written on the surface. Their first byte is below 0x80. A normal download
cuts the profile at the dive's end pointer and never needs to parse them. Their lengths
are needed only to cut back an unclosed dive:

| First byte | Length, Commander | Length, EMC |
|------------|-------------------|-------------|
| 00 | 17 | 19 |
| 01 | 21 | 23 |
| 02 | 18 | 20 |
| 03 | 17 | 19 |
| 06 | 19 | 21 |
| 07 | 19 | 21 |
| 08 | 19 | – |
| 09 | 19 | – |
| 0A | 19 | 21 |
| 0B | 21 | 21 |
| 0C | 19 | – |
| 0D | 19 | – |
| 0E | 19 | – |
| 0F | – | 19 |
| 10 | 21 | 21 |

For an unclosed dive (end pointer `FFFFFFFF`), the reference:

1. walks back from the guessed end over records that match this table, taking the longest
   consistent match;
2. while parsing, stops at a byte `FF` (erased memory) or `A8`, or at `E3`/`F3` after the
   first 2 seconds (these appear at the start of the next dive).

The summary fields are unreliable for such dives.

### Date and time

| Model | Encoding |
|-------|----------|
| TM, Commander I | u32le seconds since 1992-01-01 00:00 |
| Commander II | 6 bytes: minute, second, day, hour, year (2 digits), month |
| EMC | 6 bytes: second, minute, hour, day, month, year (2 digits) |

Byte fields are binary, not BCD. Two-digit year: > 91 → 1900 + yy, else 2000 + yy. No
time zone is stored.

For the seconds count, the reference adds 694242000 to get a Unix time. That is
1992-01-01 05:00 UTC, which is midnight US Eastern Standard Time. It then converts the
result with the host's local time zone. Whether the device's epoch is local midnight
(most likely) or Eastern time is not established.

Confidence: **likely** for the sample and header structure; **uncertain** for event
meanings, the EMC tissue/NDL cycle, the conductivity → density mapping and the TM epoch.

## Model-specific variations

- The TM has a 90-byte logbook entry, 1-byte samples, a variable sample interval, no
  surface records, and no high-speed transfer.
- Commander I and II differ in date encoding (tick count vs fields), fingerprint
  position, and logbook offsets shifted by 2 bytes.
- The EMC-14/16/20H differ only in logbook and profile sizes; their dive format is the
  same.

## Open questions

1. TM write-head encoding: u16be or word-big-endian u32 at config 0x158?
2. Why pointers in Commander logbook entries are read as u32le when config words are
   word-big-endian. This needs device data to confirm.
3. The exact high baud rate of the EMC: the reference uses 850000; a figure of about
   850400 has also been mentioned. USB-serial adapters must support non-standard rates.
4. Meanings of the surface-record types and of the TM's interpretation of events.
5. Time zone of the 1992 epoch.
6. Cochran's PC software stores downloaded logs in its own file formats, which are
   reported to be obfuscated. Those formats are outside this specification. The serial
   protocol itself shows no encryption.

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
- U.S. Standard Atmosphere 1976 (barometric formula used for the altitude conversion).
