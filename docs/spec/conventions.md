# Conventions

## Requirement language

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are used as described in RFC 2119,
and only where an implementation that ignores them would fail to talk to a device or would
misread data.

## Terminology

| Term | Meaning |
|------|---------|
| host | The computer, phone or browser downloading data |
| device | The dive computer, or the interface cable/adapter when stated |
| transport | The physical/link layer: serial, USB HID, BLE, IrDA, USB storage, ... |
| family | A group of devices sharing one protocol (one document in `families/`) |
| model code | The numeric or string identifier a device reports for its model |
| fingerprint | A short byte string identifying a dive (usually its timestamp bytes), used to stop an incremental download at the first already-known dive |
| dump | A raw copy of device memory as returned by a full memory read |

## Byte notation

- Hex bytes are written without prefix, space-separated, in transmission order: `1B 2C FF`.
- Single values in prose use `0x` prefix: `0x1B`.
- Integers in data structures: `u8`, `u16le`, `u16be`, `u32le`, `u32be`, `i16le`, ... BCD
  values as `bcd8`. Bit fields are numbered from bit 0 = least significant.
- Offsets are zero-based and given in hex with `0x` unless stated otherwise.
- Direction: `H→D` (host to device), `D→H` (device to host).
- Message exchanges are shown as:

  ```
  H→D  84 00
  D→H  5A <16 bytes> <sum8>
  ```

- `<name:size>` denotes a variable field of the given size in bytes.

## Units

The common data model uses SI units: depth in metres, temperature in degrees Celsius,
pressure in bar, time in seconds, gas fractions as 0..1. Every field description states the
raw unit and the conversion, e.g. `u16le, 1/100 m`.

Depth conversion from pressure, where a device stores pressure: document the device's own
convention (density, atmospheric pressure) and do not substitute a generic constant.

## Checksums

Every checksum or CRC is specified by name and full parameters (width, polynomial, initial
value, reflection, final XOR, covered bytes) and refers to an entry in
[common/checksums.md](common/checksums.md). Each entry there has a test vector.

## Confidence levels

Each section ends with one of:

| Level | Meaning |
|-------|---------|
| **verified** | Confirmed against device traffic or a dump, or by two independent sources that agree |
| **likely** | One reliable source, internally consistent |
| **uncertain** | Inferred, sources disagree, or details are missing |

Open questions are collected at the end of each family document.
