# Raw (vendor-class) USB transport

Some devices expose a vendor-specific USB interface (class `0xFF`) and define their own
transfer pattern: commands as control requests, data as bulk transfers. This document
describes the generic pattern and the one dive computer known to use it. USB-serial bridge
chips are also vendor-class devices, but are covered in [serial.md](serial.md) because they
present a byte stream.

## 1. Generic pattern

1. Find the device by VID:PID.
2. Select the configuration (normally configuration 1, already active).
3. Claim the stated interface. Pick the bulk IN and bulk OUT endpoints by address when the
   family specifies them, otherwise the first bulk endpoint of each direction.
4. Commands: zero-length or short vendor control requests, `bmRequestType = 0x40`
   (vendor, device recipient, host to device) or `0xC0` for device-to-host.
5. Data: bulk IN reads with a large buffer. Because the device sends a transfer as a series
   of maximum-size packets terminated by a **short packet** (or a zero-length packet when
   the total is a multiple of the packet size), a read returns at the end of each device
   transfer. Families use "fewer bytes than the buffer size" as the end of a response.

Rules:

- The bulk IN buffer SHOULD be a multiple of the endpoint's `wMaxPacketSize`; otherwise a
  babble/overflow error can occur.
- A read that times out with zero bytes is not necessarily an error; families say when it is.
- Hosts SHOULD use a generous timeout (2-3 s) on the first bulk read after a command; the
  device may prepare data before it starts streaming.

**Confidence:** likely.

## 2. Atomic Aquatics Cobalt / Cobalt 2 (`0471:0888`)

| Item | Value |
|------|-------|
| Interface | 0 |
| Endpoints | bulk IN `0x82`, bulk OUT `0x02` (one source picks the *last* bulk endpoint of each direction in the interface descriptor, which yields the same pair) |
| Bulk read buffer | 8192 bytes |
| Timeout | 2000-3000 ms per bulk read |

Commands are vendor control requests with no data stage:

```
bmRequestType  40
bRequest       see table
wValue         0000
wIndex         0000
wLength        0000
```

| bRequest | Purpose | Response on bulk IN |
|----------|---------|---------------------|
| `0x01` | Read version / identity | 14 bytes + `sum16` (u16le) |
| `0x09` | Start: send the most recent dive | dive record, then `sum16` (u16le) |
| `0x0A` | Send the next (older) dive | same; or exactly `FF FF` when there are no more dives |
| `0x02` / `0x03` | Same as `0x09` / `0x0A` against a simulator build | (development only) |

The host reads bulk IN repeatedly with an 8192-byte buffer and appends; a read returning
fewer than 8192 bytes ends the record. The last two bytes of the record are the 16-bit
additive checksum of all preceding bytes of that record
([../common/checksums.md#sum16](../common/checksums.md#sum16)), little-endian. A record of
exactly two bytes `FF FF` means "end of logbook". Details of the record are in the Atomic
Aquatics family document.

Exchange:

```
H→D control  40 09 0000 0000 0000
D→H bulk     <8192 bytes>
D→H bulk     <8192 bytes>
D→H bulk     <n < 8192 bytes>          last part, ends with sum16
H→D control  40 0A 0000 0000 0000
...
D→H bulk     FF FF                     no more dives
```

One source additionally retries a failed bulk read up to 3 times and treats a 64-byte read
like a full one; another stops on any read shorter than the buffer.

**Confidence:** likely (two sources agree on requests, end marker and checksum).

## 3. Web notes

WebUSB fits this transport directly: `requestDevice({filters:[{vendorId:0x0471,
productId:0x0888}]})`, `selectConfiguration(1)`, `claimInterface(0)`,
`controlTransferOut({requestType:'vendor', recipient:'device', request:0x09, value:0,
index:0})`, then `transferIn(2, 8192)` (endpoint number without the direction bit) in a loop.
On Windows the device needs a WinUSB-compatible driver binding.

**Confidence:** likely.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Universal Serial Bus Specification 2.0, chapter 9 (control requests) and 5.8 (bulk transfers, short packets), https://www.usb.org/document-library/usb-20-specification
- WebUSB API, https://wicg.github.io/webusb/
