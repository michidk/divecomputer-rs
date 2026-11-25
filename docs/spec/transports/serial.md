# Serial transport

Most older dive computers, and many newer ones behind a USB cable, speak an asynchronous
serial protocol. The host side is either a real UART, an OS-provided virtual port
(USB-serial bridge or CDC-ACM with an OS driver), or a USB-serial bridge driven directly by
the host (WebUSB, Android without kernel drivers). This document defines the abstract
serial model the family documents use, and then shows how to implement it on top of raw USB
for the four bridge types found in dive computer cables.

## 1. Abstract serial model

A serial transport provides:

| Operation | Meaning |
|-----------|---------|
| `configure(baud, data_bits, parity, stop_bits, flow)` | Set line coding. Families use 5..8 data bits only as 8; parity none/odd/even; 1 or 2 stop bits; flow control is always **none** in this specification. |
| `set_dtr(on)`, `set_rts(on)` | Drive the modem-control outputs. "on" means the line is **asserted** (RS-232 positive voltage, TTL low on most bridge chips). |
| `set_break(on)` | Hold TX in the spacing (break) state while on. |
| `purge(rx, tx)` | Discard bytes buffered in the host and, where possible, in the bridge chip. |
| `write(bytes)` | Send all bytes. Writes are not framed; the device sees a continuous byte stream. |
| `read(n, timeout)` | Return up to `n` bytes; return fewer (possibly 0) when `timeout` expires. |
| `sleep(ms)` | Delays are part of several protocols (wake-up sequences, inter-command gaps). |

### 1.1 Timeouts

A family states a read timeout in milliseconds. Its meaning is: the total time a single
`read(n)` call may wait for all `n` bytes. A read that times out MUST report how many bytes
did arrive, because several protocols use "fewer bytes than asked" as an end-of-data signal
or retry trigger. A timeout of 0 means "return what is buffered now"; an infinite timeout is
never needed by any family. Typical values: 1000 ms for interactive command/response
protocols, 3000-5000 ms for protocols where the device prepares data before answering.

**Confidence:** likely.

### 1.2 DTR and RTS

Many interface cables are powered or switched by the modem-control lines:

- Some cables take power for the IR or level-shifter circuitry from DTR (and sometimes RTS);
  the family then asserts DTR at open and keeps it asserted.
- Some half-duplex single-wire cables use RTS as a transmit-enable or direction line; the
  family toggles RTS around each write.
- Some devices use a DTR or RTS edge as a wake-up or reset signal.

Families MUST state the initial level of both lines after `configure`, and every later
change. Implementations MUST NOT rely on the OS default line state (it differs between
operating systems and chip drivers).

Note on terminology: some host APIs model these lines with inverted booleans. In this
specification "DTR on" always means the line is asserted, whatever the host API calls it.

**Confidence:** likely.

### 1.3 Break

A small number of families (ReefNet Sensus Pro, Cochran) signal "wake up" or "start" by
holding a break condition for a stated time (for example 16 ms) or for the duration of a
handshake read. Implementations without break support cannot talk to these devices.

**Confidence:** likely.

### 1.4 Purge

Before a command exchange, families usually purge both directions to drop stale bytes (for
example heartbeat bytes or a previous partial answer). When the bridge chip has its own
FIFOs (FTDI, CP210x), a purge SHOULD also be sent to the chip (requests below); purging only
the host buffer can leave up to one USB packet of stale data.

**Confidence:** likely.

### 1.5 Echo on single-wire cables

Several IR and single-wire cables loop the host's transmitted bytes back into its receiver.
Families that use such cables say so; the host then reads and discards exactly as many bytes
as it wrote before reading the answer. This is a property of the cable, not of the serial
transport, but it is common enough to note here.

**Confidence:** likely.

### 1.6 Overview of line settings by family

For orientation only; the family documents are normative.

| Baud | Format | Families (initial DTR / RTS where set) |
|------|--------|----------------------------------------|
| 1200 | 8N2 | Suunto Eon, Suunto Solution (RTS off) |
| 1200 → 4800 | 8N1 | Cressi Edy family (DTR on, RTS off; rate switched during session) |
| 2400 | 8O1 | Suunto Vyper family (DTR on; RTS toggled as direction line) |
| 4800 | 8N1 | Citizen Aqualand (DTR pulsed), Zeagle N2iTiON3 |
| 9600 | 8N1 | Suunto Vyper2/D9 (DTR on, RTS toggled), Mares Nemo/Darwin (DTR on, RTS on), Oceanic VT Pro and Veo 250 (DTR on, RTS pulsed), Uwatec Memomouse (DTR toggled), Dive Rite NiTek Q, Liquivision (DTR off, RTS off) |
| 9600 → 115200 | 8N1 | Suunto D9 family (auto-detected), HW OSTC 2 (19200 or 115200 probed) |
| 9600 then 115200 / 850000 | 8N2 | Cochran (break used as wake-up) |
| 19200 | 8N1 | Uwatec Aladin (DTR on, RTS off), ReefNet Sensus and Sensus Pro (break) |
| 38400 | 8N1 | Mares Puck (DTR off, RTS off), Oceanic Atom 2 family (DTR on, RTS pulsed) |
| 57600 | 8N1 | Uwatec Smart over serial cradles (Meridian, Chromis, Mantis) |
| 115200 | 8N1 | Oceanic Atom 2 family fast models (VTX, i750TC, Pro Plus X, i770R), HW OSTC/OSTC 3, Shearwater (over serial), Cressi Leonardo (DTR, RTS set), Cressi Goa (DTR off, RTS off), Ratio/DiveSystem iDive, ReefNet Sensus Ultra, McLean Extreme, Tecdiving, Seac Screen, Deep Six, Divesoft |
| 115200 | 8E1 | Mares Icon HD family (DTR off, RTS off) |
| 460800 | 8N1 | Sporasub SP2 (DTR on, RTS off) |

One source reports that the Mares Icon HD CDC cradle (`FFFF:0005`) is opened at **256000**
baud with no parity; another configures the same family at **115200 8E1**. Both are
recorded; see open questions.

**Confidence:** likely (single reference implementation per row; the Mares 256000/115200
split is uncertain).

## 2. USB-serial bridges driven directly

When the host has no OS driver (WebUSB, Android), it MUST speak the bridge chip's own
control protocol. All requests below are USB control transfers with the 8-byte setup packet
`bmRequestType, bRequest, wValue (LE), wIndex (LE), wLength (LE)`. Data is moved with bulk
transfers on the interface's bulk IN and bulk OUT endpoints (find them from the interface
descriptor; do not hard-code endpoint numbers).

### 2.1 FTDI (FT232R, FT232BM, FT-X; VID `0403`)

Interface 0, one bulk IN and one bulk OUT endpoint. All requests are vendor requests to the
device: `bmRequestType = 0x40` (host to device) or `0xC0` (device to host). `wIndex` carries
the port number: 0 (or 1) on single-port chips; on multi-port chips port A = 1, B = 2, ...

| Purpose | bRequest | wValue | wIndex | Data |
|---------|----------|--------|--------|------|
| Reset port | `0x00` | `0x0000` | port | none |
| Purge RX buffer | `0x00` | `0x0001` | port | none |
| Purge TX buffer | `0x00` | `0x0002` | port | none |
| Modem control | `0x01` | see below | port | none |
| Flow control | `0x02` | `0x0000` = none (`0x0100` RTS/CTS, `0x0200` DTR/DSR, `0x0400` XON/XOFF) | port | none |
| Baud rate | `0x03` | divisor low 16 bits | divisor bit 16 in bit 0 (single-port) / `(port) \| bit16<<8` (multi-port) | none |
| Line format | `0x04` | see below | port | none |
| Modem status (read) | `0x05` | 0 | port | 2 bytes (same layout as the RX status header) |
| Latency timer | `0x09` | milliseconds (1..255) | port | none |
| Read latency timer | `0x0A` (`0xC0`) | 0 | port | 1 byte |

**Modem control** `wValue`: high byte is a write-enable mask, low byte the new state;
bit 0 = DTR, bit 1 = RTS. DTR on = `0x0101`, DTR off = `0x0100`, RTS on = `0x0202`, RTS off =
`0x0200`; both at once, e.g. `0x0303`.

**Line format** `wValue`:

| Bits | Meaning |
|------|---------|
| 0-7 | data bits (7 or 8) |
| 8-10 | parity: 0 none, 1 odd, 2 even, 3 mark, 4 space |
| 11-13 | stop bits: 0 = 1, 1 = 1.5, 2 = 2 |
| 14 | 1 = transmit break |

So 8N1 = `0x0008`, 8O1 = `0x0108`, 8E1 = `0x0208`, 8N2 = `0x1008`. Break on = the current
format value with bit 14 set; break off = without it.

**Baud-rate divisor.** The FT232B/R baud generator divides 3 MHz by `n + f`, where `n` is a
14-bit integer and `f` one of 0, 1/8, 2/8, ..., 7/8. The three fraction bits are not in
numeric order:

| f | 0 | 0.125 | 0.25 | 0.375 | 0.5 | 0.625 | 0.75 | 0.875 |
|---|---|-------|------|-------|-----|-------|------|-------|
| code (3 bits) | 0 | 3 | 2 | 4 | 1 | 5 | 6 | 7 |

Encoded divisor = `n | (code << 14)` (17 bits). Bits 0-15 go into `wValue`, bit 16 into
`wIndex` as stated above. Special cases: encoded divisor 0 selects 3 Mbaud and 1 selects
2 Mbaud. Algorithm: `d8 = round(24 000 000 / baud)` (the divisor in eighths), `n = d8 >> 3`,
`code = table[d8 & 7]`.

| Baud | wValue | wIndex bit | Actual rate |
|------|--------|-----------|-------------|
| 1200 | `0x09C4` | 0 | 1200 |
| 2400 | `0x04E2` | 0 | 2400 |
| 4800 | `0x0271` | 0 | 4800 |
| 9600 | `0x4138` | 0 | 9600 |
| 19200 | `0x809C` | 0 | 19200 |
| 38400 | `0xC04E` | 0 | 38400 |
| 57600 | `0xC034` | 0 | 57554 |
| 115200 | `0x001A` | 0 | 115385 |
| 230400 | `0x000D` | 0 | 230769 |
| 460800 | `0x4006` | 0 | 461538 |

The older FT8U232AM uses only two fraction bits; dive computer cables are not known to use
it.

**Receive status header.** Every bulk IN packet starts with two status bytes, then up to
`wMaxPacketSize − 2` data bytes. Packets are 64 bytes on full-speed chips. The host MUST
remove the two status bytes from **each** USB packet (a single bulk transfer of, say, 512
bytes contains several 64-byte packets, each with its own header). A transfer of exactly 2
bytes carries no data; the chip sends these periodically (every latency-timer period) when
idle.

| Byte | Bits |
|------|------|
| 0 (modem status) | bits 0-3 reserved (bits 0-3 are `0001` on some chips), 4 CTS, 5 DSR, 6 RI, 7 DCD |
| 1 (line status) | 0 data ready, 1 overrun, 2 parity error, 3 framing error, 4 break received, 5 TX holding empty, 6 TX empty, 7 RX FIFO error |

**Latency timer.** The chip forwards received data when 62 bytes are buffered or when the
latency timer (default 16 ms) expires. Protocols with many small round-trips SHOULD set it to
1-2 ms.

**Minimal open sequence:** reset (`00/0000`), set baud (`03`), set line format (`04`), set
flow control none (`02/0000`), set DTR/RTS (`01`) as the family requires, purge RX and TX,
optionally set latency timer (`09`).

Worked example, 9600 8N1, DTR on, RTS off:

```
H→D control  40 00 0000 0000 0000      reset
H→D control  40 03 4138 0000 0000      9600 baud
H→D control  40 04 0008 0000 0000      8 data bits, no parity, 1 stop bit
H→D control  40 02 0000 0000 0000      no flow control
H→D control  40 01 0101 0000 0000      DTR on
H→D control  40 01 0200 0000 0000      RTS off
H→D control  40 00 0001 0000 0000      purge RX
H→D control  40 00 0002 0000 0000      purge TX
```

(Setup fields shown as `bmRequestType bRequest wValue wIndex wLength`, values in hex.)

One reference implementation sends `wIndex = 0` for the baud request in all cases and never
sets the latency timer or flow control; this works for rates up to 460800 because bit 16 of
the divisor is 0 there.

**Confidence:** verified for the request codes, bit layouts and divisor encoding (FTDI
AN232B-05 and the public FTDI register definitions); likely for the open sequence.

### 2.2 Silicon Labs CP210x (VID `10C4`, PID `EA60` and others)

Interface 0 (each port of a multi-port CP210x is its own interface), one bulk IN and one bulk
OUT endpoint. Requests are vendor requests to the **interface**: `bmRequestType = 0x41`
(host to interface) or `0xC1` (interface to host), `wIndex` = interface number.

| Purpose | bRequest | wValue | Data |
|---------|----------|--------|------|
| Enable / disable UART | `0x00` | 1 = enable, 0 = disable | none |
| Set baud divisor (legacy) | `0x01` | 3 686 400 / baud | none |
| Set line control | `0x03` | see below | none |
| Get line control | `0x04` (`0xC1`) | 0 | 2 bytes |
| Set break | `0x05` | 1 = on, 0 = off | none |
| Set modem handshake (DTR/RTS) | `0x07` | see below | none |
| Get modem status | `0x08` (`0xC1`) | 0 | 1 byte |
| Get comm status | `0x10` (`0xC1`) | 0 | 19 bytes (errors, hold reasons, queue lengths) |
| Purge | `0x12` | bit mask, `0x000F` = everything | none |
| Set flow control | `0x13` | 0 | 16 bytes |
| Set baud rate | `0x1E` | 0 | 4 bytes, `u32le` baud |

**Line control** `wValue`: bits 0-3 stop bits (0 = 1, 1 = 1.5, 2 = 2); bits 4-7 parity
(0 none, 1 odd, 2 even, 3 mark, 4 space); bits 8-15 data bits (5..8). So 8N1 = `0x0800`,
8O1 = `0x0810`, 8E1 = `0x0820`, 8N2 = `0x0802`.

**Modem handshake** `wValue`: bit 0 DTR state, bit 1 RTS state, bit 8 write-enable for DTR,
bit 9 write-enable for RTS. DTR on = `0x0101`, DTR off = `0x0100`, RTS on = `0x0202`,
RTS off = `0x0200`. DTR/RTS can only be driven when flow control does not own them (the
power-on default is no flow control).

**Purge** mask: the four low bits clear the transmit and receive queues (bits 0 and 2 the
transmit side, bits 1 and 3 the receive side); `0x000F` clears both. On CP2102N a purge also
resets the port configuration, so it MUST be followed by re-sending baud and line control.

No status bytes are inserted into the bulk IN stream.

**Minimal open sequence:** enable UART, set baud rate (`0x1E`), set line control, set DTR/RTS,
purge.

Worked example, 57600 8N1:

```
H→D control  41 00 0001 0000 0000          enable
H→D control  41 1E 0000 0000 0004  00 E1 00 00   57600 baud (u32le)
H→D control  41 03 0800 0000 0000          8N1
H→D control  41 12 000F 0000 0000          purge all
```

Variant observed in one reference implementation: the same requests sent with
`bmRequestType = 0x40` (device recipient) and `wIndex = 0`. Single-interface CP210x chips
accept this; hosts SHOULD use `0x41` with the interface number.

**Confidence:** verified for request codes 0x03, 0x07, 0x12, the request type 0x41 and the
line-control layout (Silicon Labs AN571); likely for the remaining codes and bit masks (taken
from a public driver implementation).

### 2.3 Prolific PL2303 (VID `067B`, PID `2303`)

Interface 0 has an interrupt IN endpoint (modem status), one bulk IN and one bulk OUT. Line
coding and control lines use the **CDC-ACM class requests** (section 2.4) addressed to the
interface (`bmRequestType = 0x21`, `wIndex = 0`):

- `SET_LINE_CODING` (`0x20`) with the 7-byte CDC line-coding structure.
- `SET_CONTROL_LINE_STATE` (`0x22`): bit 0 DTR, bit 1 RTS.
- `SEND_BREAK` (`0x23`): `wValue = 0xFFFF` break on, `0x0000` break off.

Before the first line coding, PL2303 (pre-HXN types) are conventionally initialised with a
vendor sequence (`bmRequestType 0x40`/`0xC0`, `bRequest 0x01`): reads of registers `0x8484`,
`0x8383`, writes `0x0404 = 0`, `0x0404 = 1`, `0 = 1`, `1 = 0`, `2 = 0x44` (`0x24` for the
older type). One reference implementation that worked with dive computer cables sent **no**
vendor initialisation and **no** line coding at all, relying on the chip's power-on default;
that only works for 9600 8N1 devices.

**Confidence:** likely for the class requests; uncertain for the necessity of the vendor
initialisation sequence.

### 2.4 USB CDC-ACM (class `0x02` communications + class `0x0A` data)

The communications interface (class 2, subclass 2) carries class requests and an optional
interrupt notification endpoint; the data interface (class 10) carries one bulk IN and one
bulk OUT endpoint. Class requests use `bmRequestType = 0x21` (host to interface) or `0xA1`,
`wIndex` = communications interface number.

| Purpose | bRequest | wValue | Data |
|---------|----------|--------|------|
| SET_LINE_CODING | `0x20` | 0 | 7 bytes |
| GET_LINE_CODING | `0x21` (`0xA1`) | 0 | 7 bytes |
| SET_CONTROL_LINE_STATE | `0x22` | bit 0 DTR, bit 1 RTS | none |
| SEND_BREAK | `0x23` | duration ms; `0xFFFF` = until cleared; `0` = clear | none |

Line coding: `u32le dwDTERate`, `u8 bCharFormat` (0 = 1 stop bit, 1 = 1.5, 2 = 2),
`u8 bParityType` (0 none, 1 odd, 2 even, 3 mark, 4 space), `u8 bDataBits`.

```
115200 8N1:  00 C2 01 00  00  00  08
115200 8E1:  00 C2 01 00  00  02  08
```

CDC-ACM has no purge request; the host purges its own buffers and drains the bulk IN
endpoint. Many embedded CDC implementations ignore the baud rate entirely; others (dive
computer cradles with a real UART behind them) need it.

**Observed device specifics** (one source, not confirmed elsewhere):

- `FFFF:0005` (Mares cradle): the host reads the device and configuration descriptors,
  selects configuration 2, sets line state RTS-only (`0x0002`), waits 20 ms, clears it
  (`0x0000`), then sets line coding 256000 8N1. The data interface is found as the first
  class-10 interface.
- `2047:082B`: line coding 115200 8N1, no other set-up.

**Confidence:** verified for the class request definitions (USB CDC 1.2 / PSTN 1.2); uncertain
for the device-specific sequences.

### 2.5 Third-party multi-mode adapters

Some aftermarket USB-to-IR adapters enumerate with the VID:PID of a common bridge chip
(`10C4:EA60`, `066F:4210`, `0403:87D0`, `0403:6001`) but answer FTDI vendor requests and carry
a vendor-specific product string. They are not part of any dive computer's protocol and are
out of scope; hosts that want to support them need the adapter vendor's documentation.
When a `10C4:EA60` device does not answer CP210x requests, trying FTDI requests is a
reasonable fallback.

**Confidence:** uncertain.

## 3. Driver selection

Where a family is reachable through several cable generations, hosts SHOULD try the bridge
types in a fixed order and keep the first that opens and accepts line configuration. One
reference implementation tries CP210x, then FTDI, then PL2303.

## Open questions

- Mares Icon HD family over the `FFFF:0005` CDC cradle: 256000 8N1 or 115200 8E1? Possibly
  different cradle generations; needs a traffic capture.
- Whether PL2303-based cables work without the vendor initialisation sequence on all chip
  revisions.
- Exact purge-mask semantics on single-port CP210x parts (AN571 wording is ambiguous; the
  all-bits value is safe).

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- FTDI, AN232B-05 "Configuring FT232R, FT2232 and FT232B Baud Rates", https://ftdichip.com/Documents/AppNotes/AN232B-05_BaudRates.pdf
- FTDI FT232R datasheet, https://ftdichip.com/products/ft232rl/
- Silicon Labs, AN571 "CP210x Virtual COM Port Interface", https://www.silabs.com/documents/public/application-notes/AN571.pdf
- Linux kernel USB-serial drivers (public register definitions for FTDI, CP210x, PL2303), https://github.com/torvalds/linux/tree/master/drivers/usb/serial
- USB Class Definitions for Communications Devices 1.2 and PSTN subclass 1.2, https://www.usb.org/document-library/class-definitions-communication-devices-12
