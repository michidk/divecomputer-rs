# divecomputer-protocol

Dive computer download protocols, part of
[divecomputer-rs](https://github.com/michidk/divecomputer-rs). Each protocol family is a
module behind a Cargo feature of the same name, implemented from the
[Dive Computer Protocols](https://michidk.github.io/divecomputer-rs/) specification.

Protocols are `async` and generic over the transport traits in `divecomputer-core`, so
the same code runs over native serial and Bluetooth stacks and over the browser's Web
Serial and Web Bluetooth APIs. The crate is `no_std` and has no runtime dependency.

| Feature | Family | Status |
| --- | --- | --- |
| `shearwater` | Shearwater Petrel family (Petrel, Perdix, NERD, Teric, Peregrine, Tern) | download, log parser (also Predator logs) |

Parsing is separate from downloading and needs no device: `shearwater::parse(raw.data())`
turns a downloaded dive into a `divecomputer_core::dive::Dive`.

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
