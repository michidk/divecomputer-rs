# Dive computer protocol specification

An open, implementation-neutral description of the protocols used to download dive logs
from dive computers, and of the dive data formats they return. It is the basis for the
Rust crates in this repository.

Status: draft. Nothing here is endorsed by any manufacturer.

## Layout

| Path | Contents |
|------|----------|
| [conventions.md](conventions.md) | Notation, terminology, units, confidence levels |
| [TEMPLATE.md](TEMPLATE.md) | Structure every family document follows |
| [common/](common/) | Checksums and CRCs, date/time encodings, the common dive data model |
| [transports/](transports/) | Serial and USB-serial chips, USB HID, Bluetooth classic, Bluetooth Low Energy, IrDA, USB mass storage and MTP |
| [families/](families/README.md) | One document per protocol family (index with confidence summary) |

## Sources and clean-room policy

This specification is written from scratch so that implementations can use permissive
licenses. Contributors follow these rules:

1. Record **facts only**: byte values, command codes, offsets, field sizes, scaling factors,
   UUIDs, USB IDs, timing values, algorithms described in prose or as freshly written
   pseudocode.
2. **Never copy** source code, comments, identifiers, prose or table layout from other
   projects, whatever their license. Describe everything in your own words and structure.
3. Facts may be learned from public documentation, observed device traffic, device dumps,
   and other open-source projects used as references. References are listed per document.
4. Where sources disagree, document every variant and mark it as an open question instead
   of picking one silently.
5. Every section carries a confidence level (see [conventions.md](conventions.md)).

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/michidk/divecomputer-rs/blob/main/LICENSE-APACHE) or
[MIT license](https://github.com/michidk/divecomputer-rs/blob/main/LICENSE-MIT) at your option.
