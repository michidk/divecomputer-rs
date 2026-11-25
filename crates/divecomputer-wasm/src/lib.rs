//! JavaScript bindings: download dive computers from the browser.
//!
//! Exists only on `wasm32-unknown-unknown`. Build it with `wasm-bindgen --target web`;
//! the crate README has the commands and `www/` holds a test page.

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod export;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod shearwater;
