# ReefNet Sensus, Sensus Pro and Sensus Ultra

ReefNet's Sensus recorders are depth/temperature loggers without decompression functions
(gauge-only). Three hardware generations use three different serial protocols, so this
document has one section per variant. All three share one design idea: the device keeps
no calendar clock, only a free-running seconds counter. The host converts dive timestamps
to calendar time by comparing them with the counter value it reads during the handshake.

ReefNet publishes a developer's guide for each generation (see References). Facts that
appear in those guides are marked **verified**. Behaviour seen only in libdivecomputer is
marked **likely**.

| Variant | Generation | Line | Memory returned | Integrity |
|---------|------------|------|-----------------|-----------|
| Sensus (original, "silver") | 1 | 19200 8N1 | 32768 bytes, one burst | sum16 |
| Sensus Pro | 2 | 19200 8N1 | 56320 bytes, one burst | CRC-16/IBM-3740 |
| Sensus Ultra | 3 | 115200 8N1 | up to 2080768 bytes, in 512-byte pages | CRC-16/IBM-3740 per packet |

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| ReefNet | Sensus | 1 | serial (RS-232 via ReefNet cable) | product code ASCII `'1'` in handshake |
| ReefNet | Sensus Pro | 2 | serial (RS-232 via cradle) | product code `0x02` in handshake |
| ReefNet | Sensus Ultra | 3 | serial (RS-232 via download unit, or USB-serial adapter) | major version `0x03` in handshake |

None of these devices has a USB, Bluetooth or IrDA identity of its own. They are reached
through a plain serial port or a generic USB-serial adapter. ReefNet sold its own
USB-serial adapter, and CP210x, FTDI and PL2303 chips have all been used in practice; see
[../transports/serial.md](../transports/serial.md) for chip setup.

The original Sensus has a single-wire (half-duplex) data link. The host must not transmit
while the device is transmitting. The vendor guide warns that the cable contains
electronics: it is not a plain wire adapter. The Pro cradle and the Ultra download unit
are passive and expose RX, TX and GND only.

Confidence: **verified** (vendor guides).

## Transport parameters

| Variant | Baud | Format | Flow control | Read timeout used by libdivecomputer |
|---------|------|--------|--------------|--------------------------------------|
| Sensus | 19200 | 8N1 | none (no handshake lines used) | 3000 ms |
| Sensus Pro | 19200 | 8N1 | none | 3000 ms |
| Sensus Ultra | 115200 | 8N1 | none at line level; byte-level prompt flow control (see below) | 3000 ms |

The Sensus guide asks for a receive buffer of at least 35000 bytes, because the full dump
arrives as one stream. Another implementation is reported to use a 100 ms read timeout
for the Ultra and to drive DTR low on PL2303 adapters. Neither is needed by the reference
implementation.

Confidence: **verified** for line settings; timeouts **likely**.

---

## Variant 1: Sensus (original)

### Session

1. Open the port at 19200 8N1 and discard pending input.
2. Send a single wake/alert byte. The vendor guide specifies `00`. libdivecomputer sends
   `0A`. Both are reported to work (see Open questions).
3. The device answers with 12 bytes:

   | Offset | Type | Meaning |
   |--------|------|---------|
   | 0x00 | 2 × ASCII | `"OK"` (`4F 4B`) |
   | 0x02 | ASCII digit | product code, `'1'` = Sensus |
   | 0x03 | ASCII digit | product version |
   | 0x04 | u8 | battery: 16 = OK, 64 = low |
   | 0x05 | u8 | current sample interval, s |
   | 0x06 | u16le | serial number (as engraved) |
   | 0x08 | u32le | current device clock, s |

   If no answer arrives within 100 ms, resend the alert byte. Give up after about 1 s:
   either no device is present or its battery is too low.
4. The device now waits for a decision. The host MUST answer: if a device is left waiting
   it may lock up until its battery is removed. Wait at least 10 ms after the last
   handshake byte, then send one of:
   - `40` to start the memory dump;
   - `00` to cancel. Send this when the host decides not to download, for example on
     close.

### Commands

| Command | H→D | Response |
|---------|-----|----------|
| Alert / wake | `00` (vendor guide) or `0A` (libdivecomputer) | 12-byte handshake |
| Dump | `40` | `44 41 54 41` ("DATA") + 32768 bytes + u16le checksum + `45 4E 44` ("END") |
| Cancel | `00` | none |

### Framing and integrity

The dump response is exactly 4 + 32768 + 2 + 3 = 32777 bytes. The checksum is a
[sum16](../common/checksums.md#sum16): the sum of the 32768 data bytes, truncated to
16 bits and starting from 0. It is sent little-endian. The host verifies the `DATA` and
`END` literals as well.

Worked example: data bytes `FF 0A E8 03 00 00 FE 14 50 20` give sum16 = `0x0376`, sent
as `76 03`.

### Memory layout

32768 bytes used as one circular log. There is no pointer to the newest dive. Dives are
found by scanning the dump for headers. The device does not erase memory, so bytes left
from older dives may sit between the end of one dive and the next header.

### Dive data format

Dive header (7 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u8 | start marker `FF` |
| 1 | u8 | sample interval, s |
| 2 | u32le | dive start, device clock seconds |
| 6 | u8 | header end marker `FE` |

Body: depth bytes, each 1 byte. A temperature byte follows the depth byte of sample 0,
sample 6, sample 12 and so on (every sixth sample, starting with the first). The layout
is therefore D T D D D D D D T D …

- Depth: u8, "adjusted" feet of seawater. Gauge depth in fsw = raw − 13 for a dive at sea
  level; the offset of 13 fsw keeps high-altitude dives from underflowing. The reference
  implementation turns raw into absolute pressure as (raw + 20) fsw, with
  1 fsw = 101325/33 Pa, then subtracts the surface pressure (default 101325 Pa) and
  divides by ρ·g (default ρ = 1025 kg/m³, g = 9.80665 m/s²). At sea level this equals
  (raw − 13) × 0.30546 m.
- Temperature: u8, °F. °C = (raw − 32) × 5/9.

End of dive: the dive ends after 17 consecutive depth samples with raw < 16 (that is,
shallower than 3 ft). The vendor says this rule is the only reliable end marker. A
reader MUST NOT read on until the next `FF … FE` header, because leftover bytes may
come first.

Dive duration = number of depth samples × interval. Max depth = largest raw depth
sample, converted as above. Gas mixes: none. Mode: gauge.

### Download procedure

```
handshake; remember (host_now, devclock = handshake.u32@8)
send 0x40; read 32777 bytes; check "DATA", "END", sum16
pos = len - 7
while pos > 0:
    pos -= 1
    if mem[pos] == 0xFF and mem[pos+6] == 0xFE:
        parse samples from pos+7 until the 17-shallow-sample rule ends the dive
        (do not read past the start of the dive found before this one)
        ts = u32le(mem[pos+2..])
        if ts <= last_known_ts: stop            # incremental download
        emit dive bytes [pos, end)
```

Fingerprint: the 4-byte dive timestamp (offset 2). Dives come out newest first.

Confidence: **verified** (vendor guide). The backward scan and the absolute-pressure
conversion are **likely** (libdivecomputer only).

---

## Variant 2: Sensus Pro

### Session

The device sleeps and checks the RX line once a second. To wake it:

- preferred: assert BREAK, wait until bytes start to arrive, clear BREAK, then read 12 bytes; or
- without BREAK support: send a byte (for example `00`) again and again until bytes
  start to arrive.

Waking can take 1–2 s. Handshake packet (12 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u8 | product code, `0x02` |
| 0x01 | u8 | firmware version |
| 0x02 | u8 | battery, 0..255 maps to 0..5 V (the discharge curve is not linear) |
| 0x03 | u8 | sample interval, s |
| 0x04 | u16le | serial number |
| 0x06 | u32le | device clock, s since power-up |
| 0x0A | u16le | CRC of bytes 0x00..0x09 |

After the handshake the device waits 1 s for a one-byte instruction, then goes back to
sleep. The host waits at least 10 ms after the last handshake byte before it sends one.

### Commands

| Command | H→D | Arguments | D→H | Notes |
|---------|-----|-----------|-----|-------|
| Dump | `B4` | – | 56320 bytes + u16le CRC | takes about 30 s |
| Set interval | `B5` | after ≥ 10 ms, 1 byte: interval 1..127 s | none | check the result in the next handshake |

The vendor says other instruction codes exist for factory use only. One of them sets the
serial number, and it works once only. Hosts MUST NOT send unknown codes.

### Framing and integrity

CRC: [CRC-16/IBM-3740](../common/checksums.md#crc-16-ibm-3740) (width 16, poly 0x1021,
init 0xFFFF, refin/refout false, xorout 0x0000). It is sent little-endian after the data
it covers. Vendor check value: ASCII `ReefNet` → `0xEF03`. Standard check value:
`123456789` → `0x29B1`.

Worked handshake example (product 2, firmware 1, battery 0x9A, interval 10 s, serial
12345, clock 86400):

```
D→H  02 01 9A 0A 39 30 80 51 01 00 E0 9A
```

### Memory layout and dive data format

56320 bytes, oldest to newest. The data may begin with the tail of a dive that is being
overwritten.

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u32 | dive start marker `00 00 00 00` |
| 0x04 | u16le | sample interval, s |
| 0x06 | u32le | dive start, device clock seconds |
| 0x0A… | u16le each | samples |
| end | u16 | end marker `FF FF` |

Sample word:

| Bits | Meaning | Conversion |
|------|---------|------------|
| 0–8 | depth, absolute fsw (9 bits) | Pa = raw × 101325/33; depth m = (Pa − P_surface) / (ρ·g) |
| 9–15 | temperature, °F (7 bits) | °C = (raw − 32) × 5/9 |

Duration = sample count × interval.

### Download procedure

Vendor procedure: scan forward one byte at a time for `00 00 00 00`, read interval and
timestamp, then read words until `FF FF`. Repeat. libdivecomputer scans backwards from the
end, so that it emits the newest dive first and can stop at the fingerprint (the 4-byte
timestamp at offset 6). Each backward step looks for the marker, then for `FF FF`, at
or after marker + 10 and before the start of the dive found before it.

Confidence: **verified** (vendor guide); scan direction choice **likely**.

---

## Variant 3: Sensus Ultra

### Overview

2 MiB flash divided into 512-byte pages:

| Segment | Pages | Bytes | Access |
|---------|-------|-------|--------|
| USER | 32 | 16384 | read/write, free-form |
| DATA | 4064 | 2080768 | read-only dive log, chronological |

Integer fields are little-endian. Units used by the device:

- voltage: u16, mV;
- temperature: u16, 0.01 K. °C = raw/100 − 273.15;
- pressure: u16, mbar absolute. The device never reports depth.

Four recording parameters (each u16) control logging:

| Parameter | Default | Valid | Meaning |
|-----------|---------|-------|---------|
| INTERVAL | 10 | 1..65535 | sample interval, s |
| THRESHOLD | 1111 | 1..65535 | dive start pressure, mbar abs (about 3 fsw at sea level) |
| ENDCOUNT | 15 | 1..65535 | number of consecutive samples below THRESHOLD that end a dive |
| AVERAGING | 1 | 1, 2, 4 | raw pressure readings averaged per sample |

### Session

The device wakes once a second. When it finds a live port it sends a 26-byte handshake
packet without being asked:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u16le | version: high byte `0x03` = Ultra, low byte = firmware (≥ 1) |
| 0x02 | u16le | serial number |
| 0x04 | u32le | device clock, s |
| 0x08 | u16le | boot count |
| 0x0A | u32le | clock value at the last boot |
| 0x0E | u16le | lifetime dive count |
| 0x10 | u16le | INTERVAL |
| 0x12 | u16le | THRESHOLD |
| 0x14 | u16le | ENDCOUNT |
| 0x16 | u16le | AVERAGING |
| 0x18 | u16le | CRC of bytes 0x00..0x17 |

As bytes, byte 0 is the firmware version and byte 1 the product (`03`).

**Prompt flow control.** Whenever the device can accept a byte it sends a prompt `A5`.
The host then has a 50 ms window to send exactly one byte. A byte sent outside the window
is ignored. If the host ignores the prompt, the device aborts the operation and sleeps.

Session sequence:

1. Flush host buffers.
2. Read 26 bytes and check the CRC. On a mismatch or timeout, wait about 250 ms, flush,
   and wait for the next handshake. The delay keeps the prompt that follows the handshake
   from being read as the start of the next packet. libdivecomputer gives up after
   3 attempts; another implementation retries up to 9 times.
3. Wait for `A5`, then send the low byte of the 16-bit instruction code.
4. Wait for `A5`, then send the high byte.

```
D→H  <26-byte handshake>
D→H  A5
H→D  21
D→H  A5
H→D  B4          (instruction 0xB421 READ_DATA)
```

Worked handshake example (firmware 1, serial 1234, clock 4600, 1 boot at 0, 5 dives,
default parameters):

```
D→H  01 03 D2 04 F8 11 00 00 01 00 00 00 00 00 05 00 0A 00 57 04 0F 00 01 00 AA 0D
```

### Commands

| Code (u16, sent LSB first) | Name | Arguments (each byte prompted) | Response |
|---------------------------|------|-------------------------------|----------|
| 0xB410 | set interval | u16le value | none; check the next handshake |
| 0xB411 | set threshold | u16le value | none |
| 0xB412 | set end count | u16le value | none |
| 0xB413 | set averaging | u16le value (1, 2 or 4) | none |
| 0xB420 | read USER | – | 32 page packets, top to bottom |
| 0xB421 | read DATA | – | up to 4064 page packets, newest page first |
| 0xB430 | write USER | 16384 data bytes + u16le CRC of them | none |
| 0xB440 | sense | – | 8-byte sense packet within 500 ms |

An invalid code or parameter value makes the device go back to sleep without acting.

Sense packet: u16le voltage (mV), u16le temperature (0.01 K), u16le pressure (mbar abs),
u16le CRC of the first 6 bytes. Example (3456 mV, 298.15 K, 1013 mbar):
`80 0D 77 74 F5 03 8D 30`.

### Framing and integrity

Page packet (516 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x000 | u16le | page number in this transfer, starting at 0 |
| 0x002 | 512 bytes | page data |
| 0x202 | u16le | CRC of the 512 data bytes only |

After each page the device sends a prompt `A5`. The host answers:

- `A5` to accept. The next page follows within 100 ms.
- `00` to reject. The same page (same number) is resent within 100 ms. The vendor allows
  any number of rejects. libdivecomputer allows 2; another implementation allows 8.
- nothing, to abort the transfer. The device sleeps.

A host should treat a page number other than the expected one as a protocol error.

All CRCs in this variant: [CRC-16/IBM-3740](../common/checksums.md#crc-16-ibm-3740), with
the same parameters as the Sensus Pro. A data page of 512 × `FF` has CRC `0x6995`.

### Memory layout

READ_DATA sends the DATA segment from the last page backwards: page 0 of the transfer is
the newest page. To rebuild memory in chronological order, a host puts transfer page n at
offset 2080768 − (n+1) × 512.

The first and last dive records in DATA may be cut off: wrapped or half-written data at
either end is normal. Erased flash reads `FF`.

### Dive data format

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u32 | header marker `00 00 00 00` |
| 0x04 | u32le | dive start, device clock seconds |
| 0x08 | u16le | INTERVAL in effect for this dive, s |
| 0x0A | u16le | THRESHOLD in effect, mbar abs |
| 0x0C | u16le | ENDCOUNT in effect |
| 0x0E | u16le | AVERAGING in effect |
| 0x10… | 4 bytes each | samples |
| end | u32 | footer `FF FF FF FF` |

Sample: u16le temperature (0.01 K), then u16le pressure (mbar abs).

Depth conversion is left to the host. Variants in use:

- the reference implementation: depth = (p × 100 Pa − P_surface) / (ρ·g), with
  P_surface = 101325 Pa and ρ = 1025 kg/m³ by default, both configurable;
- another implementation: depth m = (p − 1000) / 100, which assumes 1000 mbar at the
  surface and 100 mbar per metre.

libdivecomputer counts towards dive time only the samples whose pressure is ≥ THRESHOLD.

### Download procedure

Vendor-recommended incremental strategy:

```
send 0xB421
n = 0
loop:
    receive page packet; on CRC error answer 00 and retry
    if page number != n: error
    if n > 0 and page data is all FF: stop (no older data)    # host choice
    prepend page to buffer
    scan buffer backwards for header markers 00 00 00 00
        (if the zero run is longer than 4 bytes, the marker starts at the
         first zero of the run)
        for each header: find the footer FF FF FF FF at or after header+16
            and before the start of the dive found before it
            if dive.timestamp <= last_known_timestamp: stop the transfer
            emit dive (newest first)
    answer A5; n += 1; stop after 4064 pages
```

To stop early, the host simply does not answer the prompt. The device times out and
sleeps.

Fingerprint: the 4-byte timestamp at dive offset 4.

USER segment (optional): the vendor's PC software keeps one file there. Layout before
encryption: u32 `00000000`, u16 file size, 506-byte NUL-terminated file name, then up to
15872 bytes of file data starting at 0x200. The whole 16384-byte segment is encrypted
with RC4. The key is the user's password with a `00` byte appended, so an empty password
still gives a defined key. A decryption is correct when the first four plaintext bytes are
zero. A dive-log downloader need not support this.

Confidence: **verified** (vendor guide), except the all-`FF` stop rule and the retry
counts, which are **likely**.

---

### Date and time (all variants)

Dive timestamps count seconds on the device clock, which starts at zero when the device
is assembled or powered up and is never set. Calendar time:

```
dive_start_host = host_time_at_handshake − (device_clock_at_handshake − dive_timestamp)
```

The result is in the host's local time; no time zone is stored. The host clock must be
correct at download time. An already stored dive keeps its calculated time.

Confidence: **verified**.

## Model-specific variations

Summarised in the table at the top. The Pro and Ultra share the CRC. The original Sensus
uses an additive checksum and ASCII literals around the dump.

## Open questions

1. Sensus wake byte: the vendor guide says `00` and libdivecomputer sends `0A`. Does the
   device accept any byte?
2. Sensus Pro: libdivecomputer asserts BREAK before reading and clears it after the
   whole 12-byte packet. The vendor says to clear BREAK as soon as data starts to arrive.
   It is not known whether the late clear matters.
3. Sensus Ultra handshake retries and reject limits differ between implementations. The
   vendor sets no limit on rejects.
4. Sensus Ultra: one implementation is described as sending the instruction LSB before
   the first prompt. The vendor guide requires waiting for a prompt before each byte.
   Follow the vendor guide.

## References

- ReefNet, *SENSUS Developer's Guide* (2001), https://reefnet.ca/downloads/public/doc/sensus_devguide_original.pdf
- ReefNet, *SENSUS PRO Developer's Guide* (2002), https://reefnet.ca/downloads/public/doc/sensus_devguide_pro.pdf
- ReefNet, *Sensus Ultra Developer's Guide* (2006), https://reefnet.ca/downloads/public/doc/sensus_devguide_ultra.pdf
- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
- Greg Cook, *Catalogue of parametrised CRC algorithms* (RevEng), CRC-16/IBM-3740.
