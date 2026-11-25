# Transports

This directory describes the link layers that the family documents in `../families/` build
on. A family document names its transport, its line or GATT parameters, and refers here for
everything that is shared: how to drive a USB-serial bridge chip, how HID reports are sized,
how a GATT link is turned into a byte stream, how IrDA dongles are spoken to, and how
file-based devices are read.

| Document | Covers |
|----------|--------|
| [serial.md](serial.md) | Generic byte-stream serial model (line settings, DTR/RTS, break, purge, timeouts); FTDI, CP210x, PL2303 and CDC-ACM driven directly over USB control/bulk transfers |
| [usb-hid.md](usb-hid.md) | Devices that tunnel their protocol through HID interrupt reports |
| [usb-raw.md](usb-raw.md) | Vendor-class USB devices driven with control and bulk transfers |
| [bluetooth-classic.md](bluetooth-classic.md) | RFCOMM / Serial Port Profile links |
| [ble.md](ble.md) | Bluetooth Low Energy "GATT serial" links, plus the consolidated table of every known vendor service and characteristic UUID and advertised-name pattern |
| [irda.md](irda.md) | IrLAP, IrLMP, IAS and TinyTP as used by dive computers; USB IrDA dongles |
| [usb-storage-mtp.md](usb-storage-mtp.md) | Devices exposing their logs as files over USB mass storage or MTP |

## Layering model

Every family protocol in this specification is written against one of two abstract
interfaces:

1. **Byte stream.** `write(bytes)`, `read(n, timeout)`, `purge()`, plus optional serial-only
   controls (`configure(baud, bits, parity, stop)`, `set_dtr`, `set_rts`, `set_break`).
   Serial ports, USB-serial bridges, CDC-ACM, RFCOMM, BLE "GATT serial" links and IrDA
   (after the IrLMP/TinyTP connection is up) all present this interface. Some families add
   their own packetisation on top (for example HDLC-style framing over BLE); that is
   described in the family document or, where shared, in [ble.md](ble.md).
2. **Message / file.** USB HID reports, raw vendor USB transfers, USB mass storage and MTP do
   not behave like a byte stream. The family document describes the exact transfer pattern.

Hosts SHOULD keep the family protocol independent of the transport implementation, so that
the same family code runs over, for example, a native serial port, a WebUSB FTDI driver or
Web Serial.

## USB identification

Sorted by VID:PID. "Family" lists the protocol families known to sit behind the ID; generic
bridge chips are used by many unrelated products and MUST NOT by themselves be taken as
proof of a particular dive computer. Confidence per row: **V** = verified against a public
vendor/standards document, **L** = likely (one reliable source), **U** = uncertain.

| VID:PID | Device / chip | Class seen by host | Families / products | Conf. |
|---------|---------------|--------------------|---------------------|-------|
| `0403:6001` | FTDI FT232R / FT232BM | vendor (FTDI) | Generic cable chip; used by several vendors' interface cables. Also seen on FTDI-based IrDA adapters | L |
| `0403:6015` | FTDI FT-X series | vendor (FTDI) | Generic | L |
| `0403:87D0` | FTDI, custom PID | vendor (FTDI) | Cressi PC interface cable | V (public FTDI PID registry as shipped in Linux) |
| `0403:F460` | FTDI, custom PID | vendor (FTDI) | Oceanic / Pelagic dive-instrument cable | V (same) |
| `0403:F680` | FTDI, custom PID | vendor (FTDI) | Suunto sports-instrument cable | V (same) |
| `0403:03EA`, `0403:03EB` | FTDI VID, unknown PIDs | ? | Listed in a USB device filter without further use | U |
| `040E:0568` | unidentified | mass storage | A file-based dive computer (vendor not determined) | U |
| `0471:0888` | Philips VID, custom device | vendor class, bulk | Atomic Aquatics Cobalt / Cobalt 2 ([usb-raw.md](usb-raw.md)) | L |
| `066F:4200` | SigmaTel STIr4200 | vendor (raw SIR) | USB IrDA dongle ([irda.md](irda.md)) | L |
| `066F:4210` | SigmaTel STIr4210/4220 | USB IrDA bridge class | USB IrDA dongle; also reported by a third-party multi-mode adapter | L |
| `066F:4310` | SigmaTel (IrDA bridge family) | USB IrDA bridge class | USB IrDA dongle | U |
| `067B:2303` | Prolific PL2303 | vendor (Prolific) | Generic older cable chip | L |
| `091E:0003` | Garmin | mass storage | Garmin Descent Mk1 (file access) | L |
| `091E:2B2B` | Garmin | mass storage | Garmin Descent Mk1 (file access) | L |
| `091E:4CBA` | Garmin | MTP | Garmin Descent Mk2 / Mk2i | L |
| `091E:51B6` | Garmin | MTP | Garmin fēnix 8 (43 mm) | L |
| `091E:4000 \| id` | Garmin | MTP | Any Garmin wearable whose FIT product id is `id` (see [usb-storage-mtp.md](usb-storage-mtp.md)) | L |
| `0930:6544` | Toshiba flash drive | mass storage | Probably a test device, not a dive computer | U |
| `0AAD:00EB` | unknown | ? | Listed in a USB device filter without further use | U |
| `0DF8:0001` | unidentified | vendor (raw SIR, STIr4200-compatible) | USB IrDA dongle | U |
| `0E55:110A` | unidentified | USB IrDA bridge class | USB IrDA dongle | U |
| `10C4:EA60` | Silicon Labs CP210x | vendor (SiLabs) | Generic cable chip; used by e.g. Uwatec Meridian/Chromis/Mantis cradles and ReefNet Sensus Ultra. Some third-party adapters reuse it with a different personality (see [serial.md](serial.md)) | L |
| `1493:0030` | Suunto | HID | Suunto EON Steel | L |
| `1493:0033` | Suunto | HID | Suunto EON Core | L |
| `1493:0035` | Suunto | HID | Suunto D5 | L |
| `1493:0036` | Suunto | HID | Suunto EON Steel Black | L |
| `1685:0200` | unidentified | vendor (raw SIR, STIr4200-like, different init) | USB IrDA dongle | U |
| `2047:082B` | Texas Instruments VID | CDC-ACM | Aeris / Oceanic A300 CS class device | U |
| `2544:0004` | unidentified | mass storage | A file-based dive computer | U |
| `2E6C:3201` | Scubapro | HID | Scubapro G2, G2 TEK | L |
| `2E6C:3211` | Scubapro | HID | Scubapro G2 Console | L |
| `2E6C:4201` | Scubapro | HID | Scubapro G2 HUD | L |
| `9710:7780` | MosChip MCS7780 | vendor (raw SIR) | USB IrDA dongle | L |
| `A600:E211` | unidentified | mass storage | A file-based dive computer | U |
| `C251:2006` | Keil VID | HID | Scubapro Aladin Square | L |
| `FFFF:0005` | (unregistered VID) | CDC-ACM | Mares Icon HD family USB cable/cradle | L |

`04E8:6860` (a Samsung phone's MTP personality) also appears in one source's MTP list and is
almost certainly a developer test device.

**Confidence:** likely overall; individual rows as marked.

## Which web API fits each transport

| Transport | Browser API | Notes |
|-----------|-------------|-------|
| Native serial port / OS-driven USB-serial | Web Serial | Chromium desktop. The OS driver handles the bridge chip; the page only sees a port. DTR/RTS via `setSignals`, break via `setSignals({break})`. |
| USB-serial bridge without OS driver (Android, kiosk) | WebUSB | The page implements the chip driver itself using the control requests in [serial.md](serial.md). Requires that no OS driver holds the interface. |
| CDC-ACM | Web Serial (OS driver present) or WebUSB | Same split as above. |
| USB HID | WebHID | HID is a protected class in WebUSB and cannot be claimed there. |
| Vendor-class USB | WebUSB | Straightforward: claim interface, control + bulk transfers. |
| Bluetooth classic (RFCOMM) | Web Serial with Bluetooth RFCOMM ports | Chromium desktop only; the device must already be paired at OS level. No browser API for discovery or PIN entry. |
| Bluetooth Low Energy | Web Bluetooth | Service UUIDs must be declared up front (`filters` / `optionalServices`); no MTU control; see [ble.md](ble.md). |
| IrDA | WebUSB to a USB IrDA dongle | The page runs the whole IrLAP/IrLMP/TinyTP stack ([irda.md](irda.md)). No OS IrDA socket API is reachable from a browser. |
| USB mass storage | File System Access API (directory picker) | Mass storage is a protected WebUSB class; the OS mounts the volume and the user picks the folder. |
| MTP | none reliable | The OS usually owns the device. Desktop users copy files out; WebUSB is possible only where no OS MTP driver claims the interface. |

**Confidence:** likely (browser behaviour changes between versions; check current
Chromium documentation).

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied.
  https://www.libdivecomputer.org/
- Linux kernel FTDI product-ID list (public VID/PID registry), https://github.com/torvalds/linux/blob/master/drivers/usb/serial/ftdi_sio_ids.h
- Garmin FIT SDK product ids, https://developer.garmin.com/fit/
- WebUSB specification (protected interface classes), https://wicg.github.io/webusb/
- WebHID, Web Serial, Web Bluetooth and File System Access drafts, https://wicg.github.io/
