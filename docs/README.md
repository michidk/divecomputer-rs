# Dive Computer Protocols

An open, implementation-neutral specification of how dive logs are downloaded from dive
computers and how the returned dive data is encoded. It covers about 40 protocol families
across more than 300 models, from serial cables and infrared to USB HID and Bluetooth Low
Energy.

The specification is written from scratch and licensed under MIT OR Apache-2.0, so that
implementations in any language can use permissive licenses. It is the basis for the Rust
crates of [divecomputer-rs](https://github.com/michidk/divecomputer-rs).

Status: draft. Most facts are marked *likely*; only facts backed by vendor documentation or
captured device traffic are marked *verified*. Corrections, captures and dumps are welcome.

## Browser tools

- [Download dives](tools/index.html) from a Shearwater computer over Web Bluetooth,
  or reopen a raw JSON download saved by the downloader. Save as DiveJSON or UDDF.
- [Convert dive logs](tools/convert.html) between DiveJSON and UDDF, with a report
  of omitted values and conversion decisions.

Both tools run on your device. Dive files are not sent to a server. The downloader
needs Chrome or Edge and a Bluetooth adapter; file conversion does not need Bluetooth.

## Where to start

- [Specification overview](spec/README.md): layout and clean-room policy
- [Conventions](spec/conventions.md): notation, units and confidence levels
- [Transports](spec/transports/README.md): serial, USB, Bluetooth, IrDA, and which web API fits each
- [Checksums](spec/common/checksums.md), [date and time](spec/common/datetime.md) and the [common data model](spec/common/data-model.md)
- [Protocol families](spec/families/README.md): one document per family
