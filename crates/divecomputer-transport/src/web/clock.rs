use core::time::Duration;

use divecomputer_core::transport::Clock;
use js_sys::{Function, Promise};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    // The global `setTimeout`, available in windows and workers.
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(handler: &Function, milliseconds: i32) -> wasm_bindgen::JsValue;
}

/// A promise that resolves after `duration` (with `undefined`).
pub(crate) fn sleep(duration: Duration) -> Promise {
    let milliseconds = i32::try_from(duration.as_millis()).unwrap_or(i32::MAX);
    Promise::new(&mut |resolve, _reject| {
        set_timeout(&resolve, milliseconds);
    })
}

/// [`Clock`] on the browser's `setTimeout`.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebClock;

impl Clock for WebClock {
    async fn sleep(&self, duration: Duration) {
        // A timer promise never rejects.
        let _ = sleep(duration).await;
    }
}
