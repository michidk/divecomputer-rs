# USB HID transport

A few dive computers present a USB Human Interface Device interface and tunnel their
protocol through HID reports, so that no driver installation is needed on desktop systems.
The HID report descriptor carries no useful semantics for these devices; the reports are
opaque fixed-size containers.

## 1. Devices

| VID:PID | Product | Family | OUT report | IN report | Report ID |
|---------|---------|--------|------------|-----------|-----------|
| `C251:2006` | Scubapro Aladin Square | Uwatec Smart | 32 bytes | 64 bytes | none (0) |
| `2E6C:3201` | Scubapro G2, G2 TEK | Uwatec Smart | 32 bytes | 64 bytes | none (0) |
| `2E6C:3211` | Scubapro G2 Console | Uwatec Smart | 32 bytes | 64 bytes | none (0) |
| `2E6C:4201` | Scubapro G2 HUD | Uwatec Smart | 32 bytes | 64 bytes | none (0) |
| `1493:0030` | Suunto EON Steel | Suunto EON Steel | 64 bytes incl. leading `3F` | 64 bytes incl. leading `3F` | see 3.2 |
| `1493:0033` | Suunto EON Core | Suunto EON Steel | as above | as above | see 3.2 |
| `1493:0035` | Suunto D5 | Suunto EON Steel | as above | as above | see 3.2 |
| `1493:0036` | Suunto EON Steel Black | Suunto EON Steel | as above | as above | see 3.2 |

The G3 (model code `0x34`) is listed with HID transport in one source but its USB PID is not
given.

**Confidence:** likely.

## 2. Generic model

- The host opens the first interface of class 3 (HID) and uses its interrupt IN and interrupt
  OUT endpoints. Both devices families use **output reports on the interrupt OUT endpoint**,
  not `SET_REPORT` control transfers.
- Each host-to-device message is exactly one output report of the stated size, zero padded.
- Each device-to-host message is one input report. Multi-report answers are concatenated by
  the family protocol using an in-report length field.
- Reads block until a report arrives or the timeout expires. Typical timeouts: 200 ms per
  report (Uwatec; one source uses 50 ms during bulk transfer), 5000 ms (Suunto).

Report-ID handling differs between host APIs:

- With raw interrupt transfers (libusb, Android `bulkTransfer` on the interrupt endpoint,
  WebUSB where allowed) the bytes on the wire are exactly the report contents; there is no
  report-ID byte for devices that do not declare report IDs.
- With OS HID APIs (hidapi, Windows HID, WebHID) the caller supplies a report ID. For devices
  without numbered reports this is 0 and is **not** transmitted; a 32-byte report is written
  as a 33-byte buffer `00 <32 bytes>` (hidapi) or as `sendReport(0, <32 bytes>)` (WebHID).
- For a device whose descriptor declares report ID `0x3F` the ID byte is transmitted and is
  part of the 64 bytes on the wire.

**Confidence:** likely.

## 3. Per-family report layouts

### 3.1 Uwatec Smart (Aladin Square, G2 family)

OUT report, 32 bytes:

```
[0]      n = number of command bytes (command code + arguments)
[1..n]   command code, then arguments
[n+1..]  00 padding to 32 bytes
```

Example, model query (command `10`):

```
H→D  01 10 00 00 00 00 00 00 00 00 00 00 00 00 00 00
     00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
```

IN report, 64 bytes: byte 0 is the number of valid data bytes that follow (at most 63); the
data follows at offset 1. Hosts MUST cap the copied length at `min(byte0, received − 1)`.
One source accepts only reports of exactly 64 bytes.

There is no checksum at this layer.

**Confidence:** likely (two sources agree on sizes and the length prefix).

### 3.2 Suunto EON Steel family

Every report, both directions, is 64 bytes:

```
[0]      3F            report type / ID
[1]      L             number of payload bytes that follow (≤ 62)
[2..2+L) payload       a fragment of the family's message stream
[...]    00 padding
```

The family protocol (12-byte message header, payload, `crc-32-iso-hdlc`; see the Suunto EON
Steel family document and [../common/checksums.md#crc-32-iso-hdlc](../common/checksums.md#crc-32-iso-hdlc))
runs on top. In host-to-device reports one reference implementation places the 4-byte CRC
directly after the `L` counted bytes (it is not included in `L`) and pads the rest with
zeros; whether the device checks it over USB is unknown. Over BLE the same messages are HDLC
framed with the CRC inside the frame (see [ble.md](ble.md)).

For OS HID APIs: write report ID `0x3F` followed by 63 bytes; read reports whose first byte
is `0x3F`.

**Confidence:** likely.

## 4. Web notes

WebHID is the only browser path: HID is a protected interface class in WebUSB. Request the
device with a `vendorId`/`productId` filter. Use `device.sendReport(reportId, data)` where
`data` excludes the report ID, and the `inputreport` event, whose `data` also excludes the
report ID (for the Suunto devices, re-insert `3F` mentally: `event.reportId == 0x3F` and
`data[0]` is the length byte `L`).

**Confidence:** likely.

## Open questions

- USB PID of the Scubapro G3 and Luna 2.0 when connected by cable (if any).
- Whether the Uwatec 50 ms per-report timeout during bulk download is required or merely an
  optimisation.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- USB Device Class Definition for HID 1.11, https://www.usb.org/document-library/device-class-definition-hid-111
- WebHID API, https://wicg.github.io/webhid/
