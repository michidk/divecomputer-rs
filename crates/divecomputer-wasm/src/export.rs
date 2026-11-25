//! Export of downloaded dives to open dive-log formats.

use divecomputer_protocol::shearwater;
use divejson::Document;
use js_sys::{Date, Uint8Array};
use serde::Serialize;
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

/// Converts a validated DiveJSON document to UDDF, with a report of conversion losses.
///
/// Returns `{ text, dive_count, notes }`. Each note names its kind, source path,
/// optional code and target member, and message. Conversion stays in the browser.
///
/// # Errors
///
/// Throws an error for invalid DiveJSON, an XML writing failure, or a serialization error.
#[wasm_bindgen(js_name = convertDiveJsonToUddf)]
pub fn convert_divejson_to_uddf(text: &str) -> Result<JsValue, JsError> {
    let doc = divejson::from_str(text)?;
    let (text, report) = divejson::uddf::write(&doc)?;
    conversion(&text, doc.dives.len(), &report)
}

/// Converts UDDF to a validated DiveJSON document, with a report of conversion decisions.
///
/// Returns `{ text, dive_count, notes }`, as [`convert_divejson_to_uddf`]. The export
/// timestamp is the browser's current time in UTC.
///
/// # Errors
///
/// Throws an error for malformed UDDF, validation failure, or a serialization error.
#[wasm_bindgen(js_name = convertUddfToDiveJson)]
pub fn convert_uddf_to_divejson(text: &str) -> Result<JsValue, JsError> {
    let exported_at = String::from(Date::new_0().to_iso_string()).parse()?;
    let (doc, report) = divejson::uddf::read(text, exported_at)?;
    conversion(&divejson::to_string_pretty(&doc)?, doc.dives.len(), &report)
}

fn conversion(
    text: &str,
    dive_count: usize,
    report: &divejson::uddf::Report,
) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct Note<'a> {
        kind: &'static str,
        code: Option<&'static str>,
        at: &'a str,
        member: Option<&'a str>,
        message: &'a str,
    }
    #[derive(Serialize)]
    struct Conversion<'a> {
        text: &'a str,
        dive_count: usize,
        notes: Vec<Note<'a>>,
    }
    let notes = report
        .iter()
        .map(|note| Note {
            kind: match note.kind {
                divejson::uddf::NoteKind::Absent => "absent",
                divejson::uddf::NoteKind::Inferred => "inferred",
                divejson::uddf::NoteKind::Resolved => "resolved",
                divejson::uddf::NoteKind::Dropped => "dropped",
                _ => "other",
            },
            code: note.code,
            at: &note.at,
            member: note.member.as_deref(),
            message: &note.message,
        })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&Conversion {
        text,
        dive_count,
        notes,
    })?)
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
