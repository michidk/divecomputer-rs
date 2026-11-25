# divecomputer-transport

Transport backends for the protocols in `divecomputer-protocol`, part of
[divecomputer-rs](https://github.com/michidk/divecomputer-rs). Each backend implements a
transport trait from `divecomputer-core`.

| Feature | Target | Backend |
| --- | --- | --- |
| `ble` | browser (`wasm32-unknown-unknown`) | Web Bluetooth: `web::WebBluetoothLink` |
| — | browser | `web::WebClock` (`setTimeout`) |

Browser builds need `--cfg=web_sys_unstable_apis` in `RUSTFLAGS`;
the repository's `.cargo/config.toml` sets it.

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
