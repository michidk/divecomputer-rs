# divecomputer-export

Exports dives downloaded by [divecomputer-rs](https://github.com/michidk/divecomputer-rs)
to open dive-log formats. Today that is [DiveJSON](https://divejson.org), through the
[`divejson`](https://github.com/michidk/divejson-rs) crate. The crate is `no_std` with
`alloc` and reads only the dive model of `divecomputer-core`.

```rust,ignore
let dive = divecomputer_protocol::shearwater::parse(raw.data())?;
let exported_at = "2026-11-25T12:00:00+01:00".parse()?; // the caller's clock, with offset
let document = divecomputer_export::to_divejson(
    &[dive],
    exported_at,
    divejson::Generator::new("my-app"),
)?;
let text = divejson::to_string_pretty(&document)?;
```

Each dive becomes one DiveJSON dive with one recording. Nothing is invented: a value the
device did not record stays absent. A dive's uuid is a UUIDv5 of its brand, serial, device
dive number and start under `DIVE_NAMESPACE`, so exporting the same dive again yields the
same uuid. The returned document has passed `divejson::Document::validate`.

For UDDF, enable the `uddf` feature of `divejson` and write the document with
`divejson::uddf::write`.

No feature flags. Until `divejson` is published, the crate builds against a
[divejson-rs](https://github.com/michidk/divejson-rs) checkout next to this repository.

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
