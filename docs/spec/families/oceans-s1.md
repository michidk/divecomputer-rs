# Oceans S1

The Oceans S1 is a dive computer for scuba and freediving with a Bluetooth Low Energy link.
The host sends plain-text commands terminated by a newline. Short answers come back inline
as text. Bulk data (the dive list and individual dives) is announced as such and then sent
over an XMODEM-CRC-like block transfer with 512-byte blocks. The dive data itself is plain
text, one sample per line.

## Devices

| Vendor | Model | Model code | Transports | BLE name |
|--------|-------|------------|------------|----------|
| Oceans | S1 | 0 | BLE | starts with `S1` (case-insensitive prefix) |

No numeric model code or serial number is reported.

Confidence: **likely**.

## Transport parameters

- BLE. The service and characteristic UUIDs are not publicly documented (see
  [Open questions](#open-questions)).
- Receive timeout: 4000 ms. Flush input after connecting.
- A command response is expected in a single notification of up to 256 bytes. An XMODEM block
  (517 bytes) spans several notifications. The receiver accumulates bytes until the block
  is complete.

Confidence: **likely**.

## Session

No handshake. Send `version` first to read the firmware version. There is no logout command.

## Commands

A command is ASCII: the command word, then optionally a space and arguments, then `\n`.

The response is one text line, optionally followed by `\r`/`\n` characters. It starts with
the command word, followed by either:

- `>ok`, then optional spaces, then the inline result text; or
- `>xmr`, with nothing after it. The result follows as an XMODEM transfer (see
  [Framing](#framing-and-integrity)).

Any other response is an error.

| Command | Arguments | Response | Meaning |
|---------|-----------|----------|---------|
| `version` | — | `version>ok <major>.<minor> <hex>` | firmware version and an unidentified hexadecimal value |
| `dllist` | — | `dllist>xmr`, then XMODEM | the dive list (same text format as a dive, without samples) |
| `dlget` | `<first> <end>` | `dlget>xmr`, then XMODEM | dives `first` up to but not including `end`. Known hosts fetch one dive with `<n> <n+1>` |
| `utc` | `<seconds>` | `utc>ok` | set the clock (see [Date and time](#date-and-time)) |

Example exchange:

```
H→D  "version\n"
D→H  "version>ok 1.6 2a4f\r\n"        (values illustrative)
H→D  "dlget 12 13\n"
D→H  "dlget>xmr\r\n"   then the XMODEM transfer of dive 12
```

Confidence: **likely**.

## Framing and integrity

### XMODEM transfer

1. The host sends `43` (`'C'`) to request CRC mode.
2. The device sends blocks:

   ```
   01  seq  (0xFF − seq)  <512 data bytes>  crc_hi crc_lo
   ```

   - `01` (SOH) starts every block, even though the block carries 512 bytes. Classic XMODEM
     uses SOH for 128-byte blocks and STX for 1024-byte blocks.
   - `seq` starts at 1 and increments per block, wrapping mod 256. Byte 2 must equal
     255 − seq.
   - CRC: **CRC-16/XMODEM** (width 16, poly 0x1021, init 0x0000, refin false, refout false,
     xorout 0x0000) over the 512 data bytes, sent big-endian. Check value for `"123456789"` =
     `0x31C3`. See [../common/checksums.md#crc-16-xmodem](../common/checksums.md#crc-16-xmodem).
3. The host answers each good block with `06` (ACK).
4. The device ends with `04` (EOT). The host acknowledges with `06`.

Known hosts treat a bad block as a fatal error. NAK (`15`) and cancel (`18`) retransmission
handling is not documented.

The concatenated data is text padded at the end. Strip trailing padding and newlines. The
padding byte value is not documented; known hosts only strip line-break characters
(see [Open questions](#open-questions)).

Confidence: **likely**.

## Memory layout

No raw memory access. Dives are addressed by number.

## Download procedure

```
version()
list = dllist()                       # text, parsed as below
for each "dive" block in the list:    # keep dives newer than the fingerprint
    if timestamp > last_known_timestamp: wanted.append(number)
for number in wanted, highest number first:
    text = dlget(number, number + 1)
    fingerprint = u64be(timestamp of this dive)
```

The fingerprint is built by the host: the dive's start timestamp (from the `dive` line) as an
8-byte big-endian integer. Incremental download keeps dives whose timestamp is greater than
the stored one.

Confidence: **likely**.

## Dive data format

The dive text is a series of lines separated by `\r` and/or `\n`. Leading spaces are
ignored, and empty lines are skipped. Line types are recognised by their first word. Test
`divelog`, `dive` and `enddive` in an order that cannot confuse them (`divelog` and `dive`
share a prefix).

| Line | Fields | Meaning |
|------|--------|---------|
| `divelog v1,<n>s/sample` | n = sample interval in s (> 0) | start of the log, format version 1 |
| `dive <num>,<mode>,<o2>,<time>` | dive number; mode 0 = scuba, 1 = apnea; O2 %; start time (decimal, 64-bit) | start of a dive |
| `<depth>,<temp>,<events>` | depth in cm (unsigned); temperature in °C (signed integer); event bitmask (decimal) | one sample, one interval after the previous one |
| `continue <depth>,<seconds>` | depth in cm; surface time in s | the dive resumed after a surface interval. Insert ⌊seconds / interval⌋ surface samples (depth 0), then one sample at `depth` |
| `enddive <maxdepth>,<divetime>` | max depth in cm; dive time in s | end of a dive |
| `endlog` | — | end of the log |

Defaults: sample interval 10 s if no `divelog` line is present.

Sample event bitmask:

| Bit | Value | Meaning |
|-----|-------|---------|
| 0 | 0x0001 | dive started |
| 1 | 0x0002 | dive ended |
| 2 | 0x0004 | dive resumed |
| 3 | 0x0008 | ping sent |
| 4 | 0x0010 | ping received |
| 5 | 0x0020 | in a decompression stop |
| 6 | 0x0040 | in a safety stop |
| 7 | 0x0080 | battery low |
| 8 | 0x0100 | backlight on |

Example (illustrative):

```
divelog v1,10s/sample
dive 12,0,32,1700000000
120,24,1
350,23,0
…
enddive 1840,2460
endlog
```

### Date and time

The `dive` start time and the `utc` command argument are seconds since 1970-01-01 00:00:00,
counted in the device's **local** wall-clock time. Read the value as if it were UTC to get
local time; no time zone is stored. To set the clock, take the host's local date and time,
convert it to a seconds count as if it were UTC, and send `utc <seconds>`.

Confidence: **likely**. The meaning of the `version` hex field and of the ping events is
**uncertain**.

## Model-specific variations

None known.

## Open questions

1. GATT UUIDs and write type.
2. XMODEM padding byte (classically `1A`). It is unclear whether hosts must strip it.
3. Device behaviour on NAK, and whether the device ever sends STX/1024-byte blocks.
4. The meaning of the third `version` field.
5. Whether `dllist` includes sample lines or only `dive`/`enddive` pairs.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- XMODEM/CRC protocol description (Chuck Forsberg, "XMODEM/YMODEM Protocol Reference", 1988),
  <http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt>.
