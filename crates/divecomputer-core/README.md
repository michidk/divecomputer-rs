# divecomputer-core

Shared building blocks for dive computer protocols, part of
[divecomputer-rs](https://github.com/michidk/divecomputer-rs).

It provides:

- every checksum and CRC used by dive computers, each named after its entry in the
  [checksum catalogue](https://michidk.github.io/divecomputer-rs/spec/common/checksums.html)
- SLIP framing
- the `Error` type shared by transports and protocols
- the transport traits protocols are written against: `ByteStream`, `PacketLink`, `Clock`
- `RawDive`, a downloaded dive with its fingerprint
- the parsed dive model (`dive::Dive`: header, gas mixes, tanks, samples, events) and the
  unit newtypes in `units` (metres, °C, bar, gas percent)

| Feature | Effect |
| --- | --- |
| `serde` | `serde::Serialize` for the dive model; times as milliseconds, `start` as an ISO 8601 local date-time |

The crate is `no_std`, has no platform dependencies and runs natively, on embedded
targets and in WebAssembly.

```rust
use divecomputer_core::checksum::{CRC_16_XMODEM, Xor8};

assert_eq!(CRC_16_XMODEM.checksum(b"123456789"), 0x31C3);
assert_eq!(Xor8::of(&[0x05, 0x00, 0x16, 0x14]).value(), 0x07);
```

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
