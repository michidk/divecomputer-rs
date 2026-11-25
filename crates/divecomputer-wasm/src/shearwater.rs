//! Shearwater downloads over Web Bluetooth.

use core::ops::ControlFlow;

use divecomputer_protocol::shearwater::{
    self, BleStream, CHARACTERISTIC_UUID, SERVICE_UUID, Shearwater, Variant,
};
use divecomputer_transport::web::{WebBluetoothLink, WebClock};
use js_sys::{Array, Function, Object, Reflect, Uint8Array};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::BluetoothDevice;

/// The Shearwater BLE service UUID. Pass it in `optionalServices` (or a filter's
/// `services`) to `navigator.bluetooth.requestDevice()`.
#[wasm_bindgen(js_name = shearwaterServiceUuid)]
#[must_use]
pub fn shearwater_service_uuid() -> String {
    SERVICE_UUID.into()
}

/// Downloads dives from a Shearwater dive computer over Web Bluetooth.
///
/// `device` comes from `navigator.bluetooth.requestDevice()`. Dives are downloaded newest
/// first and stop at the dive whose fingerprint equals `fingerprint`. `on_dive`, if given,
/// is called with each dive as it arrives.
///
/// Resolves to `{ serial, firmware, model, dives: [{ fingerprint, data, dive, parseError }] }`,
/// where `serial` is 8 hex digits, `fingerprint` and `data` are `Uint8Array`s, and either
/// `dive` holds the parsed dive (see [`parse_shearwater_dive`]) or `parseError` says why
/// parsing failed.
///
/// # Errors
///
/// Rejects with an `Error` whose message describes the transport or protocol failure.
#[wasm_bindgen(js_name = downloadShearwater)]
pub async fn download_shearwater(
    device: BluetoothDevice,
    fingerprint: Option<Vec<u8>>,
    on_dive: Option<Function>,
) -> Result<JsValue, JsError> {
    let variant = Variant::for_device_name(&device.name().unwrap_or_default());
    let link = WebBluetoothLink::connect(
        &device,
        SERVICE_UUID,
        CHARACTERISTIC_UUID,
        CHARACTERISTIC_UUID,
    )
    .await?;
    let mut shearwater =
        Shearwater::connect(BleStream::new(link, variant), WebClock, variant).await?;

    let identity = shearwater.identify().await?;
    let dives = Array::new();
    let mut callback_error = None;
    let result = shearwater
        .download(&identity, fingerprint.as_deref(), |dive| {
            let (parsed, error) = match parse_shearwater_dive(dive.data()) {
                Ok(parsed) => (parsed, JsValue::UNDEFINED),
                Err(error) => (JsValue::UNDEFINED, JsValue::from(error.message)),
            };
            let object = object(&[
                ("fingerprint", Uint8Array::from(dive.fingerprint()).into()),
                ("data", Uint8Array::from(dive.data()).into()),
                ("dive", parsed),
                ("parseError", error),
            ]);
            dives.push(&object);
            if let Some(on_dive) = &on_dive {
                if let Err(error) = on_dive.call1(&JsValue::UNDEFINED, &object) {
                    callback_error = Some(error);
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        })
        .await;
    // End the session even when the download failed, so the device leaves its
    // communication screen; the download error takes precedence.
    let closed = shearwater.close().await;
    result?;
    closed?;
    if let Some(error) = callback_error {
        return Err(JsError::new(&format!("onDive threw: {error:?}")));
    }

    Ok(object(&[
        ("serial", format!("{:08X}", identity.serial()).into()),
        ("firmware", identity.firmware().into()),
        ("model", format!("{:?}", identity.model()).into()),
        ("dives", dives.into()),
    ])
    .into())
}

/// A parse failure, with the message shown to JavaScript.
struct ParseError {
    message: String,
}

/// Parses a raw Shearwater dive (the `data` of a downloaded dive) into a plain object.
///
/// Field names follow the Rust dive model in `divecomputer-core` in `snake_case`. Units:
/// depth m, temperature °C, pressure bar, gas percent, times and durations in
/// milliseconds, `start` an ISO 8601 local date-time without offset, `utc_offset` seconds.
/// Unknown values are `null`.
///
/// # Errors
///
/// Throws an `Error` naming the offending byte or record if the log is malformed.
#[wasm_bindgen(js_name = parseShearwaterDive)]
pub fn parse_shearwater_dive_js(data: &[u8]) -> Result<JsValue, JsError> {
    parse_shearwater_dive(data).map_err(|error| JsError::new(&error.message))
}

fn parse_shearwater_dive(data: &[u8]) -> Result<JsValue, ParseError> {
    let fail = |message: String| ParseError { message };
    let dive = shearwater::parse(data).map_err(|error| fail(error.to_string()))?;
    dive.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| fail(error.to_string()))
}

fn object(fields: &[(&str, JsValue)]) -> Object {
    let object = Object::new();
    for (key, value) in fields {
        // Setting a property on a plain object does not fail.
        let _ = Reflect::set(&object, &JsValue::from_str(key), value);
    }
    object
}
