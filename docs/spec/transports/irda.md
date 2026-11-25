# IrDA transport

The Uwatec/Scubapro "Smart" generation (Smart Pro, Smart Com, Smart Tec, Smart Z, Galileo
Sol/Luna/Terra/Trimix, Aladin Tec/Tec 2G/Tec 3G/Prime/One/2G/Sport, Subgear XP-10/XP-3G/XP Air)
talks through a full IrDA stack: SIR physical layer, IrLAP, IrLMP and TinyTP. On desktop
systems with an OS IrDA stack the host simply opens an IrDA stream socket; elsewhere (WebUSB,
Android, modern OSes that dropped IrDA) the host has to implement the stack itself and drive a
USB IrDA dongle. This document gives what a minimal primary-side stack needs. Other
"infrared" cables (Mares, Cressi, Suunto, Liquivision) are plain serial over IR and use
[serial.md](serial.md).

## 1. Device-side facts

| Item | Value |
|------|-------|
| Discovery nicknames | `Aladin Smart Com`, `Aladin Smart Pro`, `Aladin Smart Tec`, `Aladin Smart Z`, `Uwatec Aladin`, `UWATEC Galileo`, `UWATEC Galileo Sol` (exact match) |
| Service | IrLMP LSAP selector **1**, reached directly ("LSAP-SEL1"); no IAS query is performed by either source |
| Transport above IrLMP | TinyTP (stream socket semantics) |
| Speeds | starts at 9600 bit/s; switches to the highest speed both sides offer at connect |

**Confidence:** likely (both sources connect to LSAP 1 without IAS; TinyTP is implied by the
stream socket in one source and by the observed one-byte PDU header in the other).

## 2. SIR framing (physical / framing layer)

Asynchronous 8N1 bytes at the link speed. A frame is:

```
[FF × XBOFs]  C0  <escaped IrLAP frame + FCS>  C1
```

- `C0` BOF, `C1` EOF, `7D` control escape. Inside the frame, any byte equal to `C0`, `C1` or
  `7D` is sent as `7D, byte XOR 0x20`.
- Extra BOFs (`FF`) are sent before the BOF so slow receivers can sync; 11 is used at 9600
  bit/s before connection, the negotiated count afterwards.
- FCS: [`crc-16-ibm-sdlc`](../common/checksums.md#crc-16-ibm-sdlc) over the IrLAP frame
  (address, control, information), transmitted **least significant byte first** and escaped
  like data. A receiver can run the same CRC over frame + FCS and compare the raw register
  (before final XOR) with the residue `0xF0B8`.

Worked example, RR frame for connection address `0x2A` (address byte `0x55`, control
`0x11`):

```
frame  55 11
FCS    0xA300 -> sent 00 A3
SIR    C0 55 11 00 A3 C1      (preceded by extra BOFs)
```

**Confidence:** verified for the CRC and escape rules against the computed example; likely
for the remaining framing details (IrDA SIR is a public standard; recalled, not re-read).

## 3. IrLAP

### 3.1 Frame layout

```
[A] [C] [I...]
A = (connection_address << 1) | C/R     C/R = 1 on frames from the primary (host)
C = control field
```

Connection address `0x7F` (A = `0xFF`) is the broadcast address used during discovery and
connect. A host picks a random connection address in 1..126 for the connection.

Control field values used (P/F bit = `0x10`):

| Frame | Control | Notes |
|-------|---------|-------|
| XID command | `0x3F` (`0x2F` + P) | discovery |
| XID response | `0xBF` (`0xAF` + F) | discovery reply |
| SNRM command | `0x93` | connect |
| UA response | `0x73` | connect / disconnect accept |
| DISC command | `0x53` | disconnect |
| RD response | `0x53` | request disconnect (secondary) |
| DM response | `0x1F` | disconnected mode |
| RR | `(Nr << 5) \| 0x11` | receiver ready, with P/F |
| RNR | `(Nr << 5) \| 0x15` | receiver not ready |
| REJ | `(Nr << 5) \| 0x19` | reject |
| I-frame | `(Nr << 5) \| P<<4 \| (Ns << 1)` | data, `Ns`/`Nr` modulo 8 |

When matching received frames, ignore the P/F bit (`control & 0xEF`).

**Confidence:** likely.

### 3.2 Discovery (XID exchange)

The host (primary) sends one XID command per slot:

```
FF 3F  01  <src:4>  FF FF FF FF  <flags>  <slot>  00  [<discovery info>]
         ^format id   ^dest = broadcast       ^version
```

- `src` is a random 32-bit device address chosen by the host.
- `flags` bits 0-1: number of slots (0 = 1, 1 = 6, 2 = 8, 3 = 16); bit 2: "generate new
  device address" (used when two devices answered with the same address).
- `slot` counts 0 .. N−1, one frame per slot, with a slot timer of about 100 ms (60-90 ms in
  the standard) between frames.
- After the last slot the host sends a final XID with slot `FF` that carries its own discovery
  info.
- Discovery info = hint bytes (at least 2; bit 7 of each hint byte means "another hint byte
  follows"), a character-set byte (`00` = ASCII), and the nickname. One source sends
  `82 20 00` followed by its application name.

The device answers in a randomly chosen slot with an XID response (`FE BF ...` or with its own
address byte) carrying its own 32-bit address in the source field, the host address in the
destination field, and its discovery info (nickname, see section 1). Hosts SHOULD retry
discovery several times: one source repeats up to 5 times with 4 s pauses, another up to 4
extra times.

Device addresses are opaque 4-byte strings: copy them verbatim from the XID response into the
SNRM destination field. (One source writes its own address most-significant byte first; the
IrLAP standard defines little-endian order. Because the value is random this does not matter
as long as received addresses are copied, not re-encoded.)

Worked example (6 slots, slot 0, host address bytes `12 34 56 78`):

```
C0 FF 3F 01 12 34 56 78 FF FF FF FF 01 00 00 B3 CA C1
```

**Confidence:** likely.

### 3.3 Connect (SNRM / UA) and QoS negotiation

```
H→D  FF 93 <src:4> <dst:4> <conn_addr << 1> <QoS parameters>
D→H  <conn_addr << 1> 73 <src:4> <dst:4> <QoS parameters>
```

QoS parameters are type-length-value triples. Values sent by one source:

| PI | PL | PV | Meaning |
|----|----|----|---------|
| `01` | 2 | `3E 00` | baud rates: bit 0 2400, bit 1 9600, bit 2 19200, bit 3 38400, bit 4 57600, bit 5 115200 (bits 6-8: 576k, 1.152M, 4M) → 9600..115200 |
| `82` | 1 | `01` | maximum turnaround time 500 ms |
| `83` | 1 | `07` | data size: bit mask, `07` = 64/128/256 bytes |
| `84` | 1 | `01` | window size 1 |
| `85` | 1 | `80` | additional BOFs at 115200: `80` = 0 |
| `86` | 1 | `01` | minimum turnaround time 10 ms |
| `08` | 1 | `07` | link disconnect/threshold: 3, 8 or 12 s |

The negotiated baud rate is the highest bit set in the AND of both baud masks; the link
disconnect time likewise. After receiving UA the host switches the dongle to the negotiated
speed and sends the first RR (`(conn_addr<<1)|1, 0x11`). From then on the host is primary:
it polls with P set, the device answers with F set, window size 1, sequence numbers modulo 8.

Full SNRM as sent by one source (before SIR wrapping):

```
FF 93 <src:4> <dst:4> <K<<1>  01 02 3E 00  82 01 01  83 01 07  84 01 01  85 01 80  86 01 01  08 01 07
```

Disconnect: host sends DISC (`(K<<1)|1, 53`), device answers UA; receiving DISC, RD or DM from
the device also ends the link. Afterwards return to 9600 bit/s with 11 extra BOFs.

**Confidence:** likely.

## 4. IrLMP

Every IrLAP I-frame carries one LM-PDU:

```
[D] [S] [payload...]
D = destination LSAP selector (bit 7 = 1 for control PDUs)
S = source LSAP selector
```

Control PDUs carry an opcode in the third byte (bit 7 = confirm):

| Opcode | Meaning |
|--------|---------|
| `01` | connect request; byte 4 reserved `00`; user data follows |
| `81` | connect confirm |
| `02` | disconnect; byte 4 = reason |
| `03` / `83` | access mode request / confirm |

To reach the dive computer: `[0x81][host LSAP][01][00][TinyTP connect parameters]` (destination
LSAP 1 with the control bit; the host chooses any free local LSAP ≥ 2). On connect confirm
(`81`) data transfer can start.

IAS (LSAP 0, class/attribute lookup with `GetValueByClass`) is not needed for these devices.
A host that implements a minimal IAS server for the device's benefit answers class `Device`,
attribute `DeviceName` with its nickname and `IrLMPSupport` with `01 00 00`; neither source
saw the device query it.

**Confidence:** likely.

## 5. TinyTP

TinyTP adds one header byte to every LM data PDU:

```
bit 7      M (more: SAR, 0 when unsegmented)
bits 0-6   credit granted to the peer (number of PDUs it may send)
```

The connect request/confirm user data starts with the same byte (initial credit), optionally
followed by parameters when bit 7 is used as the parameter flag.

Observed usage with the Uwatec Smart devices:

- Every host data PDU is `00 <command bytes>` (credit 0, no SAR).
- The host strips the first byte of every received PDU.
- During the bulk data phase the host sends a data-less PDU `02` (grant 2 credits) after every
  2 received data PDUs. Hosts SHOULD grant credits generously enough that the device never
  stalls.
- No segmentation/reassembly is used; each device PDU fits the negotiated data size.

**Confidence:** likely.

## 6. USB IrDA dongles

### 6.1 USB IrDA Bridge class (class `0xFE`, subclass `0x02`)

Examples: SigmaTel STIr4210/4220 (`066F:4210`), `066F:4310`, `0E55:110A`.

- Class descriptor (type `0x21`, 12 bytes) read with `bmRequestType 0xA1`, `bRequest 0x06`
  (Get Class-Specific Descriptor), `wValue 0`, `wIndex` = interface, `wLength 12`. It lists
  supported speeds (bit 0 2400, bit 1 9600, bit 2 19200, bit 3 38400, bit 4 57600, bit 5
  115200, ...) and extra-BOF needs.
- Bulk OUT: each frame is `[header][IrLAP frame without FCS]`; the dongle adds BOF/EOF,
  escaping and FCS.
- Outbound header byte: bits 0-3 speed change (0 none, 1 2400, 2 9600, 3 19200, 4 38400,
  5 57600, 6 115200, 7 576k, 8 1.152M, 9 4M); bits 4-7 extra BOFs (0 no change, 1 = 48,
  2 = 24, 3 = 12, 4 = 6, 5 = 3, 6 = 2, 7 = 1, 8 = 0). The change applies after the frame is
  sent. A header-only transfer changes settings without sending a frame.
- Bulk IN: `[header][IrLAP frame]`, FCS already checked and removed.
- STIr421x parts use a **3-byte** header instead of 1 (first byte as above, then two more
  bytes). One source sends `32 00 00` on the first frame (12 BOFs, 9600), `00 00 00`
  afterwards, sets byte 2 to `04` at 115200 (meaning unknown), changes speed by sending
  `<code> 00 00` then `<0x30|code> 00 00` (`<0x70|code>` at 115200, i.e. 1 extra BOF), and drops
  the first 3 bytes of every received transfer.

**Confidence:** likely for the class definition; uncertain for the STIr421x 3-byte header
details.

### 6.2 SigmaTel STIr4200 and compatibles (`066F:4200`, `0DF8:0001`, `1685:0200`)

Raw SIR dongle: the host does all SIR wrapping (section 2) and FCS.

- Register write: `bmRequestType 0x40, bRequest 0x03, wValue = value, wIndex = register`.
- Register read: `bmRequestType 0xC0, bRequest 0x01, wIndex = register`; after a transmit
  the host polls register 5 (3-byte read) until bit `0x10` of the first byte is clear (TX
  FIFO drained).
- Baud: register 2 = `1 152 000 / baud − 1` (9600 → `0x77`, 115200 → `0x09`).
- Init sequence (register = value): `3=01, 3=00, 4=07, 3=58, 2=77, 3=40, 1=22, 3=C0, 3=40,
  3=06`. For `1685:0200`: `8=15, 4=02, 2=77, 1=2A, 3=80, 3=00, 4=07, 3=40`.
- Bulk OUT frame: `55 AA <len:u16le>` followed by `len` bytes of SIR data (extra BOFs, `C0`,
  escaped frame and FCS, `C1`).
- Bulk IN: raw SIR bytes; the host unwraps and checks the FCS residue.

**Confidence:** uncertain (one source; register meanings not documented publicly).

### 6.3 MosChip MCS7780 (`9710:7780`)

Raw SIR dongle with a different register interface.

- Register write: `bmRequestType 0x40, bRequest 0x0E, wValue = value, wIndex = register`.
- Register read: `bmRequestType 0xC0, bRequest 0x0F, wIndex = register`, 2 bytes; TX busy =
  register 7 bit 0.
- Init (register = value): `0=D722, 4=0000, 0=D620, 2=0905, 2=0904, 2=0900, 2=0908, 0=5620`.
- Speed: register 0 = `(0x5620 & ~0x00E0) | code` with code `00` 2400, `20` 9600, `40` 19200,
  `60` 38400, `80` 57600, `A0` 115200, `C0` 576000, `E0` 1152000; written first with bit
  `0x8000` set, then without.
- Bulk OUT frame: `<total length:u16le>` (including these 2 bytes), then `C0` × 2, escaped
  frame + FCS, `C1`.
- Bulk IN: un-escape `7D`; a frame is complete when the FCS residue matches.

**Confidence:** uncertain (one source).

## 7. Web notes

No browser exposes OS IrDA sockets. With WebUSB a page can claim a bridge-class or raw SIR
dongle (neither class is protected) and run the stack of sections 2-5 in software. The
minimal stack needed is small: discovery, SNRM/UA, RR/I-frame exchange with window 1,
IrLMP connect to LSAP 1, TinyTP credits.

## Open questions

- Byte order of IrLAP device addresses on the wire as produced by the dive computers.
- Meaning of the third STIr421x header byte (`04` at 115200).
- Whether any Uwatec device ever answers with more than one I-frame per poll (window > 1).

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Infrared Data Association: Serial Infrared Physical Layer (SIR), IrLAP 1.1, IrLMP 1.1, Tiny TP 1.1 specifications, https://www.irda.org/ (archived)
- USB IrDA Bridge Device Definition 1.0, https://www.usb.org/sites/default/files/usbir10.pdf
- Linux kernel USB IrDA definitions (public), https://github.com/torvalds/linux/blob/master/include/linux/usb/irda.h
