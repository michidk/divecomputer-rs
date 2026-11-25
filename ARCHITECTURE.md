# Architecture

This document describes how the Rust crates are structured and why. It is the reference
for design decisions; `AGENTS.md` turns it into rules for day-to-day work. Change this
document in the same pull request as any change that departs from it.

Status: early implementation. `divecomputer-core` has the checksum catalogue, SLIP
framing, the error type, the `ByteStream`, `PacketLink` and `Clock` traits and `RawDive`.
`divecomputer-core` also has the parsed dive model (`dive::Dive`, unit newtypes in
`units`, `jiff` civil date-times, `serde` behind a feature). `divecomputer-protocol` has
the Shearwater Petrel-family download, tested against a simulated device, and the
Shearwater log parser (block and native formats). `divecomputer-transport` has the Web
Bluetooth `PacketLink` and a `setTimeout` clock. `divecomputer-export` writes the dive
model as DiveJSON through the `divejson` crate (see [Export](#export)), and
`divecomputer-wasm` exposes `downloadShearwater` (raw and parsed dives),
`parseShearwaterDive`, `exportDiveJson`, `exportUddf`, `convertDiveJsonToUddf` and
`convertUddfToDiveJson` to JavaScript. The browser tools in `www/` keep file data on the
user's device. `scripts/build-pages.sh` builds the book and stages both tools under
`tools/` in the same GitHub Pages artifact. The facade, the other export
formats, the CLI and native backends are not written yet. Decisions marked **open** need a
prototype before they are final.

## Goals

1. **One library, every target.** The same protocol code runs natively (Linux, macOS,
   Windows, Android) and in the browser (`wasm32-unknown-unknown`) from day one. No
   protocol code is written twice for different targets.
2. **Library quality.** Small, well-named public APIs; types that make invalid states
   unrepresentable; no panics on device input; documented errors.
3. **No duplication.** Checksums, framing, ring buffers, byte decoding, BLE chunking and
   USB-serial chip handling each exist exactly once.
4. **Permissive licensing.** MIT OR Apache-2.0 for our code; dependencies must not be
   copyleft (see [Dependency policy](#dependency-policy)).
5. **Deterministic tests without hardware.** Every protocol is tested against recorded
   device sessions.

## Layers

```
┌───────────────────────────────────────────────────────────────┐
│ divecomputer            facade: device registry, Downloader   │
├──────────────────────────────┬────────────────────────────────┤
│ divecomputer-protocol        │ divecomputer-export            │
│  one module per family:      │  UDDF, Subsurface XML, CSV,    │
│  download + parse            │  DiveJSON                      │
├──────────────────────────────┴────────────────────────────────┤
│ divecomputer-core   (no_std + alloc)                          │
│  dive model, units, time, checksums, framing codecs,          │
│  byte reader, ring buffer, transport traits, errors           │
├───────────────────────────────────────────────────────────────┤
│ divecomputer-transport  backends selected by feature + target │
│  serial | usb (incl. FTDI/CP210x/PL2303/CDC) | hid | ble |    │
│  files  ·  native and web implementations                     │
└───────────────────────────────────────────────────────────────┘
  divecomputer-wasm   wasm-bindgen API for JavaScript (npm)
  divecomputer-cli    native tool: download, record, replay, export
```

Dependencies only point downwards. `divecomputer-protocol` depends on the transport
**traits** in `divecomputer-core`, never on `divecomputer-transport`, so protocol code
cannot reach a platform API.

### Crates

| Crate | Purpose | `no_std` | Notes |
|-------|---------|----------|-------|
| `divecomputer-core` | Shared vocabulary: dive model, units, time, errors, checksums, framing codecs (SLIP, HDLC, COBS), byte reader, ring-buffer walker, transport traits | yes (+`alloc`) | No platform or runtime dependencies |
| `divecomputer-protocol` | One module per protocol family (`suunto::eonsteel`, `shearwater`, ...), each behind a Cargo feature named after the family | yes (+`alloc`) | Generic over core transport traits |
| `divecomputer-transport` | Concrete transports. Features `serial`, `usb`, `hid`, `ble`, `files`; backend chosen by target | no | The only crate that touches OS or browser APIs |
| `divecomputer-export` | Writers for exchange formats | yes (+`alloc`) | Consumes only the core dive model |
| `divecomputer` | Facade: re-exports, device descriptor registry, `Downloader` that connects a descriptor, a transport and a protocol | no | What most users depend on |
| `divecomputer-wasm` | `wasm-bindgen` bindings and npm package | no | Thin; no logic of its own |
| `divecomputer-cli` | Command-line tool | no | Also the recorder for test sessions |

Protocol families are modules behind features rather than separate crates. Forty-plus
crates would multiply release and version work without improving isolation, because all
families depend on the same core and nothing else.

## Key design decisions

### Async protocols over minimal transport traits

Browser device APIs (WebUSB, WebHID, Web Bluetooth, Web Serial) are promise-based and
cannot block. Protocols are therefore written as `async fn`s that are generic over small
transport traits. They are runtime-agnostic: no Tokio, no async-std, no executor-specific
types in `core` or `protocol`.

Download protocols are strictly sequential request/response exchanges. A full sans-IO
state machine per family (the quinn-proto/str0m style) would add a hand-written state
enum and event loop to each of ~45 families for no gain here; async functions express the
same sequence directly. Determinism comes from the injected clock and the replay
transport instead (see [Testing](#testing)).

Parsing is separate and synchronous: a download produces raw dive records, and pure
functions turn raw records into the dive model. Parsers are `no_std`, fuzzable, and can
re-parse stored raw data without a device.

```
download (async, needs transport)  →  RawDive { bytes, fingerprint, device info }
parse    (sync, pure)              →  Dive
```

### Transport traits by capability

Transports differ in kind, not just in backend. Protocols state exactly what they need
through trait bounds, so a protocol that needs DTR/RTS cannot be handed a BLE link.

| Trait | Shape | Implemented by |
|-------|-------|----------------|
| `ByteStream` | async read with timeout, write, purge | serial, USB-serial chips, Bluetooth classic, BLE GATT-serial adapter, IrDA |
| `SerialControl` | baud rate, data bits, parity, stop bits, DTR, RTS, break | serial, USB-serial chips |
| `PacketLink` | async send/receive of whole packets with a maximum size | USB HID reports, BLE characteristics, USB bulk endpoints |
| `ControlTransfer` | USB control requests | raw USB (e.g. Atomic Cobalt) |
| `FileSource` | list and read files | USB mass storage and MTP exposed as files, browser file upload |
| `Clock` | `now()` and async `sleep()` | native timer, browser `setTimeout`, virtual clock in tests |

Shared adapters live once in core and turn one shape into another, for example a
`PacketLink` plus BLE chunking rules into a `ByteStream` ("GATT serial"), or SLIP/HDLC
framing over a `ByteStream`.

**Open:** `async fn` in public traits has no `Send` bound. Browser futures are not
`Send`; native multithreaded executors want `Send`. Prototype `trait-variant` (a `Send`
variant generated from one definition) against a single-threaded-only API before fixing
the trait signatures.

### Transport backends

One implementation per transport kind wherever a crate covers native and web:

| Kind | Native backend | Browser backend | Notes |
|------|----------------|-----------------|-------|
| Raw USB, USB-serial chips | `nusb` | `nusb` (WebUSB backend since 0.2.6) | FTDI, CP210x, PL2303 and CDC-ACM drivers written once on top of `nusb`; on desktop browsers Web Serial is preferred because OS drivers claim these chips |
| Serial (OS driver) | **open** — see dependency policy | `web-sys` Web Serial | Also Bluetooth classic SPP on both sides |
| USB HID | **open**: `hidra` (native + WebHID, young) or `async-hid` + `web-sys` | as left | Wrapped behind `PacketLink` so the choice is replaceable |
| BLE | `btleplug` | `web-sys` Web Bluetooth | `rblue` (both) is too young today; revisit |
| Files | `std::fs` | browser `File`/`FileSystemHandle` | Garmin, Divesoft, XDeep |

WebUSB, WebHID, Web Serial and Web Bluetooth bindings in `web-sys` require
`--cfg=web_sys_unstable_apis`. Configuration in a dependency is not inherited, so the
`divecomputer-wasm` crate and every downstream browser build must set it; the README and
`.cargo/config.toml` say so.

### Dive model

- Defined once in `divecomputer-core`, informed by `docs/spec/common/data-model.md`.
- SI units with lightweight newtypes (`Depth` m, `Temperature` °C, `Pressure` bar,
  `GasFraction` percent) so conversions happen once, at the parser boundary. Elapsed times
  are `core::time::Duration`, serialized as milliseconds.
- One `Dive` is one device's recording of a dive, mirroring a DiveJSON recording: header
  values, gas mixes and tanks (referenced by index), samples, and events on the same
  elapsed-time axis. Unknown values are `None`, never zero.
- Time with `jiff`: civil date-time plus an optional UTC offset, because many devices do
  not know their time zone.
- Every dive keeps its raw bytes and fingerprint so it can be re-parsed after parser
  fixes and so incremental downloads stop at known dives.
- `serde` support (`Serialize` only, today) behind the core crate's `serde` feature.

### Export

`divecomputer-export` maps the dive model onto other crates' typed models rather than
writing formats itself. DiveJSON goes through the `divejson` crate, which validates every
document it writes; UDDF is meant to come from `divejson`'s own DiveJSON-to-UDDF writer, so
the dive model is mapped once. One core `Dive` becomes one DiveJSON dive with one
recording:

- Units: depth in centimetres, temperature in hundredths of a °C, tank pressure in tenths
  of a bar, ppO₂ in hundredths of a bar, CNS in tenths of a percent, NDL and TTS in
  seconds, times in milliseconds; rounding is half away from zero.
- `started_at` carries an offset only when the dive has `utc_offset`; otherwise it is a
  local date-time. A dive without a start cannot be exported.
- Identity: `uuid5(DIVE_NAMESPACE, "dive:" + [brand, serial, device dive number, start])`,
  following the DiveJSON converter guide, so re-exporting a dive keeps its uuid. The
  namespace is `uuid5(NAMESPACE_URL, "https://michidk.github.io/divecomputer-rs/ns/divecomputer")`
  and is fixed forever.
- Cylinders: one per gas mix, plus one per tank that names no mix or shares one with an
  earlier tank; gas numbers are written only where a pressure series or gas switch refers
  to them.
- Not mapped (no DiveJSON member, or not recorded as such): setpoint, sensor ppO₂, deco
  stop depth and time, remaining gas time, vendor events, `max_temperature`,
  `water_density`, tank names and transmitter serials, bookmark headings.

`divejson` and, through its `uddf` feature, `uddf` use their published crates.io
releases. No Git or sibling-path dependency is needed.

### Errors

`thiserror`-derived error enums, one per crate, `#[non_exhaustive]`. Device input never
panics: malformed data is an error value naming the family, the field or offset, and
what was expected. Timeouts, checksum failures and protocol violations are distinct
variants because hosts react differently (retry, report, abort).

### Device registry

A static table of descriptors (vendor, model, model code, family, transports, BLE name
patterns, USB IDs) drives discovery and lets the UI list supported devices. It is data,
not code, and is checked against the spec's device tables in tests.

## Testing

| Level | What | Tooling |
|-------|------|---------|
| Unit | Checksums, codecs, byte readers, with the test vectors from `docs/spec` | `cargo test` |
| Property | Codec round trips (SLIP, HDLC, COBS), reader bounds | `proptest` |
| Session replay | Each protocol against recorded device sessions | Replay transport + virtual `Clock` |
| Parser snapshots | Raw dives from captures → dive model | `insta` |
| Fuzzing | Every parser and every framing decoder | `cargo-fuzz` |
| WebAssembly | Core, protocol and export crates build and pass tests on `wasm32-unknown-unknown` | `wasm-bindgen-test` |

Recorded sessions are the project's most valuable asset. The CLI and the web app have a
record mode that writes a session file (timestamped host writes and device reads, line
settings, timing). Session files contain only data a user explicitly shared.

## Dependency policy

- Allowed licences: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, Unlicense.
- Not allowed: GPL, LGPL, AGPL, and any dependency derived from such code (for example
  bindings to libdivecomputer).
- **Open:** `serialport` (the standard native serial crate) is MPL-2.0. MPL is file-level
  copyleft and does not affect our licence, but it is outside the list above. Decide
  between accepting it as a documented exception, or a native serial backend on `nusb`
  plus the OS serial APIs.
- Enforced in CI with `cargo-deny` (licences, advisories, duplicate versions, sources).
- Prefer well-maintained crates that support `no_std` or `wasm32` where relevant: `crc`
  for CRC tables, `cobs` for COBS, `jiff` for time, `thiserror` for errors.

## Prior art consulted

- quinn-proto, str0m and the sans-IO pattern write-up by Firezone: state machines
  separated from I/O. Adopted for parsing; adapted (async + injected clock) for download.
- `ledger-lib`: one byte-exchange trait, one backend per transport behind features, a
  generic wrapper over enabled backends.
- probe-rs and its WebUSB port, `nusb`'s WebUSB backend, `ftdi-nusb`: async USB for
  native and browser, user-gesture device requests driven from JavaScript.
- `shearwater-rs`: a clean-room protocol crate that does no I/O itself, with a separate
  BLE transport crate.
