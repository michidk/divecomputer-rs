# divecomputer-wasm

JavaScript bindings for [divecomputer-rs](https://github.com/michidk/divecomputer-rs):
download dive computers from the browser. Today it downloads dives from Shearwater
computers over Web Bluetooth (Chromium-based browsers only) and parses them.

## Build

The crate builds against [divejson-rs](https://github.com/michidk/divejson-rs) and
[uddf-rs](https://github.com/michidk/uddf-rs), checked out next to this repository.

```sh
cargo build -p divecomputer-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir crates/divecomputer-wasm/www/pkg \
  "$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)/wasm32-unknown-unknown/release/divecomputer_wasm.wasm"
```

The `wasm-bindgen` CLI version must equal the `wasm-bindgen` crate version pinned in the
workspace `Cargo.toml` (`cargo install wasm-bindgen-cli --version 0.2.129`). The build
needs `--cfg=web_sys_unstable_apis`, which the repository's `.cargo/config.toml` sets.

## Test page

`www/index.html` connects to a Shearwater computer, downloads every dive, shows a summary
of each parsed dive and lets you save the raw and parsed dives as JSON, or the parsed dives
as a [DiveJSON](https://divejson.org) document or as UDDF. Serve `www/` over HTTPS or from
`localhost` (Web Bluetooth needs a secure context), for example
`python3 -m http.server --directory crates/divecomputer-wasm/www`, and open it in Chrome or
Edge.

## API

```js
import init, { downloadShearwater, exportDiveJson, exportUddf, parseShearwaterDive, shearwaterServiceUuid } from "./pkg/divecomputer_wasm.js";

await init();
const device = await navigator.bluetooth.requestDevice({
  filters: [{ namePrefix: "Perdix" }],
  optionalServices: [shearwaterServiceUuid()],
});
const { serial, firmware, model, dives } = await downloadShearwater(
  device,
  null, // or the fingerprint of the newest dive you already have
  (dive) => console.log("dive", dive.fingerprint, dive.data.length, dive.dive ?? dive.parseError),
);

// Re-parse a stored raw dive.
const parsed = parseShearwaterDive(dives[0].data);

// Export raw dives as a DiveJSON document (indented JSON text), stamped with the current time.
const divejson = exportDiveJson(dives.map((dive) => dive.data));
// The same document as UDDF XML.
const uddf = exportUddf(dives.map((dive) => dive.data));
```

Each downloaded dive carries `dive`, the parsed dive as a plain object, or `parseError`
when parsing failed. Parsed dives use the `snake_case` field names of the Rust model in
`divecomputer-core`, depths in metres, temperatures in °C, pressures in bar, gas shares
in percent, times in milliseconds and `null` for unknown values.

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
