# Bluetooth Low Energy transport

Almost every current dive computer offers a BLE link. Nearly all of them use the same
pattern, called **GATT serial** here: one vendor service with a characteristic the host
writes commands to and a characteristic the device notifies data on. This document defines
that pattern, the rules for turning it into a byte stream, the vendor-specific deviations,
and a single table of every known service/characteristic UUID and advertised-name pattern.

UUIDs are written in lowercase 128-bit form. 16-bit SIG UUIDs `xxxx` expand to
`0000xxxx-0000-1000-8000-00805f9b34fb`.

## 1. GATT serial model

### 1.1 Roles

| Role | Properties | Direction |
|------|------------|-----------|
| service | vendor 128-bit UUID | — |
| write characteristic | `write` and/or `write without response` | H→D |
| notify characteristic | `notify` or `indicate` | D→H |
| optional auxiliary characteristics | read / write / notify | vendor-specific |

The write and notify roles can be the same characteristic (Shearwater). When a family
document gives no UUIDs, a host MAY locate the roles generically: in the vendor service, the
characteristic with a write property is the write role and the one with notify/indicate is
the notify role.

### 1.2 Connection sequence

1. Scan, filter by advertised name (table in section 3) and, where the device advertises it,
   by service UUID.
2. Connect. If the device requires bonding, let the operating system pair (Just Works or
   passkey; no dive computer in the sources uses a fixed BLE passkey known to the host).
3. Discover services.
4. Enable notifications on the notify characteristic by writing the Client Characteristic
   Configuration descriptor (`2902`): `01 00` for notify, `02 00` for indicate. Use indicate
   only if the characteristic lacks the notify property.
5. Optionally request a larger ATT MTU (section 1.4).
6. Run the family protocol.

One source waits about 1 s after the link is up before service discovery, and waits for
bonding to complete if the OS started it. One source reads Device Information (`180a`) Model
Number (`2a24`) before declaring the link ready; the Uwatec family waits for the device name
before its first command.

**Confidence:** likely.

### 1.3 Writing

- A host-to-device message is split into chunks of at most `ATT_MTU − 3` bytes (20 bytes with
  the default MTU of 23). Each chunk is one ATT write.
- With **write with response**, the next chunk MUST NOT be sent before the previous write is
  acknowledged. With **write without response**, the host SHOULD still pace writes (wait for
  the stack's flow-control callback) to avoid silent drops.
- Unless a family states otherwise, use write with response if the characteristic supports
  it.
- Chunk boundaries carry no meaning for most families: the device reassembles the byte
  stream. Exceptions (families that put a header in each chunk) are listed in section 2.

Chunk sizes used by one reference implementation's generic packetiser:

| Families | Max write chunk | Read cache |
|----------|-----------------|------------|
| Mares Icon HD (BLE), HW OSTC 3/4/5 | 20 | 244 |
| Ratio/DiveSystem iDive, McLean Extreme, Seac Screen | 244 | 244 |
| Divesoft Freedom (HDLC framing) | 244 | 244 |
| Suunto EON Steel (HDLC framing) | 20 | 20 |

244 is the ATT payload that fits a 247-byte MTU (the largest MTU that fits one
LE-data-length-extension packet). Hosts that cannot control the MTU SHOULD use 20.

**Confidence:** likely.

### 1.4 MTU

Larger MTUs are an optimisation for almost every family. One source requests MTU 63 (60-byte
payload) for the original Shearwater Petrel 2 profile, 80 (77) for newer Shearwater models,
263 for one Pelagic profile and 160 for a non-dive device; all others keep the default.
Shearwater is the one family where the MTU matters to the protocol, because the device
fills notifications to `MTU − 3` and the host strips a per-notification header (section 2).

**Confidence:** likely.

### 1.5 Reading

Every notification's value is appended to a receive FIFO in arrival order; the family
protocol reads bytes from the FIFO with its usual timeouts. Purge clears the FIFO. Hosts MUST
NOT assume one notification equals one protocol message, except where section 2 says so.

Notifications may arrive from more than one characteristic in the vendor service (Cressi,
Tusa); a host SHOULD subscribe to every notify characteristic the family document names and
tag the data by source if the family needs it.

**Confidence:** likely.

## 2. Framing layers above GATT

| Family | Per-notification / per-write structure |
|--------|------------------------------------------|
| Most (Mares, Pelagic i330R, Ratio, Cressi, HW OSTC, Halcyon, Seac, ...) | none; plain byte stream |
| Suunto EON Steel family | HDLC-style framing over the stream: frame = `7E` + escaped message + `7E`; inside a frame `7D` and `7E` are sent as `7D, b XOR 0x20`. The message is the family's 12-byte header, payload and a 4-byte `crc-32-iso-hdlc` (little-endian). A frame may span many notifications. |
| Divesoft Freedom / Liberty | Same HDLC-style escaping (`7E` delimiters, `7D` escape, XOR `0x20`); frame contents in the family document (`crc-16-ibm-sdlc`). |
| Uwatec Smart (G2, Aladin Sport/H Matrix, A1/A2, Luna 2.0) | H→D: `[n][command bytes]`, not padded. D→H: each notification begins with one byte that is a length on single-packet replies; on multi-packet replies (newer firmware) it behaves like a counter that grows by 19 per notification (observed sequence `F7 14 27 3A 4D 60 73 86 99 AC BF D2 E5 F7 …`) and is **not** a length. Hosts take `notification_length − 1` payload bytes from every notification and ignore byte 0. |
| Shearwater | Each SLIP chunk (at most 32 bytes) is written as 20-byte writes `[count][index][≤18 data]`; each notification is `[count][index][data]`, where `count` is the number of notifications in the message and `index` runs from 0 (H→D) or 1 (D→H, as observed). The SLIP stream inside is described in the Shearwater family document. |
| Pelagic legacy BLE (Atom 2 protocol over BLE) | H→D: `CD 40 seq len payload`. D→H: 20-byte notifications, each a 4-byte header followed by 16 data bytes. |
| Pelagic i330R / Apeks DSX | `CD flags cmd crc len payload` packets with a `crc-8-msb-0x12` checksum; see the Pelagic family document. |
| Garmin Descent Mk1 | COBS-encoded GFDI messages delimited by `00`, `crc-16-arc` per message. |

Worked example, HDLC-style escaping of message bytes `01 7E 02 7D`:

```
7E 01 7D 5E 02 7D 5D 7E
```

**Confidence:** likely (Uwatec counter behaviour and Shearwater index base are uncertain).

## 3. Consolidated table of vendor BLE profiles

"Names" gives the advertised local name filter. Matching is case-insensitive. **prefix** =
name starts with the string; **exact** = equal; **prefix+digits** = prefix followed only by
decimal digits; **prefix+hex** = prefix followed only by hexadecimal digits. Where the two
sources use different match rules both are given. "Src": A = open-source reference library,
B = observed application behaviour, P = public vendor/SIG document.

| Vendor / family | Service | Write (H→D) | Notify (D→H) | Other characteristics | Names | Notes | Src | Conf. |
|-----------------|---------|-------------|--------------|-----------------------|-------|-------|-----|-------|
| Mares (Icon HD family: Puck Pro/+ BLE, Smart, Genius, Quad Air, Smart Air, Quad, Puck Air 2, Sirius, Quad Ci, Quad 2, Sirius L, Puck 4/Lite/Pro EZ/Pro Ultra; Bluelink Pro adapter) | `544e326b-5b72-c6b0-1c46-41c1bc448118` | `99a91ebd-b21f-1689-bb43-681f1f55e966` | `1d1aae28-d2a8-91a1-1242-9d2973fbe571` | — | `Mares bluelink pro`, `Mares Genius`, `Sirius`, `Sirius L`, `Quad Ci`, `Quad2`, `Puck4`, `Puck Lite`, `Puck`, `Puck Pro U` (prefix) | 20-byte writes with response. One source removes an existing OS bond before connecting to `Mares bluelink pro` / `Mares Genius`. | A names, B UUIDs | likely |
| Heinrichs Weikamp (OSTC 2/2 TR/3/cR/Plus/Sport/Nano/4/5) | `0000fefb-0000-1000-8000-00805f9b34fb` (Telit/Stollmann Terminal I/O) | `00000001-0000-1000-8000-008025000000` (UART data RX) | `00000002-0000-1000-8000-008025000000` (UART data TX) | `00000003-0000-1000-8000-008025000000` (UART credits RX), `00000004-0000-1000-8000-008025000000` (UART credits TX) | `OSTC`, `FROG` (prefix); also seen: `OSTCs`, `OSTC4`, `BM+SR` | The Terminal I/O profile defines credit-based flow control on the two credit characteristics; one source works without using them. | A names, B UUIDs 1/2, P credits | likely; credits uncertain |
| Shearwater (Petrel 2, Perdix, Perdix AI, Nerd 2, Teric, Peregrine, Petrel 3, Perdix 2, Tern, Tern TX, Peregrine TX, Perdix 3) | `fe25c237-0ece-443c-b0aa-e02033e7029d` | `27b7570b-359e-45a3-91bb-cf7e70049bd2` | same characteristic | — | `Predator`, `Petrel`, `Petrel 3`, `NERD`, `NERD 2`, `Perdix`, `Perdix 2`, `Perdix 3`, `Teric`, `Peregrine`, `Peregrine TX`, `Tern` (exact in A; prefix in B) | Per-chunk header (section 2). MTU 63 or 80 requested in B; write without response for the first profile only. | A names, B UUIDs | likely |
| Scubapro / Uwatec Smart (G2, G2 TEK, G2 Console, G2 HUD, G3, Aladin Sport Matrix, Aladin H Matrix, A1, A2, Luna 2.0, Luna 2.0 AI) | `fdcdeaaa-295d-470e-bf15-04217b7aa0a0` | `a188b7dd-debb-449a-852d-c243d46b4b1a` | `aa0c68f0-ea9c-493d-8112-62879e72af68` | GAP Peripheral Preferred Connection Parameters `2a04` (written once by B, value `06 00 06 00 00 00 0a 00` = 7.5 ms interval, latency 0, timeout 100 ms) | A exact: `G2`, `Aladin`, `HUD`, `A1`, `A2`, `G2 TEK`, `Galileo 3`, `Luna 2.0 AI`, `Luna 2.0`; B prefix: `Aladin`, `Aladin H`, `G2`, `Galileo 3`, `A1`, `A2`, `Luna`, `HUD` | Requires bonding; B only connects to already-bonded devices. | A names, B UUIDs | likely |
| Pelagic Atom 2 protocol over BLE (Oceanic Pro Plus X, Geo 4.0, Veo 4.0, Pro Plus 4, Geo Air; Aqualung i200C, i300C, i470TC, i550C, i750TC, i770R; Sherwood Sage, Wisdom 4, Beacon) | `cb3c4555-d670-4670-bc20-b61dbc851e9a` | `6606ab42-89d5-4a00-a8ce-4eb5e1414ee0` | `a60b8e5c-b267-44d7-9764-837caf96489e` | — | prefix+digits, where the prefix is the model code as two ASCII characters: `ER` (0x4552), `EZ` (0x455A), `FG` (0x4647), `FH` (0x4648), `FI` (0x4649), `FQ` (0x4651), `FR` (0x4652), `FS` (0x4653), `FT` (0x4654), `FU` (0x4655), `FV` (0x4656), `GB` (0x4742), `GC` (0x4743), `GI` (0x4749), `GK` (0x474B) | B lists `ER EZ FH FI FQ FR FS FT FU FV GC GI` as prefixes. Example name `FH123456`. | A names, B UUIDs | likely |
| Pelagic, Oceanic VTX | `da2b84f1-6279-48de-bdc0-afbea0226079` | `bf03260c-7205-4c25-af43-93b1c299d159` | `18cda784-4bd3-4370-85bb-bfed91ec86af` | — | `EW` (prefix) | Model code 0x4557. | B | uncertain |
| Pelagic i330R protocol (Aqualung i330R, i330R Console, Apeks DSX) | `ca7b0001-f785-4c38-b599-c7c5fbadb034` | `ca7b0003-f785-4c38-b599-c7c5fbadb034` | `ca7b0002-f785-4c38-b599-c7c5fbadb034` | — | prefix+digits `GA` (0x4741), `GD` (0x4744), `GM` (0x474D) | Pairing PIN shown on the device; access code stored by the host (family document). B also has an otherwise unused profile on the same UUIDs with MTU 263. | A names, B UUIDs | likely |
| Suunto EON Steel family (EON Steel, EON Core, D5, EON Steel Black) | `98ae7120-e62e-11e3-badd-0002a5d5c51b` | `c6339440-e62e-11e3-a5b3-0002a5d5c51b` | `d0fd6b80-e62e-11e3-a2e9-0002a5d5c51b` | — | `EON Steel`, `EON Core`, `Suunto D5`, `EON Steel Black` (prefix); B: `EON`, `Suunto D5` | HDLC framing (section 2). Bonding required. | A names, B UUIDs | likely |
| Suunto Spartan (not a dive computer family in A) | `61353090-8231-49cc-b57a-886370740041` | `17816557-5652-417f-909f-3aee61e5fa85` | `34802252-7185-4d5d-b431-630e7050e8f0` | — | `Spartan` (prefix) | Defined in B but not reachable from its model list. Service UUID advertised. | B | uncertain |
| Garmin Descent Mk1 | `6a4e2401-667b-11e3-949a-0800200c9a66` | `6a4e4c80-667b-11e3-949a-0800200c9a66` | `6a4ecd28-667b-11e3-949a-0800200c9a66` | — | `Descent` (prefix) | GFDI protocol; bonding required. | B | likely |
| Garmin Descent Mk2 | `6a4e8022-667b-11e3-949a-0800200c9a66` | `6a4e4c80-667b-11e3-949a-0800200c9a66` | `6a4ecd28-667b-11e3-949a-0800200c9a66` | — | `Descent Mk2` (prefix) | Defined but not used in B; Mk2 logs are normally read over MTP. | B | uncertain |
| Cressi (Cartesio, Goa, Leonardo 2.0, Donatello, Michelangelo, Raffaello, Neon, Nepto) | `6e400001-b5a3-f393-e0a9-e50e24dc10b8` (Nordic UART service UUID) | B: the service UUID itself is used as write role | B: `6e400002-b5a3-f393-e0a9-e50e24dc10b8` | A: `6e400003-…` (read 5 bytes), `6e400004-…` (read 2 bytes), `6e400005-…` (read 2 bytes) give the version information over BLE; B also accepts notifications from `6e400003` and `6e400004` | A: prefix `<model in hex>_` followed by hex digits, e.g. `1_…` Cartesio, `9_…` Neon, `a_…` Nepto. B: regular expression `[0-9]{1,2}_[0-9]+` | The roles differ from the standard Nordic UART layout (where `6e400002` is write and `6e400003` is notify). | A, B | uncertain (role assignment) |
| Tusa DC Solar Link, Scubapro Z1 | `c4acde54-23a2-4008-99ae-8c4297647a12` | `c4acde54-23a2-4008-99ae-8c4297647a14` | `c4acde54-23a2-4008-99ae-8c4297647a13` | `c4acde54-23a2-4008-99ae-8c4297647a15` (written without response; also notifies) | `IQ1204`, `SPROXX`, `Z1` (prefix) | | B | likely |
| Ratio / DiveSystem (iX3M 2021, iDive 2, iX3M 2, ATOM) | `49535343-fe7d-4ae5-8fa9-9fafd205e455` (Microchip transparent UART) | `49535343-8841-43f4-a8d4-ecbe34729bb3` | `49535343-1e4d-4bd9-ba61-23c647249616` | — | `DS`, `IX5M`, `RATIO-` (prefix+digits in A; prefix `IX5M`, `RATIO-` in B) | | A names, B UUIDs | likely |
| Crest (`Crest-GA`), TI CC254x serial profile | `f000ffe0-…` (TI) | one of `…ffe1`/`…ffe2`/`…ffe3` | one of `…ffe1`/`…ffe2`/`…ffe3` | — | `Crest-GA` | B defines all six write/notify permutations and reaches none; full 128-bit base not confirmed. | B | uncertain |
| McLean Extreme | not documented | | | | `McLean Extreme` (exact) | Also on BT classic. | A | uncertain |
| Deep Six Excursion, Crest CR-4, Genesis Centauri, Scorpena Alpha | not documented | | | | `EXCURSION`, `Crest-CR4`, `CENTAURI`, `ALPHA` (exact) | | A | uncertain |
| Deepblu Cosmiq+ | not documented | | | | `COSMIQ` (exact) | | A | uncertain |
| Oceans S1 | not documented | | | | `S1` (prefix) | | A | uncertain |
| Divesoft Freedom, Liberty | not documented | | | | `Freedom`, `Liberty` (prefix) | HDLC framing. | A | uncertain |
| Halcyon Symbios (HUD = model 1, Handset = model 7) | not documented | | | | `H01…`, `H07…` (first three characters), or a name of at least 10 decimal digits whose characters 4-5 are the two-digit model | | A | uncertain |
| Seac Screen / Tablet | not documented | | | | `Tablet` (prefix+digits) | | A | uncertain |

Other UUIDs seen on these links:

| UUID | Meaning | Use |
|------|---------|-----|
| `1800` / `2a00` | GAP service / Device Name | read by one source for a non-dive device |
| `1800` / `2a04` | GAP Peripheral Preferred Connection Parameters | written for Scubapro (above) |
| `180a` / `2a24` | Device Information / Model Number String | read before the first command by one source |
| `180a` / `2a29` | Device Information / Manufacturer Name | declared, unused |
| `2902` | Client Characteristic Configuration | notification enable |
| `a87988b9-694c-479c-900e-95dfa6c00a24` | unknown vendor characteristic | if present in the selected service, one source writes `01` to it once before the first data write and waits 100 ms; owner unknown |

**Confidence:** per row as marked.

## 4. Web Bluetooth notes

- Every service UUID the page will touch MUST be listed in `filters[].services` or
  `optionalServices` at `requestDevice` time. Name filters use `name` (exact) or
  `namePrefix`; "prefix+digits" and the Halcyon rule need `namePrefix` plus a check after
  selection.
- There is no API to request an MTU or read the negotiated MTU. Chromium negotiates the
  largest MTU the platform allows. Writes with response larger than `MTU − 3` are carried out
  as long writes (up to 512 bytes) where the device supports them; writes without response
  larger than `MTU − 3` fail. Hosts SHOULD therefore chunk to 20 bytes unless the family
  needs otherwise, and Shearwater-style protocols MUST derive the notification size from the
  first full notification received.
- Use `writeValueWithResponse` / `writeValueWithoutResponse` explicitly, and await each call
  before the next chunk.
- Bond removal and connection-parameter writes are not available to pages; characteristics in
  the GAP service and some Device Information characteristics may be hidden by the Web
  Bluetooth GATT blocklist. The Scubapro connection-parameter write is an optimisation and can
  be skipped.
- Pairing happens through the OS dialog the first time an encrypted characteristic is
  accessed.

**Confidence:** likely.

## Open questions

- Cressi BLE: which characteristic is the write role and which notify (Nordic UART layout or
  the renumbered layout seen in one source)?
- HW OSTC: are Terminal I/O credits required by any firmware?
- Uwatec multi-packet first-byte semantics (counter vs. length).
- Service UUIDs for McLean, Deep Six, Deepblu, Oceans, Divesoft, Halcyon and Seac.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Bluetooth Core Specification 5.x, Vol 3 Part F (ATT) and Part G (GATT), https://www.bluetooth.com/specifications/specs/
- Bluetooth SIG Assigned Numbers, https://www.bluetooth.com/specifications/assigned-numbers/
- Nordic Semiconductor, Nordic UART Service, https://docs.nordicsemi.com/bundle/ncs-latest/page/nrf/libraries/bluetooth/services/nus.html
- Telit (Stollmann) Terminal I/O profile documentation, https://www.telit.com/
- Microchip RN4870/4871 transparent UART service, https://www.microchip.com/
- Web Bluetooth specification and GATT blocklist, https://webbluetoothcg.github.io/web-bluetooth/
