//! Browser backends, built on the Web APIs through `web-sys`.

#[cfg(feature = "ble")]
mod ble;
mod clock;

#[cfg(feature = "ble")]
pub use ble::WebBluetoothLink;
pub use clock::WebClock;

use divecomputer_core::Error;
use wasm_bindgen::{JsCast, JsValue};

/// Turns a rejected promise's value into a transport error with its message.
fn js_error(value: JsValue) -> Error {
    let message = match value.dyn_into::<js_sys::Error>() {
        Ok(error) => String::from(error.message()),
        Err(value) => value.as_string().unwrap_or_else(|| format!("{value:?}")),
    };
    Error::Transport(message)
}
