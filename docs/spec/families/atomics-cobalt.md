# Atomic Aquatics Cobalt

The Atomic Aquatics Cobalt and Cobalt 2 connect over USB with a vendor-specific interface
(not HID, not CDC, not mass storage). The host sends zero-length vendor control requests
on endpoint 0, and the device answers each one with a block of data on a bulk IN
endpoint. Dives are fetched one at a time, newest first. Each dive arrives as one
self-contained blob protected by an additive 16-bit checksum.

There is no public vendor documentation. Everything below comes from libdivecomputer and
from an independent second implementation, so the best confidence available is
**likely**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Atomic Aquatics | Cobalt | 0 | USB vendor class | memory about 29 × 64 KiB |
| Atomic Aquatics | Cobalt 2 | 2 | USB vendor class | memory about 41 × 64 KiB |

USB identity: VID:PID **0471:0888** for both models (VID 0x0471 is registered to Philips).
The model is reported in the version block (see
Session).

Confidence: **likely**.

## Transport parameters

- Configuration 1, interface 0. The interface has bulk IN and bulk OUT endpoints. The
  reference implementation uses the first bulk endpoint of each direction on the first
  interface. Another implementation uses the last ones. Bulk OUT is never used.
- Control requests: bmRequestType `0x40` (vendor, host-to-device, recipient device),
  wValue 0, wIndex 0, wLength 0, no data stage.
- Bulk reads: request 8192 bytes per transfer. A transfer that returns fewer bytes than
  requested (a short packet) ends the response.
- Timeouts: libdivecomputer uses 2000 ms for both control and bulk transfers. The other
  implementation uses 500 ms for control and 3000 ms for bulk.

See [../transports/usb-raw.md](../transports/usb-raw.md) for general USB handling.

Confidence: **likely**.

## Session

There is no handshake or authentication. libdivecomputer reads the version block once
after opening; the other implementation skips this step.

Version request:

```
control  bmRequestType=40 bRequest=01 wValue=0000 wIndex=0000 wLength=0
bulk IN  <14 bytes> <u16le sum16>
```

Version block (14 bytes):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | 8 × ASCII digit | serial number (decimal digits) |
| 0x08 | u16le | firmware version, major part |
| 0x0A | u16le | firmware version, minor part |
| 0x0C | u16le | model: 0 = Cobalt, 2 = Cobalt 2 |

The response MUST be exactly 16 bytes and its checksum MUST match.

Worked example (serial 12345678, firmware 4/1, Cobalt 2):

```
D→H  31 32 33 34 35 36 37 38 04 00 01 00 02 00 AB 01
```

The firmware split into major/minor is an inference. libdivecomputer combines the two
words as (major << 16) + minor.

Confidence: **likely**.

## Commands

All commands are zero-length vendor control requests with a single bRequest value.

| bRequest | Meaning | Response on bulk IN |
|----------|---------|---------------------|
| `0x01` | read version | 14 bytes + sum16 |
| `0x09` | first (newest) dive | dive blob + sum16, or `FF FF` |
| `0x0A` | next (older) dive | dive blob + sum16, or `FF FF` |
| `0x02` | first dive, simulator log | as `0x09` |
| `0x03` | next dive, simulator log | as `0x0A` |

The `0x02`/`0x03` pair reads dives recorded in the computer's simulation mode. Normal
downloads use `0x09`/`0x0A`.

Confidence: **likely**.

## Framing and integrity

The response to a dive request is read with repeated 8192-byte bulk reads, appended
together, until one read returns fewer than 8192 bytes. That read may return 0 bytes,
and a timeout on it counts as zero bytes. Variant: the other implementation also keeps
reading when a transfer returns exactly 64 bytes (one full-speed max packet). This
suggests that on some host stacks a transfer completes at each 64-byte packet.

- Exactly 2 bytes `FF FF`: no more dives.
- Otherwise the last 2 bytes are a u16le [sum16](../common/checksums.md#sum16) of all
  bytes before them (byte sum, initial value 0, modulo 65536). Remove these 2 bytes
  before parsing.

Example: the bytes `31 32 … 38` (ASCII "12345678") followed by `04 00 01 00 02 00` sum
to `0x01AB`, sent as `AB 01`.

Confidence: **likely**.

## Memory layout

The device does not expose memory addresses. Total log capacity (for progress display
only): Cobalt 1900544 bytes (29 × 65536); Cobalt 2 2686976 bytes (41 × 65536).

## Download procedure

```
version = read_version()                       # optional
first = true
loop:
    send control bRequest = (first ? 0x09 : 0x0A); first = false
    blob = read bulk until a short transfer
    if blob == FF FF: done
    verify and strip sum16
    if len(blob) < 228: error
    if blob[0x14 .. 0x1A] == fingerprint: done  # incremental
    emit blob
```

Fingerprint: the 6 bytes at offset 0x14 (year, month, day, hour, minute).

Confidence: **likely**.

## Dive data format

All multi-byte fields are little-endian. A blob is laid out as follows: header (228
bytes), then N_gas gas/tank records of 18 bytes, then N_sw records of 6 bytes, then
N_samples samples of 16 bytes. A valid blob is at least
228 + 18·N_gas + 6·N_sw + 16·N_samples bytes long.

### Header

| Offset | Type | Raw unit | Meaning |
|--------|------|----------|---------|
| 0x04 | 8 × ASCII | – | serial number, usually shown as `XXXX-XXXX` |
| 0x14 | u16le | year | dive start year |
| 0x16 | u8 | – | month 1..12 |
| 0x17 | u8 | – | day |
| 0x18 | u8 | – | hour |
| 0x19 | u8 | – | minute |
| 0x1A | u8 | s | sample interval |
| 0x1B | u8 | °F | surface temperature; °C = (raw − 32) × 5/9 |
| 0x1E | u16le | 0.01 | firmware ("program") version, e.g. 401 = 4.01 |
| 0x22 | u16le | – | dive number |
| 0x24 | u8 | – | mode: 0 = open circuit trimix, 1 = closed circuit, 2 = open circuit nitrox |
| 0x26 | u16le | mbar | surface (atmospheric) pressure |
| 0x28 | u8 | h | surface interval, hours |
| 0x29 | u8 | min | surface interval, minutes |
| 0x2A | u8 | – | N_gas: number of gas/tank records |
| 0x2B | u8 | – | N_sw: number of 6-byte records (gas switches) |
| 0x50 | u16le | – | N_samples |
| 0x56 | u16le | mbar abs | pressure at maximum depth |
| 0x58 | u16le | min | dive time; seconds = raw × 60 |

Depth from pressure: depth m = (p_mbar − p_surface_mbar) × 100 / (ρ·g), using the
surface pressure from 0x26. The reference implementation uses ρ = 1025 kg/m³,
g = 9.80665 m/s² by default (10051.8 Pa per metre).

### Gas / tank record (18 bytes, at 228 + 18·i)

| Offset | Type | Meaning |
|--------|------|---------|
| +0 | u8 | mix identifier, referenced by samples |
| +2 | u8 | tank size type (see below) |
| +4 | u8 | O₂ %, fraction = raw/100 |
| +5 | u8 | He %, fraction = raw/100 |
| +6 | u16le | start pressure, psi; bar = raw × 0.0689476 |
| +8 | u16le | tank size value |
| +10 | u16le | working pressure, psi |
| +12 | u16le | 1 = this tank carries the pressure transmitter whose readings appear in the samples |
| +14 | u16le | end pressure, psi |

Tank size type, per libdivecomputer:

- 1 = cubic feet at a working pressure in psi; 2 = cubic feet at a working pressure in
  bar. In both cases the value at +8 is in ft³ of free gas and the working pressure at
  +10 is read as psi. Water volume L = ft³ × 28.3168 / (wp_psi / 14.696).
- 3 = water volume in 0.1 L. The working pressure is not used.

The other implementation reads the record differently: for type 3 with a working pressure
of 0, size = value/10 L; otherwise size = value × 410.706 / wp_psi. That divisor uses
bar (14.5038 psi) where libdivecomputer uses atm (14.696 psi). Recorded as an open
question.

### Switch records (6 bytes each)

Present after the gas records. Their contents are not decoded by either source. Gas
switches are recovered from the samples instead.

### Samples (16 bytes each)

| Offset | Type | Meaning |
|--------|------|---------|
| +0 | u16le | ambient pressure, mbar abs; depth as above |
| +2 | u16le | tank pressure, psi (the tank marked at gas record +12) |
| +4 | u8 | active mix identifier (matches gas record +0); a change means a gas switch |
| +5 | u8 | no-deco limit, minutes |
| +8 | u8 | water temperature, °F |
| +11 | u8 | warning flags |
| +6, +7, +9, +10, +12..+15 | – | not decoded |

Warning flags at +11:

| Bit | Meaning |
|-----|---------|
| 0 | ascent rate too fast |
| 1 | in decompression |
| 2 | ceiling violated |
| 3 | ppO₂ out of range |

Deco state: a non-zero NDL means not in deco. NDL = 0 with flag bit 1 set means a deco
stop is required. NDL = 0 without bit 1 leaves the previous state in place.

Time of sample k (counting from 1) = k × interval.

### Date and time

Local wall-clock time as broken-down fields (offsets 0x14–0x19). Seconds are not stored.
There is no time zone.

Confidence: header offsets 0x14–0x2B, 0x50–0x58, gas O₂/He/pressures and sample fields
+0/+2/+4/+5/+8/+11: **likely** (two implementations agree). Offsets 0x04, 0x1E, 0x22,
0x28/0x29: **likely** (one source each). Tank volume encoding: **uncertain**.

## Model-specific variations

The Cobalt and the Cobalt 2 use the same protocol and blob format. They differ only in
memory size and in the model word of the version block.

## Open questions

1. Tank volume: which type codes mean imperial vs metric, and is the working pressure
   divided by 1 atm or 1 bar?
2. The meaning of the 6-byte records counted at 0x2B, and of the undecoded sample bytes.
3. Do some host USB stacks complete bulk reads at 64 bytes? If so, a reader must not
   treat a 64-byte read as the end of a response.
4. The firmware fields in the version block (0x08/0x0A) versus the program version at
   dive offset 0x1E.

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
- USB 2.0 specification, chapter 9 (standard and vendor control requests), https://www.usb.org/
