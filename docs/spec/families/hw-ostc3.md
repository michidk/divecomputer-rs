# Heinrichs Weikamp hwOS family (OSTC 2/3/Plus/Sport/cR/Nano/2 TR, OSTC 4/5)

All Heinrichs Weikamp computers from the OSTC 3 onwards share one byte-oriented command
protocol. The 8-bit computers run hwOS (Tech or Sport): OSTC 3, OSTC Plus, OSTC 2 (2015 and
later), OSTC 2 TR, OSTC Sport, OSTC cR and OSTC Nano. The OSTC 4 and OSTC 5 run separate
32-bit firmware that implements the same download commands. The protocol runs unchanged over
USB serial (FTDI), Bluetooth classic (SPP) and Bluetooth Low Energy (a serial-port emulation
profile). The host sends a single command byte. The device echoes it, exchanges a fixed or
self-describing amount of data, and ends with a ready byte. The device keeps a table of 256
dive headers of 256 bytes each (plus a 16-byte compact form) and a profile ring. The host
reads the header table, picks the dives to fetch, and requests each dive by its slot number.
The dive data format is described in [hw-ostc-log-format.md](hw-ostc-log-format.md). The Frog
uses a variant of this command set and is covered in
[hw-ostc-legacy.md](hw-ostc-legacy.md#frog).

## Devices

The device reports a one-byte *hardware descriptor* (command `6A`, or byte 1 of the
command `60` reply) and a *model byte* (byte 4 of the `60` reply). The hardware descriptor is
a bit set, so several products share values. The vendor advises that a host distinguish only
hwOS Sport, hwOS Tech and OSTC 4/5, because all units of one kind parse identically.

| Vendor | Model | Hardware descriptor | Model byte (`60`) | Transports | Notes |
|--------|-------|---------------------|-------------------|------------|-------|
| Heinrichs Weikamp | OSTC cR | `05` or `07` | `00` | USB, BT, BLE | hwOS Tech |
| Heinrichs Weikamp | OSTC 3 | `0A` | `00` | USB | hwOS Tech |
| Heinrichs Weikamp | OSTC 2 (2015+) | `11`, `13` or `1B` | `00` | BT, BLE | hwOS Tech |
| Heinrichs Weikamp | OSTC Sport | `12` or `13` | `00` (one revision of the vendor doc lists `51`) | BT, BLE | hwOS Sport, feature low byte `02` |
| Heinrichs Weikamp | OSTC Plus (OSTC 3+) | `1A` or `13` | `00` | BT, BLE | hwOS Tech |
| Heinrichs Weikamp | OSTC 2 TR | `33` | `00` | BT, BLE | hwOS Tech, tank receiver |
| Heinrichs Weikamp | OSTC Nano | unknown | `00` | USB, BLE | hwOS; see open questions |
| Heinrichs Weikamp | OSTC 4 | `3B` | `43` | BT, BLE | 32-bit firmware |
| Heinrichs Weikamp | OSTC 5 | `3B` | `44` | BT, BLE | 32-bit firmware |

Meaning of the hardware descriptor bits (vendor doc):

| Bit | Meaning |
|-----|---------|
| 0 | rechargeable battery with battery-management chip |
| 1 | ambient light sensor |
| 2 | analog sensor inputs and S8 digital input |
| 3 | digital optical input |
| 4 | BLE module |
| 5 | hwOS: tank pressure receiver. OSTC 4/5 documentation: 32-bit dual core |
| 6, 7 | unused, 0 |

The value `3B` (bits 0, 1, 3, 4, 5) identifies the OSTC 4/5 generation. A host MUST know which
generation it talks to, because the firmware version encoding, the header slot order and
several log-format details differ (see [Model-specific variations](#model-specific-variations)).

Identification per transport:

- USB: FTDI FT232 USB-serial bridge (standard FTDI driver). The vendor documents no custom
  VID:PID.
- Bluetooth classic and BLE advertised names start with `OSTC`. Observed names include
  `OSTC2`, `OSTC #…`, `OSTC+…`, `OSTC3#…`, `OSTCs…` (Sport) and `OSTC4…`. A module that has
  not been named yet may advertise the Telit/Stollmann module default `BM+SR…`.
- BLE: the service UUID identifies the radio module (see below).

Confidence: **verified** for hardware descriptors, bit meanings, the model bytes `43`/`44` and
the Sport feature byte (vendor interface documents and OSTC 4 firmware source). **likely**
for advertised-name prefixes. **uncertain** for the OSTC Nano descriptor.

## Transport parameters

### Serial (USB and Bluetooth classic)

- 115200 baud, 8 data bits, no parity, 1 stop bit, no flow control.
- Bluetooth classic exposes a standard SPP (RFCOMM) serial port. On the OSTC Sport the module
  may request legacy PIN pairing with PIN `0000`.
- Vendor timing: inside a command, each further byte must arrive within 400 ms of the previous
  one (the timeout restarts on every byte). The idle timeout between commands is 120 s. After
  it the device leaves COM mode by itself and sends `FF`. Before the first byte, the device
  waits up to 4 min for `BB` or `AA`. The vendor recommends that hosts flush after every
  command or parameter block and wait for the device reply before sending more.
- Host read timeout used by existing implementations: 2000 to 3000 ms per read, after a
  300 ms settle delay and an input purge on open.
- Writes: existing implementations send up to 1024 bytes per write call, or 64 bytes per call
  for the OSTC 4/5.

### Bluetooth Low Energy

Two radio modules are in use. Both carry the raw serial byte stream, with no extra framing.

**Telit (Stollmann) BlueMod, "Terminal I/O" profile** (OSTC Sport, 2, Plus, 2 TR, cR, early
OSTC 4):

| Role | UUID | Properties |
|------|------|------------|
| Service | `0000FEFB-0000-1000-8000-00805F9B34FB` (16-bit `FEFB`) | |
| UART data RX (host writes data) | `00000001-0000-1000-8000-008025000000` | write without response, 1-20 bytes |
| UART data TX (device sends data) | `00000002-0000-1000-8000-008025000000` | notify, 1-20 bytes |
| UART credits RX (host grants credits) | `00000003-0000-1000-8000-008025000000` | write (with response), `u8` |
| UART credits TX (device grants credits) | `00000004-0000-1000-8000-008025000000` | indicate, `u8` |

The Terminal I/O connection setup is mandatory and ordered:

1. Enable **indications** on credits TX (`00000004…`) by writing `02 00` to its CCCD, and wait
   for the write response. If the module requires a secure link, bonding is triggered here.
2. Enable **notifications** on data TX (`00000002…`) by writing `01 00` to its CCCD.
3. Write an initial credit count (`u8`, 0-255) to credits RX (`00000003…`) and wait for the
   write response. The link is then established.

Flow control is per packet. One credit allows one notification or one write of 1 to 20 bytes.
The device may send as many data notifications as the host has granted. The host has to
replenish credits before they run out. One existing implementation grants 254 (`FE`), counts
received notifications, and grants another 222 when only 32 remain. The host may only write
data while it holds credits that the device granted through credits-TX indications. It
subtracts one per written packet.

The Terminal I/O module advertises manufacturer-specific data: length `08`, then `FF 8F 00 09
B0`, then compatibility version `01`, operation mode (`00` bonding only, `01` functional, `10`
bondable and functional) and a "connection requested" flag.

**u-blox "u-connectXpress" BLE Serial Port Service** (newer OSTC units, including later OSTC
4/5):

| Role | UUID |
|------|------|
| Service | `2456E1B9-26E2-8F83-E744-F34F01E9D701` |
| FIFO (data, both directions: write and notify) | `2456E1B9-26E2-8F83-E744-F34F01E9D703` |
| Credits (both directions: write and indicate/notify) | `2456E1B9-26E2-8F83-E744-F34F01E9D704` |

The same credit scheme is applied: subscribe to the credits characteristic first, then to the
FIFO, then write an initial credit grant.

Packet sizes: host writes are at most 20 bytes per packet. No MTU exchange is required.
Notifications carry at most 20 bytes of payload under the default MTU.

A variant without credits was observed: one implementation subscribes only to data TX and
never grants credits. Whether this works depends on the module configuration (open question).

Confidence: **verified** for the Terminal I/O UUIDs, properties and setup order (Telit/Stollmann
*Terminal I/O Profile Client Implementation Guide*, distributed by Heinrichs Weikamp) and for
the use of Terminal I/O and u-blox SPS (vendor interface document 2025). **likely** for the
u-blox UUIDs and the credit-refill strategy (one open-source implementation). **likely** for
serial line parameters other than the vendor-stated 115200 8N1.

## Session

### Ready byte, echo and unsupported commands

- In download mode the ready byte is `4D` (`M`). In service mode it is `4C` (`L`).
- The device echoes every command byte it understands. It then performs the command and
  finally sends the ready byte. `FF` (exit) gets the echo but no ready byte.
- A device that does not implement a command answers with the ready byte only (no echo). A
  host SHOULD treat "ready byte received where the echo was expected" as "command
  unsupported". On the OSTC 4/5 the unknown-command path sends the ready byte, which confirms
  this.
- After any command except exit, the host MUST read the trailing ready byte before it sends
  the next command.

### Entering download mode

```
H→D  BB
D→H  BB 4D
```

The device shows "Download mode enabled".

### Entering service mode (do not use for downloading)

Service mode enables flash and firmware commands. A downloader has no reason to enter it,
because every download command also works in download mode.

```
H→D  AA            (then wait)
D→H  4B
H→D  AB  D→H AB    (wait for each echo before sending the next key byte)
H→D  CD  D→H CD
H→D  EF  D→H EF
D→H  4C
```

On a wrong key the device returns to waiting for `BB`/`AA`. The OSTC 4/5 reads all three key
bytes and then answers `4B AB CD EF 4C` in one burst.

### Identification

1. `60`: hardware and feature descriptor (5 bytes). If unsupported (old firmware), fall back to
   `6A` (1 byte). Some early firmware supports neither.
2. `69`: identity (64 bytes): serial number and firmware version.

### Shutdown

```
H→D  FF
D→H  FF
```

The device shows "Exited" for 2 s and powers down the radio. The host should exit before the
120 s idle timeout fires. On the OSTC 4/5 in service mode, any command byte below `60` also
ends the session (this guards against modem strings such as `RING`).

Confidence: **verified** (vendor COMM-mode documentation, vendor interface PDF, OSTC 4
firmware source).

## Commands

Download-mode commands (all also accepted in service mode):

| Command | H→D | Arguments (after echo) | D→H after echo | Notes |
|---------|-----|------------------------|----------------|-------|
| Start / download mode | `BB` | none | `4D` | |
| Hardware + features | `60` | none | 5 bytes, then `4D` | `u16be` hardware (`00 hh`), `u16be` feature, `u8` model |
| Full headers | `61` | none | 65536 bytes (256 × 256), then `4D` | slot 0 first |
| Set clock | `62` | 6 bytes: hour, minute, second, month, day, year−2000 | `4D` | values are range-checked. Modifies the device |
| Custom text | `63` | 60 bytes, NUL-terminated | `4D` | modifies the device |
| Get dive | `66` | 1 byte slot number 0-255 | 256-byte header + profile, then `4D` | see [Download procedure](#download-procedure) |
| Identity | `69` | none | 64 bytes, then `4D` | `u16le` serial, 2 bytes firmware, 60 bytes custom text |
| Hardware descriptor | `6A` | none | 1 byte, then `4D` | |
| Firmware details | `6B` | 1 byte: `FF` main, `FE` RTE, `10` font pack | 4 bytes, then `4D` | OSTC 4/5 only. Unknown id → `FF FF FF FF` |
| BT signal strength | `6C` | none | `4D` | OSTC 4/5 only, shows on device |
| Compact headers | `6D` | none | 4096 bytes (256 × 16), then `4D` | hwOS 1.92+ and OSTC 4/5. Recommended for BLE |
| Display text | `6E` | 16 bytes | `4D` | shown on screen (15 characters on hwOS, 4 × 12 on OSTC 4/5) |
| Set icon | `6F` | variable data | `4D` | OSTC 4/5 only. Modifies the device: do not send |
| Read setting limits | `70` | 1 byte id | 4 or 7 bytes, then `4D` | OSTC 4/5 only |
| Read setting | `72` | 1 byte id | 1-4 bytes (OSTC 4/5: always 4), then `4D` | |
| Write setting | `77` | 1 byte id, then 1-5 bytes (OSTC 4/5: always 5) | `4D` | modifies the device |
| Reset all settings | `78` | none | `4D` | **destructive**: factory defaults |
| Exit | `FF` | none | none | |

Setting ids for `72`/`77` include gases 1-5 (`10`-`14`: O2 %, He %, type, change depth),
diluents 1-5 (`15`-`19`) and setpoints 1-5 (`1A`-`1E`: cbar, change depth). Single-byte
options start at `1F`. A downloader does not need them. On hwOS, the host must pause after
the id byte of `77`, because the device only knows how many value bytes follow after it has
evaluated the id. Existing implementations wait 10 ms.

Commands a downloader MUST NOT send. They are listed only so that an implementation can
reject them:

| Byte | Mode | Effect |
|------|------|--------|
| `20` | service | read raw flash range (3-byte address, 3-byte length, big-endian). Read-only, but needs service mode |
| `22` | service | erase the complete logbook |
| `23` | service | reset battery gauge |
| `30`, `31` | service | write flash (stream; 256-byte page) |
| `40`, `42` | service | erase one / N 4 KiB flash blocks, no confirmation |
| `50` | service | start firmware update (followed by a 5-byte checksum, then reboot) |
| `73`, `74`, `75`, `76` | service (OSTC 4/5) | upload firmware blob (hwOS 4: CPU2 / main firmware / RTE / fonts) |
| `71`, `79`, `80`-`82` | service (OSTC 4/5) | manufacturing data, second serial, Bluetooth name |
| `83`-`89` | service (OSTC 4/5) | copy, overwrite or repair logbook and sample memory |
| `FE` | service (OSTC 4/5) | hardware unit tests |
| `C1` | service | jump to the low-level bootloader (cold start) |
| `6F`, `62`, `63`, `77`, `78` | download | modify settings, clock, text or icon. Only send on explicit user request |

Confidence: **verified** (vendor interface PDFs 2019 and 2025, COMM-mode documentation 2020,
OSTC 4 firmware source).

## Framing and integrity

There is no framing, escaping, compression, encryption or checksum on the download path.
Integrity rests on:

- the command echo and the trailing ready byte,
- exact response lengths (fixed per command, or computed from the header for `66`),
- the header start and end markers `FA FA` … `FB FB` (bytes 0-1 and 254-255 of every
  256-byte header),
- the profile end marker `FD FD`,
- the profile length repeated in the profile's own first three bytes.

Over BLE the byte stream is split into notifications of up to 20 bytes. Their boundaries carry
no meaning.

### Worked example: identity and one empty dive

```
H→D  BB
D→H  BB 4D
H→D  69
D→H  69 39 30 03 5E <60 bytes custom text> 4D
     → serial = 0x3039 = 12345, firmware (hwOS) = 3.94
H→D  66
D→H  66
H→D  07                           (slot 7)
D→H  FA FA 00 00 00 00 00 00 24 08 00 00 … (256-byte header) … FB FB
     08 00 00 FD FD               (empty profile: length 8, then end marker)
     4D
```

Here the header announces a profile length of 8 at bytes 9-11, and 8 − 3 = 5 profile bytes
follow. This is the form hwOS sends when the profile of a dive has been overwritten or is
invalid. The begin and end pointers (bytes 2-7) are zero.

Confidence: **verified** for markers and the empty-profile form (vendor COMM documentation).
**likely** for the length arithmetic (see below).

## Memory layout

- External flash: 4 MiB (`000000`-`3FFFFF`) on hwOS devices. The profile ring occupies
  `000000`-`1FFFFF` (2 MiB) and wraps. The firmware staging area starts at `3E0000`. Raw
  flash access exists only in service mode and is not needed for downloading. The OSTC 4/5
  has no raw-read command.
- Logbook: 256 header slots of 256 bytes each. A slot that has never been used reads as all
  `FF`. When the ring is full, the oldest profiles are overwritten ("first in, first out").
  Their headers may survive and are then served with an empty profile.
- Slot order: the order of the 256 slots is not chronological. Sort by the date/time in the
  header, or by the internal dive counter (header offset 80, compact offset 13). On the OSTC
  4/5, slot `i` as sent corresponds to internal entry `255 − i`, counted backwards from the
  newest dive. So the newest dive is the highest-numbered non-empty slot. The same slot number
  is used for `66`.

Confidence: **verified** for the 256 × 256 table, `FF` fill and OSTC 4 slot mapping (vendor doc
and OSTC 4 source). **likely** for the flash map (2 MiB ring, 4 MiB size, firmware area: one
open-source implementation plus a vendor source comment).

## Download procedure

```
open transport; settle 300 ms; purge input
send BB, expect BB 4D
send 60 → if echo: read 5 bytes + 4D
          else (got 4D only): send 6A → if echo: read 1 byte + 4D, else no descriptor
send 69, read 64 bytes + 4D
generation := OSTC4/5 if hardware descriptor == 3B else hwOS
send 6D; if unsupported: send 61 (full headers) instead
for each slot: skip if all bytes are FF; read profile length L, fingerprint, dive counter
determine newest slot:
    hwOS:     slot with the highest dive counter
    OSTC 4/5: last non-empty slot in index order
walk slots from newest backwards (wrapping modulo 256):
    skip empty slots
    stop if the 5-byte fingerprint equals the stored fingerprint
    expected := 256 + L − 3
    (full-header fallback only: if header firmware < 0.93, expected := expected − 3)
    send 66, expect echo, send slot number
    read 256-byte header
    if (hwOS) header bytes 2..7 are all 0 and header L == 8:
         expected := 256 + 5          (profile was invalidated)
    read expected − 256 profile bytes, then 4D
    validate (below), store
send FF, read echo
```

Validation of a received dive, as done by existing implementations:

1. The version byte (header offset 8) and the date, time, max depth and dive time (offsets
   12-21) must equal those of the logbook entry. With compact headers, also the dive counter
   (offset 80 against compact offset 13). With full headers, the whole of bytes 12-255 must
   match.
2. If the profile does not end with `FD FD`, or is shorter than 2 bytes, keep only the header
   (no samples).
3. A profile that is exactly `FD FD` is a valid empty profile.
4. Otherwise the profile's first three bytes (`u24le` length) must equal the header length
   field at 9-11. On the OSTC 4/5 the header field is 3 larger than the profile field.
   Mismatch → keep the header only.

Fingerprint: the 5 bytes at header offset 12 (year−2000, month, day, hour, minute). In the
compact header they are at offset 3.

Progress estimate: the vendor states roughly 3 bytes of profile per second of dive time
(about 21.6 KB for one hour).

Confidence: **likely** (one open-source implementation, consistent with vendor
documentation). The "L − 3" rule and the firmware-0.93 correction are not in vendor
documentation: see open questions.

## Dive data format

See [hw-ostc-log-format.md](hw-ostc-log-format.md): 256-byte header versions `23` and `24`, the
16-byte compact header, the profile "small header" with divisor table, and the sample and
event encoding.

### Date and time

Local time without time zone. Year is 2000-based. With profile version `24` (all OSTC 4/5 dives
and newer hwOS dives) the stored timestamp is the **start** of the dive. With `23` it is the
**end**. Set clock order: hour, minute, second, month, day, year−2000.

## Model-specific variations

| Aspect | hwOS (8-bit) | OSTC 4/5 |
|--------|--------------|----------|
| Firmware in `69` reply, bytes 2-3 | major, minor (`u16be`, e.g. `03 5E` = 3.94) | `u16le` packed: bits 15-11 major, 10-6 minor, 5-1 patch, bit 0 beta |
| Firmware in dive header, offset 48-49 | major, minor | same packed `u16le` |
| Header slot order | storage order | reversed (slot `i` = internal entry `255 − i`) |
| Header length field vs profile length field | equal | header = profile + 3 |
| `60` model byte | `00` | `43` (OSTC 4), `44` (OSTC 5) |
| Write chunk size | up to 1024 | 64 bytes recommended |
| `77` write length | 1-5 value bytes depending on id | always 5 |
| `72` read length | 1-4 bytes depending on id | always 4 |
| Raw flash commands `20`/`30`/`31`/`40`/`42` | yes (service) | no |
| Profile when samples are missing | header with zero pointers, length 8, `08 00 00 FD FD` | firmware synthesises dummy samples from header data |

Example of OSTC 4 firmware packing: version 1.7.5 (not beta) = `(1<<11)|(7<<6)|(5<<1)|0` =
`0x09CA`, sent as `CA 09`.

When the OSTC 4/5 is in its bootloader (service mode), sending `60` has been reported to drop
the Bluetooth link. One implementation therefore sends `6A` first in service mode, and if
that returns `3B` it assumes model `43` without sending `60`.

The OSTC 4/5 internally stores logbook version `30`. Over the wire it sends `24` for entries
of that version. For entries of any other internal version it sends `FF`, with offsets 57-58
set to `FF FF` and offset 244 set to `FF`.

hwOS Tech firmware since 3.09 and hwOS Sport since 10.64 support the 256-byte flash write
`31`. This matters only for firmware update.

Confidence: **verified** for the firmware encoding, slot order, length offset, dummy samples
and model bytes (OSTC 4 firmware source and vendor doc). **likely** for the bootloader `60`
behaviour (one implementation only).

## Open questions

1. Exact meaning of the header profile-length field. The vendor says it counts the profile
   "including final FD FD" (and once names offsets 8-10 instead of 9-11). Implementations
   transfer `L − 3` bytes after the header, and on hwOS the profile's own length field equals
   `L`. Why the length counts 3 bytes more than are sent is unexplained.
2. The firmware-older-than-0.93 correction (another −3) comes from one implementation and has
   no vendor source.
3. OSTC Nano hardware descriptor value is not documented.
4. Hardware descriptor `13` is listed for OSTC 2, Plus and Sport. Only the feature byte (Sport
   `02`) separates Sport from Tech.
5. The 2019 vendor document lists model byte `51` for the OSTC Sport. The 2020 COMM
   documentation says hwOS always sends `00`.
6. The 2025 vendor table says `60` returns "Data (6)", but the byte-by-byte description and the
   firmware source give 5 data bytes.
7. Whether BLE downloads work without the Terminal I/O credit handshake. One implementation
   omits it, while the module vendor declares it mandatory.
8. u-blox SPS characteristic properties are taken from an open-source implementation. The
   u-blox document UBX-16011192 was not consulted.

## References

- Heinrichs Weikamp, *External Interface for Heinrichs Weikamp Dive-Computers with hwOS (OSTC3,
  OSTC cR, OSTC 2 and OSTC Plus)*, last update 2019-04-10, distributed with the hwOS sources
  (`doc/hwos_interface.pdf`), <https://code.heinrichsweikamp.com/public/hwos_code>.
- Heinrichs Weikamp, *External Interface … hwOS and OSTC4/5* (`hwos4_interface_v16.pdf`), last
  update 2025-07-13, in the OSTC 4 sources, <https://code.heinrichsweikamp.com/public/ostc4>.
- Heinrichs Weikamp, *OSTC hwos3 COMM Mode Documentation* (`comm-docu.txt`), 2020-03-16, in
  the hwOS sources.
- Heinrichs Weikamp OSTC 4/5 firmware source (GPL), `tComm.c`, `logbook.c`, `logbook.h`,
  `settings.c`, used as a factual reference only.
- Stollmann (Telit), *Terminal I/O Profile – Client Implementation Guide*, release r02,
  <https://www.heinrichsweikamp.net/downloads/TIO_Implementation_Guide_r02.pdf>.
- u-blox, *u-connectXpress Bluetooth Low Energy Serial Port Service*, UBX-16011192 (cited by
  the vendor, not consulted).
- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied.
- Subsurface (GPL-2.0) BLE transport, used as a factual reference for the u-blox UUIDs and
  credit handling; no code copied.
