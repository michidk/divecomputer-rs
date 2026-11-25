# Heinrichs Weikamp OSTC (original, Mk2, 2N, 2C) and Frog

This document covers two older Heinrichs Weikamp protocols:

- **OSTC dump protocol** (OSTC, OSTC Mk.2, OSTC 2N, OSTC 2C; PIC18 firmware). This is serial
  over USB only. One command byte returns a fixed-size memory image: a 6-byte preamble, a
  256-byte block of internal EEPROM, battery and firmware bytes, then the complete external
  EEPROM profile ring (32 or 64 KiB). Dives are found in the ring by start and end markers.
  There is no per-dive request, no echo on the download command and no checksum.
- **Frog** (serial over USB and Bluetooth classic). It uses an early variant of the hwOS
  command protocol in [hw-ostc3.md](hw-ostc3.md), with a few deviations listed in
  [Frog](#frog).

The dive record format (header versions `20`, `21`, `22`, samples, divisors, events) is in
[hw-ostc-log-format.md](hw-ostc-log-format.md).

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Heinrichs Weikamp | OSTC | 0 | USB serial | serial number ≤ 300 |
| Heinrichs Weikamp | OSTC Mk.2 | 1 | USB serial | serial 301-2048 |
| Heinrichs Weikamp | OSTC 2N | 2 | USB serial | serial 2049-7000 |
| Heinrichs Weikamp | OSTC 2C | 3 | USB serial | serial > 7000 |
| Heinrichs Weikamp | Frog | 0 (own family) | USB serial, Bluetooth classic | name prefix `FROG` |

The dump protocol has no model query. The model codes above are derived from the serial
number (dump offset 6, `u16le`) using the thresholds shown. They are a host-side convention,
not a device value.

Identification:

- USB: a USB-serial bridge enumerating as a virtual COM port. No vendor-specific VID:PID is
  documented. The later hwOS models use an FTDI FT232.
- Frog Bluetooth classic: SPP, device name starting with `FROG`.

Confidence: **likely** (one open-source implementation for the model thresholds and the Frog
name prefix). **uncertain** for the USB bridge chip on the PIC18 OSTCs.

## Transport parameters

| Parameter | OSTC/Mk2/2N/2C | Frog |
|-----------|----------------|------|
| Baud rate | 115200 | 115200 |
| Framing | 8N1, no flow control | 8N1, no flow control |
| Host read timeout | 4000 ms | 3000 ms |
| On open | wait 100 ms, purge both directions | wait 300 ms, purge |

There is no wake-up sequence. The OSTC must be in surface mode (not diving, not asleep, not in
a menu), where its UART receive interrupt is active.

Confidence: **likely** (one open-source implementation plus earlier observation notes that
agree on 115200).

## Session

### OSTC / Mk2 / 2N / 2C

There is no handshake and no session state. Each command is one ASCII byte. The firmware
handles the byte in its receive interrupt and performs the action at its next surface-loop
pass. Some commands echo the byte, others do not.

**Do not probe these devices with hwOS commands.** The command letters overlap with the hwOS
command set, but the meanings differ dangerously. For example, `63` enters simulator mode,
`66` resets battery statistics and `69` writes EEPROM bank 1. A host that has to auto-detect
SHOULD send `BB` first. hwOS and Frog devices answer `BB 4D`. The PIC18 OSTC ignores `BB`
(it is not one of its command bytes).

## Commands

### OSTC / Mk2 / 2N / 2C

| Command | H→D | Echo | Arguments | D→H | Safe for a downloader |
|---------|-----|------|-----------|-----|-----------------------|
| Dump | `61` (`a`) | no | none | 33034 or 65802 bytes (below) | **yes**: the download command |
| Set clock | `62` (`b`) | yes | 6 bytes: hour, minute, second, month, day, year−2000 | none | modifies the clock |
| Simulator mode | `63` (`c`) | | | | **no**: starts the dive simulator |
| Write EEPROM bank 0 | `64` (`d`) | yes, and each data byte is echoed | 252 bytes (bank offsets 4-255) | | **no** |
| Firmware + MD2 hash | `65` (`e`) | no | none | 18 bytes: firmware major, minor, 16-byte MD2 hash | yes |
| Reset battery statistics | `66` (`f`) | | | | **no** |
| Read EEPROM bank 0 | `67` (`g`) | no | none | 256 bytes | yes |
| Reset deco data | `68` (`h`) | yes | none | | **no**: clears tissue saturation, CNS, no-fly time |
| Write EEPROM bank 1 | `69` (`i`) | yes | as `64` | | **no** |
| Read EEPROM bank 1 | `6A` (`j`) | no | none | 256 bytes | yes |
| Store tissue data | `6B` (`k`) | yes | none | | modifies state |
| Screen dump | `6C` (`l`) | yes | none | run-length image (below) | yes |
| Read EEPROM bank 2 | `6D` (`m`) | no | none | 256 bytes | yes |
| Write EEPROM bank 2 | `6E` (`n`) | yes | as `64` | | **no** |
| Bootloader | `C1` | | | bootloader answers `57 4B` | **no**: firmware update entry |

The bootloader request `C1` is answered by the bootloader with the PIC type byte `57` and
`4B` (`K`). It is tried at 19200 and at 115200 baud. Firmware is then written in 64-byte
blocks. A downloader MUST NOT send `C1`.

Screen dump encoding (optional feature): 320 × 240 pixels, sent column by column (y runs
fastest). Each run starts with a byte *b*:

- *b* bit 7 = 0: black run of (*b* & 0x7F) + 1 pixels;
- *b* bits 7-6 = `11`: white run of (*b* & 0x3F) + 1 pixels;
- *b* bits 7-6 = `10`: two more bytes follow with an RGB565 colour (big-endian), and the run
  is (*b* & 0x3F) + 1 pixels.

Confidence: **verified** for the command letters, their actions, the dump layout up to the
ring, the MD2 reply and the bank reads (vendor Mk2 firmware source). **likely** for echo
behaviour of `b`/`d`/`i`/`n`/`l`, the `64` argument length, the screen encoding and the
bootloader exchange (one open-source implementation).

## Framing and integrity

There is no checksum. Checks available to the host:

- dump bytes 0-5 MUST be `AA AA AA AA AA 55`;
- the total length is fixed by the firmware version;
- each dive record starts with `FA FA` and its profile ends with `FD FD`. The header has its
  own end marker `FB FB` (version `21`: header bytes 55-56).

### Worked example (dump prefix)

```
H→D  61
D→H  AA AA AA AA AA 55             preamble (offset 0-5)
     39 30 …                       EEPROM bank 0, 256 bytes (offset 6-261); serial = 0x3039 = 12345 → OSTC 2C
     E8 0F                         battery, u16le = 4072 (mV, likely)          (offset 262-263)
     02 5E                         firmware 2.94 (u16be 0x025E > 0x015A → 64 KiB ring)  (offset 264-265)
     <65536 bytes profile ring>    (offset 266-65801)
```

## Memory layout

Dump layout (host buffer offsets):

| Offset | Size | Content |
|--------|------|---------|
| 0 | 6 | preamble `AA AA AA AA AA 55` |
| 6 | 256 | internal EEPROM bank 0. Offset 6-7 = serial number (`u16le`), offset 8-9 = total dive count |
| 262 | 2 | current battery voltage, low byte first |
| 264 | 2 | firmware major, minor. As `u16be`: version × 256 + minor |
| 266 | 32768 or 65536 | external EEPROM (profile ring), rotated (below) |

Ring size: 32768 bytes when firmware (`u16be` at 264) ≤ `0x015A` (1.90), otherwise 65536 bytes.
The total dump is 33034 or 65802 bytes.

The device sends the ring starting at the byte after its current write position and wraps
around. The oldest data therefore comes first and the newest dive ends near the end of the
dump. The firmware writes `FD FD FE` at the end of the newest dive. The `FE` is overwritten
by the next dive and lets the firmware find the newest dive after a reset. While a dive is in
progress the firmware reserves 57 bytes for the header and writes the header only at the end
of the dive. So an interrupted dive may have samples but no `FA FA` header.

Confidence: **verified** for the layout, preamble and rotation start (vendor firmware
source). **likely** for the 1.90 size switch (one implementation plus observation notes:
"firmware > 1.90").

## Download procedure

```
send 61
read 266 bytes; check preamble
size := 266 + (65536 if u16be(buf[264..266]) > 0x015A else 32768)
read the remaining bytes (large reads; the device streams without pauses)
end := size
cursor := size
loop:
    find the last occurrence of FA FA that starts at or after 266 and before cursor;
    if none: stop
    start := position of that FA FA
    search forward from start (not past end) for FD FD; if found:
        record := buf[start .. position of FD FD + 2]
        if record[3..8] == stored fingerprint: stop       (newest-first order)
        emit record
    end := start
    cursor := start
```

Records are therefore produced newest first. A record whose `FD FD` is not found before the
start of the next-newer dive is skipped. This happens for the oldest, partly overwritten dive.
A parser should also check that the version byte at record offset 2 is `20` or `21`.

Fingerprint: 5 bytes at record offset 3 (month, day, year−2000, hour, minute of dive end).

Dive data format: see [hw-ostc-log-format.md](hw-ostc-log-format.md#versions-20-and-21-ostc-mk2-2n-2c).
The record starts with `FA FA`, so the header offsets in that document apply directly.

Date and time: local time of the dive **end**, no seconds, no time zone. See the log-format
document for the start-time computation.

Confidence: **likely** (one open-source implementation, consistent with the vendor firmware's
record markers). An earlier observation scanned forward from offset 266 for `FA FA 21`
instead. It parses every dive and does no fingerprint filtering. Both approaches find the same
records.

## Frog

The Frog speaks the hwOS command protocol ([hw-ostc3.md](hw-ostc3.md)) at 115200 8N1, with
these differences:

| Aspect | Frog |
|--------|------|
| `BB` (download mode) | answered with `4D` only: **no echo** |
| `61` (full headers) | **no echo**: the 65536 header bytes and then `4D` follow directly |
| Other commands | echoed as in hwOS. Ready byte `4D` |
| Service-mode entry | `AA AA AB AC`. Answer `4B 4C`, without echoing the key. Do not use |
| Compact headers `6D` | not supported. Always use `61` |
| Identity `69` | 17 bytes: `u16le` serial, firmware major, minor, 13 bytes custom text |
| Display text `6E` | 15 bytes |
| Custom text `63` | 13 bytes |
| Header layout | version `22`, see the log-format document |
| Profile location | ring `000000`-`1FFFFF`. Header offset 2 = begin pointer (`u24le`), offset 5 = end pointer |
| Fingerprint | header offset 9, 5 bytes |
| Dive counter | header offset 52, `u16le` |
| Firmware in header | offset 32-33 (major, minor) |

Profile length for `66`: the device sends the 256-byte header followed by
(ring distance from begin to end) − 6 bytes, where

```
distance = end − begin                   if end ≥ begin
         = end + 0x200000 − begin        otherwise
(begin == end means an empty profile)
```

Both pointers MUST lie in `000000`-`1FFFFF`. Otherwise the header is corrupt. The newest dive
is the non-empty slot with the highest dive counter. Download walks backwards from it, as for
hwOS. The only validation applied to the received profile is the `FD FD` end marker. The Frog
profile has no small header: the interval and divisors are in the dive header.

Firmware update commands and sizes differ from hwOS (116 KiB image). A downloader MUST NOT
use them.

Confidence: **likely** (one open-source implementation. No vendor Frog documentation was
available).

## Model-specific variations

- Firmware ≤ 1.90 on the PIC18 OSTCs: 32 KiB ring.
- Profile version `20` (early firmware): 47-byte header, no mode byte, all gases enabled.
- Version `21` adds average depth, total dive time, GF/multipliers, mode, last-stop setting,
  gas-enabled mask and on-time.

## Open questions

1. The serial-number thresholds for model detection are a host convention. The vendor does not
   document them, and they may misclassify refurbished or re-serialised units.
2. The USB bridge chip of the PIC18 OSTCs (FTDI or other) is unconfirmed.
3. The Frog header layout and the "distance − 6" profile length have no vendor source.
4. Whether the Frog Bluetooth module supports anything beyond SPP (for example BLE) is unknown.
   Implementations list Bluetooth classic only.
5. The meaning of internal EEPROM bank 0 beyond the serial number and dive count is not
   documented here. The hwOS bank-0 map does not apply to the PIC18 OSTC.

## References

- Heinrichs Weikamp OSTC Mk.2/2N/2C firmware source (GPL-3.0): `interface.asm`, `isr.asm`,
  `divemode.asm`, published by Heinrichs Weikamp (repository
  <https://code.heinrichsweikamp.com/public/mk2>, now access-restricted; public mirror
  <https://github.com/erique/ostc-mk2>). Used as a factual reference only.
- Heinrichs Weikamp, *External Interface for Heinrichs Weikamp Dive-Computers with hwOS*, for
  the shared protocol conventions inherited by the Frog,
  <https://code.heinrichsweikamp.com/public/hwos_code>.
- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied.
