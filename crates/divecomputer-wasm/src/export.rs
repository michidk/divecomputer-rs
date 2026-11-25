//! Export of downloaded dives to open dive-log formats.

use divecomputer_protocol::shearwater;
use divejson::Document;
use js_sys::{Date, Uint8Array};
use wasm_bindgen::prelude::*;

/// Exports raw Shearwater dives (the `data` of downloaded dives) as a DiveJSON document.
///
/// Each dive is re-parsed and becomes one DiveJSON dive; the device identity comes from
/// the log itself. `exported_at` is the browser's current time in UTC. Returns the
/// document as indented JSON text.
///
/// # Errors
///
/// Throws an `Error` naming the dive (by position) that failed to parse, or the DiveJSON
/// rule the document would break.
#[wasm_bindgen(js_name = exportDiveJson)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen passes JavaScript arrays by value"
)]
pub fn export_divejson(dives: Vec<Uint8Array>) -> Result<String, JsError> {
    Ok(divejson::to_string_pretty(&document(&dives)?)?)
}

/// Exports raw Shearwater dives as UDDF, through the DiveJSON document
/// [`export_divejson`] writes. Returns the UDDF XML text; what UDDF cannot carry is
/// left out.
///
/// # Errors
///
/// As [`export_divejson`], plus a failure to write the UDDF.
#[wasm_bindgen(js_name = exportUddf)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "wasm-bindgen passes JavaScript arrays by value"
)]
pub fn export_uddf(dives: Vec<Uint8Array>) -> Result<String, JsError> {
    let (xml, _report) = divejson::uddf::write(&document(&dives)?)?;
    Ok(xml)
}

/// Parses each raw dive and exports them all as one DiveJSON document.
fn document(dives: &[Uint8Array]) -> Result<Document, JsError> {
    let dives = dives
        .iter()
        .enumerate()
        .map(|(index, data)| {
            shearwater::parse(&data.to_vec())
                .map_err(|error| JsError::new(&format!("dive {index}: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let exported_at = String::from(Date::new_0().to_iso_string())
        .parse()
        .map_err(|error| JsError::new(&format!("exported_at: {error}")))?;
    let mut generator = divejson::Generator::new("divecomputer-rs");
    generator.version = Some(env!("CARGO_PKG_VERSION").into());
    Ok(divecomputer_export::to_divejson(
        &dives,
        exported_at,
        generator,
    )?)
}
