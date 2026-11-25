<h1 align="center">divecomputer-rs</h1>

<p align="center">
  An open specification of dive computer download protocols, and a Rust library that
  implements it natively and in the browser.
</p>

<p align="center">
  <a href="docs/README.md">Specification</a> ·
  <a href="#supported-dive-computers">Supported dive computers</a> ·
  <a href="#rust-library">Rust library</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

<p align="center">
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0"></a>
</p>

Dive computers store your dives in proprietary formats and hand them over through
vendor-specific protocols: serial cables, infrared, USB HID, Bluetooth classic and
Bluetooth Low Energy. This project documents those protocols openly and implements them as
a permissively licensed Rust library, so that any dive log, website or app can download
dives without depending on vendor software or copyleft code.

## Specification

The [Dive Computer Protocols](docs/README.md) specification covers about 40 protocol families.
For each family it describes:

- the devices, model codes, USB IDs and Bluetooth names that identify them
- link settings: baud rates and line control, HID report sizes, BLE services and
  characteristics
- the session handshake and every command, with worked example frames
- framing, escaping and checksums
- memory layout and the procedure for an incremental download
- the dive header and sample formats, with units and scaling

Shared material is described once: [transports](docs/spec/transports/README.md),
[checksums](docs/spec/common/checksums.md), [date and time
encodings](docs/spec/common/datetime.md) and the [common dive data
model](docs/spec/common/data-model.md). Every section states how well it is confirmed,
from *verified* against vendor documentation or captured device traffic to *uncertain*;
see the [conventions](docs/spec/conventions.md).

The specification is written from scratch under the same licence as the code. Facts may come
from any public source; no code or text is copied from other dive-log software.

## Supported dive computers

| Manufacturer | Devices | Transports | Specification |
| --- | --- | --- | --- |
| Suunto | Solution, Eon, Vyper, Cobra, Gekko, Zoop, D-series, HelO2, EON Steel, EON Core, D5 | serial, USB HID, BLE | [Solution/Eon](docs/spec/families/suunto-solution-eon.md), [Vyper](docs/spec/families/suunto-vyper.md), [D9](docs/spec/families/suunto-d9.md), [EON Steel](docs/spec/families/suunto-eonsteel.md) |
| Scubapro / Uwatec | Aladin, Smart, Galileo, Meridian, Mantis, Chromis, G2, G3, A1, A2, Luna, Aladin Square | IrDA, serial, USB HID, BLE | [Aladin/Memomouse](docs/spec/families/uwatec-aladin-memomouse.md), [Smart](docs/spec/families/uwatec-smart.md) |
| Mares | Nemo, Puck, M1/M2, Icon HD, Puck Pro, Matrix, Smart, Quad, Genius, Sirius, Puck 4 | serial, USB, BLE | [Nemo/Puck/Darwin](docs/spec/families/mares-nemo-puck-darwin.md), [Icon HD](docs/spec/families/mares-iconhd.md), [Genius](docs/spec/families/mares-genius.md) |
| Pelagic brands: Oceanic, Aqualung, Sherwood, Aeris, Apeks, Hollis, Genesis, Tusa, Subgear | VT Pro, Veo, Atom, Geo, Pro Plus, i-series, i330R, DSX and many more | serial, BLE | [Overview](docs/spec/families/pelagic-common.md), [VT Pro](docs/spec/families/pelagic-vtpro.md), [Veo 250](docs/spec/families/pelagic-veo250.md), [Atom 2](docs/spec/families/pelagic-atom2.md), [i330R](docs/spec/families/pelagic-i330r.md) |
| Shearwater | Predator, Petrel, Perdix, Nerd, Teric, Peregrine | Bluetooth classic, BLE | [Protocol](docs/spec/families/shearwater.md), [log format](docs/spec/families/shearwater-log-format.md) |
| Heinrichs Weikamp | OSTC, OSTC 2, 3, 4, 5, Plus, Sport, cR, Frog | serial, Bluetooth classic, BLE | [Legacy](docs/spec/families/hw-ostc-legacy.md), [hwOS](docs/spec/families/hw-ostc3.md), [log format](docs/spec/families/hw-ostc-log-format.md) |
| Cressi | Edy, Leonardo, Giotto, Newton, Drake, Goa, Cartesio, Donatello, Michelangelo, Neon, Nepto | serial, BLE | [Edy](docs/spec/families/cressi-edy.md), [Leonardo](docs/spec/families/cressi-leonardo.md), [Goa](docs/spec/families/cressi-goa.md) |
| Garmin | Descent, fēnix | USB storage, MTP, BLE | [Descent](docs/spec/families/garmin-descent.md) |
| Ratio, DiveSystem, Seac | iDive, iX3M, Guru, Jack | serial, Bluetooth classic, BLE | [iDive](docs/spec/families/divesystem-idive.md) |

Further families: [Atomic Aquatics Cobalt](docs/spec/families/atomics-cobalt.md),
[Citizen Aqualand](docs/spec/families/citizen-aqualand.md),
[Cochran](docs/spec/families/cochran-commander.md),
[Deep Six and Crest](docs/spec/families/deepsix-excursion.md),
[Deepblu](docs/spec/families/deepblu-cosmiq.md),
[Dive Rite NiTek Q](docs/spec/families/diverite-nitekq.md),
[Divesoft](docs/spec/families/divesoft-freedom.md),
[Halcyon Symbios](docs/spec/families/halcyon-symbios.md),
[Liquivision](docs/spec/families/liquivision-lynx.md),
[McLean Extreme](docs/spec/families/mclean-extreme.md),
[Oceans S1](docs/spec/families/oceans-s1.md),
[ReefNet Sensus](docs/spec/families/reefnet-sensus.md),
[Seac Screen](docs/spec/families/seac-screen.md),
[Sporasub SP2](docs/spec/families/sporasub-sp2.md),
[Tecdiving](docs/spec/families/tecdiving-divecomputereu.md),
[Tusa DC Solar Link and Scubapro Z1](docs/spec/families/tusa-dc-solar-link.md),
[XDeep Black](docs/spec/families/xdeep-black.md) and
[Zeagle N2iTiON3](docs/spec/families/zeagle-n2ition3.md).
The [family index](docs/spec/families/README.md) lists every document with its confidence
summary.

## Rust library

The crates implement the specification. They are not on crates.io yet; use them from a
checkout of this repository.

| Crate | What it does |
| --- | --- |
| [`divecomputer-core`](crates/divecomputer-core) | Checksums and CRCs from the [checksum catalogue](docs/spec/common/checksums.md), SLIP framing, the transport traits protocols are written against, and the parsed dive model. `no_std`; runs natively, on embedded targets and in WebAssembly. |
| [`divecomputer-protocol`](crates/divecomputer-protocol) | Downloads dives from the Shearwater Petrel family and parses Shearwater logs, Predator logs included. |
| [`divecomputer-transport`](crates/divecomputer-transport) | Web Bluetooth backend for the transport traits. |
| [`divecomputer-export`](crates/divecomputer-export) | Writes parsed dives as [DiveJSON](https://divejson.org). |
| [`divecomputer-wasm`](crates/divecomputer-wasm) | JavaScript bindings: Shearwater download over Web Bluetooth, parsing, DiveJSON and UDDF export, and a browser test page. |

```rust
use divecomputer_core::checksum::{CRC_16_XMODEM, Xor8};

assert_eq!(CRC_16_XMODEM.checksum(b"123456789"), 0x31C3);
assert_eq!(Xor8::of(&[0x05, 0x00, 0x16, 0x14]).value(), 0x07);
```

`divecomputer-export` and `divecomputer-wasm` build against the
[divejson-rs](https://github.com/michidk/divejson-rs) and
[uddf-rs](https://github.com/michidk/uddf-rs) repositories at pinned Git revisions.
Cargo fetches them automatically; no sibling checkouts are needed.
[ARCHITECTURE.md](ARCHITECTURE.md) describes how the crates fit together and how one
protocol implementation targets both native platforms and the browser.

## Contributing

Recorded downloads from real devices, memory dumps and corrections to the specification
are the most valuable contributions. [CONTRIBUTING.md](CONTRIBUTING.md) explains how to
share them and the clean-room rule that keeps the project permissively licensed.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed
as above, without any additional terms or conditions.
