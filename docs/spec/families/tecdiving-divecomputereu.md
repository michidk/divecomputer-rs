# Tecdiving DiveComputer.eu

The Tecdiving DiveComputer.eu is a technical/CCR dive computer. It connects through a serial
cable or Bluetooth Classic (SPP). Every packet starts with `7E` and carries a 32-bit length,
a command byte, the data, a CRC-16 and two zero bytes. The host asks for a logbook of 7-byte
dive summaries. It then fetches each dive as a 100-byte header followed by 8-byte samples,
sent in packets of up to 1000 samples. The device can be slow to answer: the first byte of a
response may take 6-8 seconds.

## Devices

| Vendor | Model | Model code | Transports |
|--------|-------|------------|------------|
| Tecdiving | DiveComputer.eu | 0 | serial, Bluetooth Classic |

- Bluetooth Classic name: `DiveComputer` (exact, case-insensitive).
- Serial: the USB-serial chip and VID:PID are not known.

Confidence: **likely**.

## Transport parameters

- 115200 baud, 8N1, no flow control (serial and Bluetooth SPP).
- Wait 100 ms and flush buffers after opening.
- Wait 300 ms before sending every request.
- Read timeout: 1000 ms per read. While waiting for the start byte `7E`, keep retrying after
  timeouts, for about 15 s in total (known hosts: 14 retries of 1 s). Reset the retry count
  whenever a non-`7E` byte arrives, and drop that byte.
- Read the payload in pieces of at most 1000 bytes.

Confidence: **likely**.

## Session

1. Open the line and flush.
2. Send **init** (`53`). The device answers with response code `56` and 56 bytes of device
   information. The serial number is the 32-bit value made of the `u16be` at offset 0x22 (high
   half) and the `u16be` at offset 0x26 (low half). The other bytes are not decoded.
3. Download.
4. Send **exit** (`59`) before closing. No response is read.

Confidence: **likely**.

## Commands

| Function | Cmd | Request data | Response code | Response data |
|----------|-----|--------------|---------------|---------------|
| Init | 53 | — | 56 | 56 bytes device info |
| Logbook | 57 | — | 57 | `u16be` count N, then N × 7-byte summaries |
| Dive | 58 | `u16be` logbook number (1 = first entry of the logbook) | 51, then one or more 52 | header packet (100 bytes), then profile packets |
| Exit | 59 | — | none | |

The logbook entries are in newest-first order. Logbook number 1 is the newest dive. The
7-byte summary equals the first 7 bytes of that dive's header.

Confidence: **likely**.

## Framing and integrity

```
7E  00  len[4] (u32le)  cmd  data[len]  crc_hi crc_lo  00 00
```

- Byte 1 is a type byte. It is always `00`; a receiver rejects any other value.
- `len` counts the data bytes only.
- CRC: **CRC-16/XMODEM**: width 16, poly 0x1021, init 0x0000, refin false, refout false,
  xorout 0x0000. It covers bytes 1..(7 + len − 1), that is everything after `7E` up to the end
  of the data. It is sent big-endian and followed by two `00` bytes, which a receiver checks.
  Check value for `"123456789"` = `0x31C3`. See
  [../common/checksums.md#crc-16-xmodem](../common/checksums.md#crc-16-xmodem).
- Host requests use the same framing, with at most 2 data bytes.

Worked examples:

```
H→D  7E 00 00 00 00 00 53 6A 96 00 00             init
H→D  7E 00 00 00 00 00 57 2A 12 00 00             logbook
H→D  7E 00 02 00 00 00 58 00 01 87 AD 00 00       dive, logbook number 1
```

Confidence: **likely**.

## Memory layout

No raw memory access.

## Download procedure

```
init(); log = logbook()
N = u16be(log[0..1]); require len(log) == 2 + 7 × N
k = number of leading summaries before the one equal to the stored fingerprint
for i in 0 .. k-1:
    send dive(i + 1)
    hdr = receive(code 51, exactly 100 bytes)
    ns = u32be(hdr[36..39]); want = 100 + 8 × ns
    data = hdr
    while len(data) < want:
        chunk = receive(code 52, exactly min(want − len(data), 8000) bytes)
        data += chunk
    require data[0..6] == log[2 + 7i .. 2 + 7i + 6]
exit()
```

Fingerprint: the 7-byte summary, which is the first 7 bytes of the dive.

Confidence: **likely**.

## Dive data format

Header and profile fields are **big-endian**.

### Header

100 bytes:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x00..0x06 | 7 bytes | | summary / fingerprint (includes the date bytes below) |
| 0x02 | u8 | | year − 2000 |
| 0x03 | u8 | | month |
| 0x04 | u8 | | day |
| 0x05 | u8 | | hour |
| 0x06 | u8 | | minute |
| 0x07 | u8 | | second |
| 0x0E | u16be | mbar | surface pressure |
| 0x11 | i8 | °C | surface temperature |
| 0x17 | u16be | min | dive time |
| 0x1B | u16be | cm | average depth |
| 0x1D | u16be | 0.1 m | maximum depth |
| 0x24 | u32be | | number of samples |
| 0x29 | i8 | °C | minimum temperature |
| 0x2A | i8 | °C | maximum temperature |
| 0x2F | u8 | s | sample interval |
| other | | | not decoded |

### Samples

8 bytes each:

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| +0 | i8 | °C | temperature |
| +1 | u8 | 0.1 bar | ppO2 |
| +2 | u16be | 0.1 m | depth |
| +4 | u8 | 0.1 bar | setpoint |
| +5..+7 | | | not decoded |

Time advances by the header sample interval per sample, starting at one interval.

### Date and time

Local time, no zone stored.

Confidence: **likely** for the layout (one implementation). **Uncertain** for the average
and maximum depth units. The reference implementation divides them by 100 and by 10 with
integer arithmetic, which truncates to whole metres. Read the raw values as 0.01 m and 0.1 m
and keep the fractions.

## Model-specific variations

None known.

## Open questions

1. Serial cable chip and VID:PID.
2. Meaning of bytes 0x00..0x01 of the header/summary (possibly a dive number).
3. Sample bytes +5..+7 and the undecoded header bytes (gas, decompression, battery).
4. Whether a dive can have more than one ppO2 sensor value.
5. Contents of the 56-byte init response beyond the serial number.

## References

- libdivecomputer (LGPL-2.1), <https://www.libdivecomputer.org/>, used as a factual reference;
  no code copied.
- DiveComputer.eu product site, <https://www.divecomputer.eu/>.
