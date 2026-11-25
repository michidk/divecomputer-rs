# Genz / XDeep Black

The Black dive computer, sold under the Genz and XDeep brands in EANx, TMX and BT
variants, appears on USB as a mass-storage volume. Each dive is a directory holding two
small binary files: a fixed-layout summary and a stream of tagged 4-byte profile records.
There is no command protocol. Only one source describes the format, and it covers only the
fields a single host reads, so most of this document is **likely** or **uncertain**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Genz | Black EANx | – | USB mass storage | |
| Genz | Black TMX | – | USB mass storage | |
| XDeep | Black EANx | – | USB mass storage | |
| XDeep | Black TMX | – | USB mass storage | |
| XDeep | Black BT | – | USB mass storage | Bluetooth link not described |

No model code is known. The files give no way to tell the variants apart.

USB identification: unknown. A host that supports this device also accepts the USB IDs
`040E:0568`, `2544:0004` and `A600:E211` in its generic mass-storage driver. It does not
say which belongs to which vendor, so none of them is confirmed for the Black.
**uncertain**

## Transport parameters

USB mass storage (Bulk-Only Transport, SCSI, FAT file system). See
[../transports/](../transports/). Nothing device-specific is known. **likely**

## Session

None. The host mounts or reads the volume.

## Commands

None.

## Framing and integrity

No checksum in either file is known. **uncertain**

## Memory layout

```
LBDATA/
  <dive directory>/        one per dive, name format unknown
    SUMMARY.BIN            dive header
    LOGBOOK.BIN            dive profile
  <dive directory>/
    ...
```

Names are matched case-insensitively. **likely**

## Download procedure

```
for each sub-directory D of LBDATA:
    read D/SUMMARY.BIN and D/LOGBOOK.BIN
    if SUMMARY.BIN is missing: skip D
    start = u32le at SUMMARY.BIN offset 0
    if start <= fingerprint start: skip D        (incremental download)
    decode summary and profile
```

The fingerprint is the 4-byte start time at SUMMARY.BIN offset 0. The order of directories
is not known to be chronological, so the host MUST check every directory instead of
stopping at the first known dive. **likely**

## Dive data format

### Header (SUMMARY.BIN)

All integers little-endian.

| Offset | Type | Raw unit | Conversion | Meaning |
|--------|------|----------|------------|---------|
| 0x00 | u32le | s | See Date and time | Dive start |
| 0x04 | u32le | s | – | Surface interval before the dive |
| 0x08 | u16le | 1/10 m | v / 10 | Maximum depth |
| 0x0A–0x0F | – | – | – | Unknown |
| 0x10 | u16le | s | – | Dive duration |
| 0x12 | u16le | 1/10 °C, offset 1000 | (v − 1000) / 10 | Minimum temperature |
| 0x14–0x17 | – | – | – | Unknown |
| 0x18 | u8 | % | v / 100 | O₂ fraction of the gas (one gas only) |

The total file size and the remaining fields are unknown. Helium (TMX) is not decoded.
**likely** for the listed offsets; the rest **uncertain**.

### Samples (LOGBOOK.BIN)

The file is a sequence of 4-byte records:

| Offset | Type | Meaning |
|--------|------|---------|
| 0 | u16le | Tick: time index of the record |
| 2 | u16le | Tagged value: bits 0–3 = channel, bits 4–15 = 12-bit value `x` |

| Channel | Meaning | Conversion |
|---------|---------|------------|
| 0 | Depth | `x / 10` m |
| 1 | Temperature | `(x − 1000) / 10` °C |
| 2 | Compass heading | `x` degrees; 4095 = no heading |
| 3–15 | Unknown | – |

Records sharing the same tick belong to one sample. A host builds a sample from each tick
that has a depth record, carrying the last known temperature forward.

The tick is believed to be one second:
- The one host that reads this format takes the depth only at ticks 6, 11, 16, … (tick > 1
  and (tick − 1) mod 5 = 0).
- It then reports the profile with a fixed 5 s interval.

Whether the device logs every second, or only every 5 s at those ticks, is not known.

A known decoder pitfall: dividing the raw 16-bit value by 16 as a floating-point number
leaves the channel nibble in the fraction. Shift right by 4 bits instead (or mask with
`0xFFF0` first).

Confidence: **likely** for the record layout and channels 0–2; **uncertain** for the tick
unit and interval.

Worked example (LOGBOOK.BIN fragment, tick 6 at 12.3 m, 18.5 °C, heading 270°):

```
06 00 B0 07    tick 6, value 0x07B0: channel 0, x = 0x07B = 123 → 12.3 m
06 00 11 4A    tick 6, value 0x4A11: channel 1, x = 0x4A1 = 1185 → 18.5 °C
06 00 E2 10    tick 6, value 0x10E2: channel 2, x = 0x10E = 270°
```

### Date and time

- The start time at SUMMARY.BIN offset 0 counts seconds since 1970-01-01 00:00. It is the
  dive computer's local wall-clock time, not UTC: decode it as if it were UTC and do not
  apply a time-zone conversion.
- No time-zone information is stored.
- One host truncates the start to whole minutes when comparing dives.

**likely**

## Model-specific variations

None known. The EANx, TMX and BT variants are read the same way. TMX trimix data and the BT
variant's wireless link are not described.

## Open questions

1. USB VID:PID of each variant, and whether the device needs anything beyond a standard
   mass-storage mount.
2. Dive directory naming scheme and ordering.
3. Unknown SUMMARY.BIN bytes (0x0A–0x0F, 0x14–0x17, after 0x18): average depth, helium
   fraction, mode, deco model, battery, serial and firmware are all plausible candidates.
4. Tick unit and real sample interval. Whether depth records exist at every tick.
5. Meaning of LOGBOOK.BIN channels 3–15 (ppO₂, deco, tank pressure, events?).
6. Whether either file carries a checksum or a version byte.
7. Bluetooth download on the Black BT.

## References

- No public documentation or open-source implementation of this format is known. This
  document is based on the observed behaviour of one host, together with basic USB mass
  storage and FAT conventions.
