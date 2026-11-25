# McLean Extreme

The McLean Extrême is a technical dive computer (recreational, technical, CCR and gauge
modes). It uses a dual-mode Bluetooth module: classic Bluetooth with the Serial Port
Profile, and BLE with a transparent-UART GATT service. Both carry the same byte stream.
The protocol is request/response. Each message is framed with a start byte, type, 32-bit
length, command byte, payload, a big-endian CRC-16/XMODEM and two zero bytes. The host
reads the device configuration (which includes the dive count), then fetches dives one at
a time by index, newest first.

No public vendor protocol documentation is known. The protocol facts come from
libdivecomputer. The Bluetooth module type comes from a public mailing-list post, and the BLE UUIDs from Microchip's public documentation.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| McLean | Extreme (Extrême) | 0 | Bluetooth classic (SPP), BLE, serial | |

Identification:

- Bluetooth classic / BLE name: exactly `McLean Extreme`.
- BLE service: Microchip (ISSC) Transparent UART (see Transport parameters).

Confidence: **likely** for the name; **uncertain** for the BLE service.

## Transport parameters

**Serial / SPP:** 115200 baud, 8N1, no flow control. For an RFCOMM link the line settings
are ignored. Read timeout 1000 ms. After opening: wait 100 ms, then flush.

**BLE:** Microchip Transparent UART service:

| Role | UUID | Properties |
|------|------|------------|
| service | `49535343-FE7D-4AE5-8FA9-9FAFD205E455` | primary |
| device → host (notify) | `49535343-1E4D-4BD9-BA61-23C647249616` | notify, write, write without response |
| host → device (write) | `49535343-8841-43F4-A8D4-ECBE34729BB3` | write, write without response |

Enable notifications through the CCCD of the notify characteristic. The reference splits
outgoing data into writes of at most 244 bytes and treats incoming notifications (up to
244 bytes each) as a byte stream. Protocol frames are not aligned to BLE packets. See
`../transports/`.

Timing: before **every** transmitted frame the reference waits 300 ms. After a request,
the first byte of the reply (`7E`) can take 6–8 s to arrive. A host must wait at least
about 15 s for it; the reference uses 1 s reads with up to 14 retries. Bytes received
while waiting for `7E` that are not `7E` are discarded.

Confidence: **likely** (serial, timing); **uncertain** (BLE UUIDs and write type).

## Session

There is no handshake or authentication. A session is: any number of request/response
exchanges, then the close command `AA`, which has no reply. The host should always send
`AA` before disconnecting.

## Commands

| Cmd | Name | Request payload | Reply payload | Notes |
|-----|------|-----------------|---------------|-------|
| `91` | serial number | none | variable, ≤ 512 bytes | opaque byte string |
| `A0` | read computer configuration | none | 151 bytes (0x97) | see below |
| `A1` | write computer configuration | (151 bytes, assumed) | – | not used by the reference |
| `A3` | read dive | u16le dive index | 94-byte header frame, then one or more sample frames | see Download procedure |
| `AA` | close session | none | none | |
| `AC` | set clock | u32le seconds since 2000-01-01 00:00:00 | none | |
| `AD` | firmware version | none | 4 bytes, u32le | |

The reply's command byte MUST equal the request's command byte.

Confidence: **likely**.

## Framing and integrity

The same layout is used in both directions:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | start `7E` |
| 1 | 1 | type, always `00` |
| 2 | 4 | payload length, u32le |
| 6 | 1 | command |
| 7 | L | payload |
| 7+L | 2 | CRC, **big-endian** |
| 9+L | 2 | `00 00` |

CRC: [CRC-16/XMODEM](../common/checksums.md#crc-16-xmodem). Width 16, poly 0x1021, init
0x0000, refin/refout false, xorout 0x0000. It covers bytes 1 … 6+L (type, length,
command, payload), but not the start byte or the trailer. Check value `123456789` →
`0x31C3`. A receiver MUST check the CRC and the two trailing zero bytes. The maximum
payload length the host sends is 512 bytes.

Worked examples:

```
H→D  7E 00 00 00 00 00 AD 64 47 00 00          firmware version request
D→H  7E 00 04 00 00 00 AD 01 02 00 00 8F 25 00 00   reply, payload 01 02 00 00
H→D  7E 00 02 00 00 00 A3 00 00 84 2F 00 00    read dive index 0
H→D  7E 00 00 00 00 00 AA 14 A0 00 00          close
```

Confidence: **likely**.

## Memory layout

Not exposed. Dives are addressed by index 0 … N−1, where N is the dive count in the
computer configuration. Index 0 is the oldest dive.

### Computer configuration (`A0` reply, 151 bytes)

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u8 | format version; MUST be 0 |
| 0x19 | u16le | number of stored dives |

The first 45 bytes (0x00–0x2C) appear to be the same settings block that starts every
dive header (gases, setpoints, environment and mode; see below). Other bytes are not
decoded.

## Download procedure

```
fw     = transfer(AD) → 4 bytes
serial = transfer(91) → ≤ 512 bytes
cfg    = transfer(A0) → 151 bytes; require cfg[0] == 0
n      = u16le(cfg[0x19])
for i in 0 .. n−1:
    idx = n − 1 − i                        # newest first
    send A3 with payload u16le(idx)
    hdr = receive frame A3, payload exactly 94 bytes; require hdr[0] == 0
    count = u16le(hdr[0x5C])
    need = count × 4
    samples = empty
    while len(samples) < need:
        frame = receive frame A3 (payload ≤ remaining)
        append its payload
    dive = hdr ++ samples
    if dive[0x2D..0x31] == fingerprint: stop
    emit dive
send AA
```

Fingerprint: the 4 bytes at 0x2D (dive start time).

Clock setting: send `AC` with u32le (local wall-clock time in seconds since
2000-01-01 00:00, counted as if it were UTC). There is no reply.

Confidence: **likely**.

## Dive data format

A dive is the 94-byte header followed by 4-byte samples. Total size = 94 + 4 × count. Any
other size means the dive is corrupt.

### Header

Settings block (0x00–0x2C), as recorded for this dive:

| Offset | Type | Meaning |
|--------|------|---------|
| 0x00 | u8 | format version (0) |
| 0x01 + 2k | u8 | gas k O₂ %, k = 0..7 |
| 0x02 + 2k | u8 | gas k He % |
| 0x13 + j | u8 | CCR setpoint j (j = 0..3), 0.01 bar |
| 0x1E | u16le | surface pressure, mbar |
| 0x23 | u8 | water density: 0 = 1000, 1 = 1020, 2 = 1030 kg/m³; other values invalid |
| 0x2C | u8 | dive mode: 0 = recreational (OC), 1 = technical (OC), 2 = CCR, 3 = gauge |

Dive block (0x2D–0x5D):

| Offset | Type | Meaning |
|--------|------|---------|
| 0x2D | u32le | dive start, seconds since 2000-01-01 00:00 |
| 0x39 | u32le | dive end, same scale; dive time = end − start |
| 0x3D | u8 | minimum temperature, °C |
| 0x3E | u8 | maximum temperature, °C |
| 0x43 | u16le | maximum depth as absolute pressure, mbar |
| 0x45 | u16le | average depth as absolute pressure, mbar |
| 0x5C | u16le | number of samples |

Header depth: m = (p_mbar − surface_mbar) × 100 / (ρ × 9.80665), with ρ from 0x23.

### Samples (4 bytes, fixed 10 s interval)

| Offset | Type | Meaning |
|--------|------|---------|
| +0 | u16le | depth, 0.1 m (already a depth, not a pressure) |
| +2 | u8 | temperature, °C |
| +3 | u8 | flags |

Flags:

| Bits | Meaning |
|------|---------|
| 2–4 | active gas number 0..7 (index into the header gas table) |
| 5–6 | active setpoint number 0..3 (index into 0x13 + j) |
| 7 | 1 = closed-circuit loop active; the setpoint applies |
| 0–1 | not decoded |

Sample k (from 1) is at time 10k s. A change in the gas bits is a gas switch.

Example sample `2C 01 12 84`: depth 300 → 30.0 m, 18 °C; flags 0x84 → CCR on, setpoint
index 0, gas 1.

### Date and time

u32 seconds since 2000-01-01 00:00:00 in the diver's local time. The reference decodes it
as UTC and then drops the zone, which gives the wall-clock time back unchanged. No time
zone is stored.

Confidence: **likely**, except bits 0–1 of the flags and undocumented header bytes.

## Model-specific variations

Only one model. The format version bytes (config and dive header offset 0) allow future
layouts. Readers MUST reject versions other than 0.

## Open questions

1. BLE write type (with or without response) and whether the device needs MTU ≥ 247 for
   244-byte writes.
2. The payload format of `A1` (write configuration).
3. The content of the serial-number reply (ASCII?). The reference only hashes it.
4. The meaning of undocumented header bytes and of flags bits 0–1.
5. Whether a wired serial cable exists, or whether "serial" means SPP only.

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
- Microchip Developer Help, *Transparent UART Service for BM70/RN4870*, https://developerhelp.microchip.com/xwiki/bin/view/applications/ble/android-development-for-bm70rn4870/transparent-uart-service-for-bm70rn4870/
- Subsurface mailing list, "McLean Extreme Integration", https://groups.google.com/g/subsurface-divelog/c/WmfbBjWVZ78 (states the dual-mode SPP + Microchip transparent UART module).
- McLean Extreme product site, https://mcleanextreme.com/
