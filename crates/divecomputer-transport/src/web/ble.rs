use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};

use divecomputer_core::{Error, Result, transport::PacketLink};
use js_sys::{Array, Function, Promise, Uint8Array};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    BluetoothDevice, BluetoothRemoteGattCharacteristic, BluetoothRemoteGattServer, Event,
};

use super::{clock::sleep, js_error};

/// Web Bluetooth only exposes the negotiated MTU to the browser, so writes use the
/// default ATT payload of 20 bytes (spec: BLE transport, "Web Bluetooth notes").
const MAX_PACKET_LEN: usize = 20;
const VALUE_CHANGED: &str = "characteristicvaluechanged";

/// Notifications received but not read yet, and the resolver of a pending receive.
#[derive(Default)]
struct Inbox {
    packets: VecDeque<Vec<u8>>,
    wake: Option<Function>,
}

/// [`PacketLink`] over a GATT write characteristic and a notify characteristic, using
/// Web Bluetooth.
///
/// The page obtains the [`BluetoothDevice`] with `navigator.bluetooth.requestDevice()`
/// from a user gesture and must list the service in `filters` or `optionalServices`.
/// Dropping the link stops listening and disconnects.
pub struct WebBluetoothLink {
    server: BluetoothRemoteGattServer,
    write: BluetoothRemoteGattCharacteristic,
    notify: BluetoothRemoteGattCharacteristic,
    inbox: Rc<RefCell<Inbox>>,
    listener: Closure<dyn FnMut(Event)>,
}

impl WebBluetoothLink {
    /// Connects to `device`, looks up the characteristics in `service` (UUIDs in
    /// lowercase 128-bit form) and enables notifications. `write` and `notify` may be the
    /// same characteristic.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] with the browser's message if connecting, the lookup or
    /// enabling notifications fails.
    pub async fn connect(
        device: &BluetoothDevice,
        service: &str,
        write: &str,
        notify: &str,
    ) -> Result<Self> {
        let server = device
            .gatt()
            .ok_or(Error::Unsupported("the device has no GATT server"))?;
        let server = server.connect().await.map_err(js_error)?;
        let service = server
            .get_primary_service_with_str(service)
            .await
            .map_err(js_error)?;
        let write = service
            .get_characteristic_with_str(write)
            .await
            .map_err(js_error)?;
        let notify = service
            .get_characteristic_with_str(notify)
            .await
            .map_err(js_error)?;

        let inbox = Rc::new(RefCell::new(Inbox::default()));
        let listener = Closure::<dyn FnMut(Event)>::new({
            let inbox = Rc::clone(&inbox);
            move |event: Event| {
                let Some(value) = event
                    .target()
                    .and_then(|target| target.dyn_into::<BluetoothRemoteGattCharacteristic>().ok())
                    .and_then(|characteristic| characteristic.value())
                else {
                    return;
                };
                let bytes = Uint8Array::new_with_byte_offset_and_length(
                    &value.buffer(),
                    u32::try_from(value.byte_offset()).unwrap_or(0),
                    u32::try_from(value.byte_length()).unwrap_or(0),
                )
                .to_vec();
                let mut inbox = inbox.borrow_mut();
                inbox.packets.push_back(bytes);
                if let Some(wake) = inbox.wake.take() {
                    // Resolving a promise does not throw.
                    let _ = wake.call0(&wasm_bindgen::JsValue::UNDEFINED);
                }
            }
        });
        notify
            .add_event_listener_with_callback(VALUE_CHANGED, listener.as_ref().unchecked_ref())
            .map_err(js_error)?;
        notify.start_notifications().await.map_err(js_error)?;

        Ok(Self {
            server,
            write,
            notify,
            inbox,
            listener,
        })
    }
}

impl PacketLink for WebBluetoothLink {
    fn max_packet_len(&self) -> usize {
        MAX_PACKET_LEN
    }

    async fn send(&mut self, packet: &[u8]) -> Result<()> {
        self.write
            .write_value_with_response_with_u8_slice(packet)
            .map_err(js_error)?
            .await
            .map_err(js_error)?;
        Ok(())
    }

    async fn receive(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        if let Some(packet) = self.inbox.borrow_mut().packets.pop_front() {
            return Ok(packet);
        }
        let notified = Promise::new(&mut |resolve, _reject| {
            self.inbox.borrow_mut().wake = Some(resolve);
        });
        // Neither promise rejects; whichever settles first ends the wait.
        let _ = Promise::race(&Array::of2(&notified, &sleep(timeout))).await;
        let mut inbox = self.inbox.borrow_mut();
        inbox.wake = None;
        inbox.packets.pop_front().ok_or(Error::Timeout)
    }

    fn purge(&mut self) -> impl Future<Output = Result<()>> {
        self.inbox.borrow_mut().packets.clear();
        core::future::ready(Ok(()))
    }
}

impl Drop for WebBluetoothLink {
    fn drop(&mut self) {
        // Best effort: the page may already be disconnected.
        let _ = self.notify.remove_event_listener_with_callback(
            VALUE_CHANGED,
            self.listener.as_ref().unchecked_ref(),
        );
        self.server.disconnect();
    }
}

impl core::fmt::Debug for WebBluetoothLink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WebBluetoothLink")
            .field("write", &self.write.uuid())
            .field("notify", &self.notify.uuid())
            .finish_non_exhaustive()
    }
}
