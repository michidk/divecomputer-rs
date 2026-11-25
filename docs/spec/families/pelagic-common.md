# Pelagic Pressure Systems: shared concepts

Pelagic Pressure Systems builds dive computers that are sold under many brands: Oceanic, Aeris,
Aqualung, Sherwood, Hollis, Genesis, Tusa, Subgear, Seemann, Beuchat and Apeks. Four download
protocols exist. They differ on the wire but share one memory model: a fixed device-info page, a
pointer page, a logbook ring buffer of fixed-size entries and a profile ring buffer of
variable-length dive records. This document describes what the protocols share. The protocol
documents are:

| Document | Protocol generation | Transports | Typical devices |
|----------|---------------------|------------|-----------------|
| [pelagic-vtpro.md](pelagic-vtpro.md) | 1990s/2000s, 9600 baud, interface cable with a microcontroller | serial | VT Pro, Versa Pro, Pro Plus 2, Atmos, Elite, Wisdom, Aeris 500 AI |
| [pelagic-veo250.md](pelagic-veo250.md) | 2000s, 9600 baud, interface cable with a microcontroller | serial | Veo 180/200/250, React Pro, XR-2, Insight, DG02, XP5 |
| [pelagic-atom2.md](pelagic-atom2.md) | 2007 onwards, 38400/115200 baud; later a BLE wrapper | serial, BLE | Atom, Geo, VT3/VT4/VTX, OC1, i-series, Wisdom 2-4, Amphos, Talis, Zen, XP-Air, F10/F11 |
| [pelagic-i330r.md](pelagic-i330r.md) | 2020s, flash-addressed, PIN-paired | BLE | Aqualung i330R, Apeks DSX |

## Devices and model codes

Every model has a 16-bit model code. Read as big-endian, its two bytes are printable ASCII letters
(`0x4344` = "CD" = Oceanic Geo). The code serves three purposes:

1. It selects the dive-data layout in the parser. The download protocol does not report it in a
   way the parser can rely on, so the host stores the model code with each downloaded dive.
2. On BLE models, the advertised device name starts with the same two letters, followed by the
   decimal serial number (for example `FQ001124` for an i770R). See
   [BLE identification](#ble-identification).
3. On serial models, the host derives it by matching the 16-byte version string (see below).

The table lists every known model, sorted by model code. "Family" names the protocol document.
"Version string" is the 16-byte identification returned by the version command, with `·` marking
bytes that vary (wildcards, see [Version string matching](#version-string-matching)). "Layout"
refers to the [memory layout table](#memory-layouts).

| Code | ASCII | Vendor and model | Family | Version string | Min. firmware | Layout | BLE |
|------|-------|------------------|--------|----------------|---------------|--------|-----|
| 4151 | AQ | Aeris 500 AI | vtpro (INTR variant) | none (empty) | - | L-500AI | |
| 4155 | AU | Oceanic Versa Pro | vtpro | `VERSAPRO ·· 256K` | - | L-32A | |
| 4158 | AX | Aeris Atmos 2 | vtpro | `ATMOSTWO ·· 256K` | - | L-32A | |
| 4159 | AY | Oceanic Pro Plus 2 | vtpro | `PROPLUS2 ·· 256K` | - | L-32A | |
| 4244 | BD | Aeris Atmos AI | vtpro | `ATMOSAIR ·· 256K` | - | L-32A | |
| 4245 | BE | Oceanic VT Pro | vtpro | `VTPRO  r··  256K` | - | L-32A | |
| 4246 | BF | Sherwood Wisdom | vtpro | `WISDOM r··  256K` | - | L-32W | |
| 4247 | BG | Genesis React Pro | veo250 | `GENREACT ·· 256K` | - | L-32V | |
| 424B | BK | Oceanic Veo 200 | veo250 | `VEO 200 R·· 256K` | - | L-32V | |
| 424C | BL | Oceanic Veo 250 | veo250 | `VEO 250 R·· 256K` | - | L-32V | |
| 424F | BO | Aeris Elite | vtpro | `ELITE  r··  256K` | - | L-32A | |
| 4250 | BP | Oceanic Atom 1.0 | atom2 | `ATOM rev··  256K` | - | L-32A | |
| 4251 | BQ | Seemann XP5 | veo250 | `SEEMANN R·· 256K` | - | L-32V | |
| 4252 | BR | Oceanic Veo 180 | veo250 | `VEO 180 R·· 256K` | - | L-32V | |
| 4255 | BU | Aeris XR-2 | veo250 | `AERISXR2 ·· 256K` | - | L-32V | |
| 4257 | BW | Aeris Epic (first code) | atom2 | `2M EPIC r·· 512K` | - | L-64C | |
| 4258 | BX | Oceanic VT3 | atom2 | `OCE VT3 R·· 512K` | - | L-64D | |
| 4259 | BY | Aeris Elite T3 (first code) | atom2 | `ELITET3 R·· 512K` | - | L-64D | |
| 425A | BZ | Sherwood Insight | veo250 | `INSIGHT R·· 256K` | - | L-32V | |
| 4342 | CB | Oceanic Atom 2.0 | atom2 | `2M ATOM r·· 512K` | 0x3349 (below: L-64C) | L-64A | |
| 4344 | CD | Oceanic Geo | atom2 | `OCE GEO R·· 512K` | 0x3242 (below: L-64C) | L-64A | |
| 4345 | CE | Aeris Manta | atom2 | `MANTA  R··  512K` | 0x3242 (below: L-64C) | L-64A | |
| 4346 | CF | Aeris XR-1 NX | atom2 | `AERIS XR1 NX R··` | - | L-EMPTY | |
| 4347 | CG | Oceanic Datamask | atom2 | `DATAMASK ·· 512K` | - | L-64D | |
| 4348 | CH | Aeris Compumask | atom2 | `COMPMASK ·· 512K` | - | L-64D | |
| 434D | CM | Aeris F10 | atom2 | `FREEWAER ·· 512K` | - | L-64F | |
| 434E | CN | Oceanic OC1 (first code) | atom2 | `OCWATCH R·· 1024` | - | L-128O | |
| 4350 | CP | Sherwood Wisdom 2 | atom2 | `WISDOM R··  512K` | below 0x3342 | L-64W | |
| 4352 | CR | Hollis DG02 | veo250 | `HO DGO2 R·· 256K` | - | L-32V | |
| 4353 | CS | Sherwood Insight 2 | atom2 | `INSIGHT2 ·· 512K` | - | L-64A | |
| 4354 | CT | Genesis React Pro White | atom2 (veo250 dive format) | `REACPRO2 ·· 512K` | - | L-64R | |
| 4357 | CW | Tusa Element II (IQ-750) | atom2 | `ELEMENT2 ·· 512K` | - | L-64B | |
| 4358 | CX | Oceanic Veo 1.0 | atom2 | `OCEVEO10 ··   8K` | - | L-EMPTY | |
| 4359 | CY | Oceanic Veo 2.0 | atom2 | `OCEVEO20 ·· 512K` | - | L-64B | |
| 435A | CZ | Oceanic Veo 3.0 | atom2 | `OCEVEO30 ·· 512K` | - | L-64A | |
| 4441 | DA | Tusa Zen (IQ-900) | atom2 | `TUSAZEN ··  512K` | - | L-64B | |
| 4442 | DB | Tusa Zen Air (IQ-950) | atom2 | `TUZENAIR ·· 512K` | - | L-64Z | |
| 4443 | DC | Aeris Atmos AI 2 | atom2 | `ATMOSAI R·· 512K` | - | L-64A | |
| 4444 | DD | Oceanic Pro Plus 2.1 | atom2 | `PROPLUS2 ·· 512K` | - | L-64A | |
| 4446 | DF | Oceanic Geo 2.0 | atom2 | `OCEGEO20 ·· 512K` | - | L-64A | |
| 4447 | DG | Oceanic VT4 | atom2 | `OCEANVT4 ·· 1024` | - | L-128V | |
| 4449 | DI | Oceanic OC1 (second code) | atom2 | `OC1WATCH ·· 1024` | - | L-128O | |
| 444B | DK | Beuchat Voyager 2G | atom2 | `VOYAGE2G ·· 512K` | - | L-64Z | |
| 444C | DL | Oceanic Atom 3.0 | atom2 | `OCEATOM3 ·· 1024` | - | L-128A | |
| 444D | DM | Hollis DG03 | atom2 | `HOLLDG03 ·· 512K` | - | L-64B | |
| 4450 | DP | Oceanic OCS | atom2 | `OCSWATCH ·· 1024` | - | L-128O | |
| 4451 | DQ | Oceanic OC1 (third code) | atom2 | not known | - | (fallback) | |
| 4452 | DR | Oceanic VT 4.1 | atom2 | `OCEAVT41 ·· 1024` | - | L-128V | |
| 4453 | DS | Aeris Epic (second code) | atom2 | `EPIC1  R··  512K` | - | L-64C | |
| 4455 | DU | Aeris Elite T3 (second code) | atom2 | `ELITET31 ·· 512K` | - | L-64D | |
| 4456 | DV | Oceanic Atom 3.1 | atom2 | `ATOM31  ··  1024` | - | L-128A | |
| 4457 | DW | Aeris A300 AI | atom2 | `AERISAIR ·· 1024` | - | L-128V | |
| 4458 | DX | Sherwood Wisdom 3 | atom2 | `WISDOM R··  512K` | 0x3342 | L-64W | |
| 445A | DZ | Aeris A300 | atom2 | `AERIA300 ·· 512K` | - | L-64C | |
| 4542 | EB | Hollis TX1 | atom2 | `HOLLDG04 ·· 2048` | - | L-256T | |
| 4543 | EC | Beuchat Mundial 2 | atom2 | `MUNDIAL R·· 512K` | below 0x3300 | L-64F | |
| 4545 | EE | Sherwood Amphos | atom2 | `AMPHOSSW ·· 512K` | - | L-64Z | |
| 4546 | EF | Sherwood Amphos Air | atom2 | `AMPHOAIR ·· 512K` | - | L-64Z | |
| 4548 | EH | Oceanic Pro Plus 3 | atom2 | `PROPLUS3 ·· 512K` | - | L-64P | |
| 4549 | EI | Aeris F11 | atom2 | `AERISF11 ·· 1024` | - | L-128F | |
| 454B | EK | Oceanic OCi | atom2 | `OCEANOCI ·· 1024` | - | L-128C | |
| 454C | EL | Aeris A300CS | atom2 | `AER300CS ·· 2048` | - | L-256C | |
| 454E | EN | Tusa Talis | atom2 | `TUSTALIS ·· 512K` | - | L-64Z | |
| 4550 | EP | Beuchat Mundial 3 | atom2 | `MUNDIAL R·· 512K` | 0x3300 | L-64F | |
| 4552 | ER | Oceanic Pro Plus X | atom2 | `OCEANOCX ·· ····` | - | L-HX | yes |
| 4553 | ES | Oceanic F10 | atom2 | `OCEANF10 ·· 512K` | - | L-64F | |
| 4554 | ET | Oceanic F11 | atom2 | `OCEANF11 ·· 1024` | - | L-128F | |
| 4555 | EU | Subgear XP-Air | atom2 | `XPSUBAIR ·· 1024` | - | L-128V | |
| 4556 | EV | Sherwood Vision | atom2 | `SWVISION ·· 1024` | - | L-128V | |
| 4557 | EW | Oceanic VTX | atom2 | `OCEANVTX ·· 2048` | - | L-256C | see note 3 |
| 4559 | EY | Aqualung i300 | atom2 | `AQUAI300 ·· 512K` | - | L-64B | |
| 455A | EZ | Aqualung i750TC | atom2 | `AQUAI750 ·· 2048` | - | L-256C | yes |
| 4641 | FA | Aqualung i450T | atom2 | `AQUAI450 ·· 2048` | - | L-256Q | |
| 4642 | FB | Aqualung i550 | atom2 | `AQUAI550 ·· 1024` | - | L-128O | |
| 4646 | FF | Aqualung i200 | atom2 | `AQUAI200 ·· 512K` | - | L-64A | |
| 4647 | FG | Sherwood Sage | atom2 | `SWDRAGON ·· 2048` | - | L-256C | yes |
| 4648 | FH | Aqualung i300C | atom2 | `AQUA300C ·· 512K` | - | L-64B | yes |
| 4649 | FI | Aqualung i200C (first code) | atom2 | `AQUA200C ·· 512K` | - | L-64A | yes |
| 464E | FN | Aqualung i100 (first code) | atom2 | `AQUAI100 ·· 512K` | - | L-64B | |
| 4651 | FQ | Aqualung i770R | atom2 | `AQUA770R ·· ····` | - | L-HR | yes |
| 4652 | FR | Aqualung i550C | atom2 | `AQUA550C ·· 1024` | - | L-128O | yes |
| 4653 | FS | Oceanic Geo 4.0 | atom2 | `OCEGEO40 ·· 512K` | - | L-64B | yes |
| 4654 | FT | Oceanic Veo 4.0 | atom2 | `VEOSMART ·· 512K` | - | L-64B | yes |
| 4655 | FU | Sherwood Wisdom 4 | atom2 | `WISDOM04 ·· 1024` | - | L-128O | yes |
| 4656 | FV | Oceanic Pro Plus 4 | atom2 | `PROPLUS4 ·· 512K` | - | L-64P | yes |
| 4657 | FW | Sherwood Amphos 2.0 | atom2 | `AMPHOS20 ·· 512K` | - | L-64Z | |
| 4658 | FX | Sherwood Amphos Air 2.0 | atom2 | `AMPAIR20 ·· 512K` | - | L-64Z | |
| 4741 | GA | Apeks DSX | i330r | (no version string) | - | L-DSX | only |
| 4742 | GB | Sherwood Beacon | atom2 | `SWBEACON ·· 2048` | - | L-256C | yes |
| 4743 | GC | Aqualung i470TC | atom2 | `AQUA470C ·· 1024` | - | L-128O | yes |
| 4744 | GD | Aqualung i330R | i330r | (no version string) | - | L-I330R | only |
| 4745 | GE | Aqualung i100 (second code) | atom2 | `AQUAI100 ·· 1024` | - | L-128V | |
| 4749 | GI | Aqualung i200C (second code) | atom2 | `AQUA200C ·· 1024` | - | L-128O | yes |
| 474B | GK | Oceanic Geo Air | atom2 | `GEOAIR   ·· 1024` | - | L-128O | yes |
| 474D | GM | Aqualung i330R Console | i330r | (no version string) | - | L-I330R | only |

Notes:

1. Several products have two or three model codes (Epic, Elite T3, OC1, i100, i200C). Each is a
   separate hardware or firmware generation with its own layout and dive format.
2. The same version-string prefix can map to different codes depending on firmware
   (`WISDOM R` → Wisdom 2 or 3; `MUNDIAL R` → Mundial 2 or 3), or on the size suffix
   (`AQUAI100`/`AQUA200C` with `512K` vs `1024`).
3. The VTX also has a Bluetooth link. Its advertised name starts with `EW`. One source reports a
   distinct GATT service and an unframed command mode for it; see
   [pelagic-atom2.md](pelagic-atom2.md#ble).
4. The Beuchat models are Pelagic OEM devices and are listed for completeness.
5. The Subgear XP-Air (`4555`) is a Pelagic device. The other Subgear XP models use a different
   protocol family and are not covered here.

Confidence: **likely** (one reference implementation; another implementation agrees for the model codes
it lists).

## BLE identification

A BLE device belongs to this family if its advertised local name has the form `LLdddddd`: two
ASCII letters equal to the big-endian bytes of a BLE-capable model code (column "BLE" above),
followed only by decimal digits. Matching ignores letter case. The digits are the device serial
number and are used by the atom2 BLE handshake.

Example: name `FQ001124` → letters `F` `Q` → `0x46 0x51` → model code `0x4651` (Aqualung i770R),
serial 001124.

Confidence: **likely**.

## Version string matching

Serial-protocol devices (vtpro, veo250, atom2) answer the version command with a 16-byte ASCII
string. Its structure is:

| Bytes | Content |
|-------|---------|
| 0..8 or 0..7 | Model name, padded with spaces |
| next 2 | Firmware version, two bytes (often ASCII digits) |
| last 4 or 5 | Memory size tag: `256K`, `512K`, `1024`, `2048` (Kbit), `  8K`, or binary data on the newest models |

To identify a device, the host compares the string with a list of 16-byte templates. In a template
a `00` byte matches any byte. All other bytes must match exactly. The bytes that match the first
run of `00` bytes in the template, read big-endian, form the firmware number. A template can also
require a minimum firmware number. Templates are tried in a fixed order, and the first match wins.
Templates with a firmware minimum MUST be tried before the generic template with the same prefix.

Example: the reply `2M ATOM r` `33 4A` ` 512K` gives firmware `0x334A`. This is at least `0x3349`,
so the device is an Atom 2.0 with layout L-64A. With firmware `0x3141` it would get layout L-64C.

If no template matches, one implementation still continues. It picks a layout from the size tag
(`256K` → L-32A, `512K` → L-64D, `1024` → L-128O, `2048` → L-256T, anything else → L-64D) and uses
model code 0, which makes the parser use its default dive format.

Confidence: **likely**.

## Memory model

All four protocols expose a linear byte-addressed memory. The vtpro, veo250 and atom2 protocols read
it in 16-byte pages. The i330r protocol reads arbitrary flash ranges. The first three protocols
share this map:

| Address | Size | Content |
|---------|------|---------|
| `cfg` (0x0000) | 16 | Device-info page: model code, serial number |
| `ptr` (normally 0x0040) | 16 | Global ring-buffer pointers |
| logbook ring | layout-specific | Fixed-size logbook entries, one per dive |
| profile ring | layout-specific | Variable-length dive records (header, samples, footer) |

### Device-info page

| Offset | Type | Meaning |
|--------|------|---------|
| 0x08 | u16be | Model code |
| 0x0A..0x0D | | Serial number, in one of three encodings (column "Serial" in the layout table) |

Serial encodings:

- **S0**: bytes 0x0A, 0x0B, 0x0C are `bcd8`. Serial = b0x0A·10000 + b0x0B·100 + b0x0C.
- **S1**: bytes 0x0B, 0x0C, 0x0D are binary values 0..99. Serial = b0x0B·10000 + b0x0C·100 + b0x0D.
- **S2**: bytes 0x0B, 0x0C, 0x0D are BCD with the nibbles swapped (low nibble is the tens digit).
  Serial = lo(0B)·100000 + hi(0B)·10000 + lo(0C)·1000 + hi(0C)·100 + lo(0D)·10 + hi(0D).

### Global pointer page

| Offset | Type | Meaning |
|--------|------|---------|
| 0x04 | u16le | Logbook pointer A (address of the oldest entry) |
| 0x06 | u16le | Logbook pointer B (see "Logbook pointer meaning") |
| 0x08 | u16le | Profile ring first byte |
| 0x0A | u16le | Profile ring last byte |

"Logbook pointer meaning":

- **G0**: pointer B is the address of the newest entry. The logbook span is [A, B + entry size).
- **G1**: pointer B is already exclusive. The span is [A, B).

If A equals the exclusive end, the span covers the whole ring. The ring is treated as full, and
uninitialised entries (all `FF`) are skipped. The profile ring pointers in this page are not needed
for download, because each logbook entry carries its own profile pointers.

The Aeris 500 AI uses a different pointer page; see [pelagic-vtpro.md](pelagic-vtpro.md). The
i330r family has no pointer page; its pointers come from a "flash map" record, see
[pelagic-i330r.md](pelagic-i330r.md).

### Profile pointers inside a logbook entry

Each logbook entry holds the start and end of its dive's profile in the profile ring. "Profile
pointer encoding" in the layout table selects one of five forms (offsets are relative to the
start of the logbook entry):

| Form | First | Last | Units |
|------|-------|------|-------|
| E0 | u16le at +5 (masked, see below) | u16le at +6, shifted right by 4 | page numbers |
| E1 | u16le at +4 | u16le at +6 | page numbers |
| E2 | u16le at +16 | u16le at +18 | page numbers |
| E3 | u16le at +16 | u16le at +18 | byte addresses |
| E4 | u32le at +8 | u32le at +12 | byte addresses, last is exclusive |

In form E0 the two values are packed 12-bit numbers that share the nibble at byte +6.

For page-number forms (E0, E1, E2), mask each value according to the number of pages in the
non-high-memory part of the device. With N = (memory size − high-memory base) / page unit, the mask
is `0x0FFF` if N ≤ 4096, `0x1FFF` if N ≤ 8192, `0x3FFF` if N ≤ 16384, and `0x7FFF` otherwise.
Then multiply by the page unit. The page unit is 16 bytes, or 256 bytes when the layout has a
high-memory base.

For forms E0..E3, add one page unit to `last` to make it exclusive. Then add the high-memory base
(0 for most layouts) to both values.

Worked example (layout L-64B, 4096 pages, form E0): logbook entry bytes 5..7 are `A4 30 C6`.
- First = u16le(`A4 30`) = 0x30A4, masked with 0x0FFF → page 0x0A4 → address 0x0A40.
- Last = u16le(`30 C6`) = 0xC630, shifted right by 4 → page 0xC63 → address 0xC630, plus one
  page → exclusive end 0xC640.
- The dive's profile occupies 0x0A40..0xC63F.

Confidence: **likely**.

## Memory layouts

The layout labels below are this specification's own names. Sizes and addresses are bytes.
"Read block" is the block size the atom2 protocol uses for normal reads (see
[pelagic-atom2.md](pelagic-atom2.md#commands)). "Dir" is the direction in which the logbook ring is
read: B = backwards from the newest entry, F = forwards from the oldest entry.

| Layout | Memory size | High-mem base | Logbook ring | Entry | Profile ring | Logbook ptr | Profile ptr | Serial | Dir | Read block | Used by |
|--------|-------------|---------------|--------------|-------|--------------|-------------|-------------|--------|-----|------------|---------|
| L-EMPTY | 0x400 | - | empty | 8 | empty | G0 | E0 | S0 | B | 16 | Veo 1.0, XR-1 NX (no dive download) |
| L-32A | 0x8000 | - | 0x0240-0x0440 | 8 | 0x0440-0x8000 | G0 | E0 | S0 | B | 16 / 4×16 | VT Pro family, Atom 1.0 |
| L-32W | 0x8000 | - | 0x03D0-0x05D0 | 8 | 0x05D0-0x8000 | G0 | E0 | S0 | B | 4×16 | Sherwood Wisdom |
| L-32V | 0x8000 | - | 0x0400-0x0600 | 8 | 0x0600-0x8000 | G1 | E1 | S1 | B | 4×16 | Veo 250 family |
| L-500AI | 0x20000 | - | via index command | 8 | 0x0200-0x20000 | special | E1 | S2 | - | 4×16 | Aeris 500 AI |
| L-64D | 0x10000 | - | 0x0240-0x0A40 | 8 | 0x0A40-0x10000 | G0 | E0 | S0 | B | 16 | VT3, Elite T3, Datamask, Compumask |
| L-64A | 0xFFF0 | - | 0x0240-0x0A40 | 8 | 0x0A40-0xFE00 | G0 | E0 | S0 | B | 16 | Atom 2 (newer fw), Geo/Manta (newer fw), Insight 2, Veo 3, Atmos AI 2, Pro Plus 2.1, Geo 2, i200, i200C |
| L-64B | 0x10000 | - | 0x0240-0x0A40 | 8 | 0x0A40-0xFE00 | G0 | E0 | S0 | B | 16 | Element II, Veo 2, Zen, i300, DG03, i100, i300C, Geo 4, Veo 4 |
| L-64C | 0xFFF0 | - | 0x0240-0x0A40 | 8 | 0x0A40-0xFFF0 | G0 | E0 | S0 | B | 16 | Atom 2/Geo/Manta (older fw), Epic, A300 |
| L-64W | 0xFFF0 | - | 0x03D0-0x0A40 | 8 | 0x0A40-0xFE00 | G0 | E0 | S0 | B | 16 | Wisdom 2, Wisdom 3 |
| L-64P | 0x10000 | - | 0x03E0-0x0A40 | 8 | 0x0A40-0xFE00 | G0 | E0 | S0 | B | 128 | Pro Plus 3, Pro Plus 4 |
| L-64Z | 0xFFF0 | - | 0x0240-0x0A40 | 8 | 0x0A40-0xFE00 | G0 | E1 | S0 | B | 16 | Zen Air, Amphos, Amphos Air, Amphos 2.0, Amphos Air 2.0, Voyager 2G, Talis |
| L-64R | 0xFFF0 | - | 0x0400-0x0600 | 8 | 0x0600-0xFE00 | G1 | E1 | S0 | B | 16 | React Pro White |
| L-64F | 0x10000 | - | 0x0100-0x0D80 | 32 | 0x0D80-0x10000 | G0 | E3 | S0 | B | 16 | F10, Mundial 2/3 |
| L-128F | 0x20000 | - | 0x0100-0x0D80 | 32 | 0x0D80-0x20000 | G0 | E2 | S0 | B | 128 | F11 |
| L-128O | 0x20000 | - | 0x0240-0x0A40 | 8 | 0x0A40-0x1FE00 | G0 | E1 | S0 | B | 16 | OC1, OCS, i550, i550C, Wisdom 4, i470TC, i200C (v2), Geo Air |
| L-128C | 0x20000 | - | 0x10C0-0x1400 | 8 | 0x1400-0x1FE00 | G0 | E1 | S0 | B | 16 | OCi |
| L-128A | 0x20000 | - | 0x0400-0x0A40 | 8 | 0x0A40-0x1FE00 | G0 | E1 | S0 | B | 16 | Atom 3.0, Atom 3.1 |
| L-128V | 0x20000 | - | 0x0420-0x0A40 | 8 | 0x0A40-0x1FE00 | G0 | E1 | S0 | B | 16 | VT4, VT 4.1, A300 AI, Vision, XP-Air, i100 (v2) |
| L-256T | 0x40000 | - | 0x0780-0x1000 | 8 | 0x1000-0x40000 | G0 | E1 | S0 | B | 16 | Hollis TX1 |
| L-256C | 0x40000 | - | 0x0900-0x1000 | 16 | 0x1000-0x3FE00 | G0 | E1 | S0 | B | 256 | A300CS, VTX, i750TC, Sage, Beacon |
| L-256Q | 0x40000 | - | 0x10C0-0x1400 | 16 | 0x1400-0x3FE00 | G0 | E1 | S0 | B | 16 | i450T |
| L-HX | 0x440000 | 0x40000 | 0x1000-0x10000 | 16 | 0x40000-0x440000 | G0 | E1 | S0 | B | 256 | Pro Plus X |
| L-HR | 0x640000 | 0x40000 | 0x2000-0x10000 | 16 | 0x40000-0x640000 | G0 | E1 | S0 | B | 256 | i770R |
| L-I330R | 0x400000 | - | 0x102000-0x106000 | 64 | 0x10A000-0x400000 | G1 | E4 | hwcal | F | any | i330R, i330R Console |
| L-DSX | 0x2000000 | - | 0x800000-0x880000 | 512 | 0x1000000-0x2000000 | G1 | E4 | hwcal | B | any | Apeks DSX |

Ring ranges are written as [begin-end): begin is inclusive, end is exclusive. "4×16" means the
vtpro/veo250 multi-page read of up to four 16-byte pages per command.

High memory (L-HX, L-HR): addresses at or above the high-memory base are reached with a separate
read command and 256-byte pages; see [pelagic-atom2.md](pelagic-atom2.md#high-memory).

Confidence: **likely**. See the open questions for values where sources disagree (i770R memory
size, Atom 1.0 logbook end, VT4 logbook begin, React Pro White profile end).

## Ring-buffer arithmetic

All rings are half-open intervals [B, E) of size N = E − B. For two addresses a and b inside the
ring:

- distance(a, b) = (b − a) mod N, computed without going negative.
- If the distance is 0, the result depends on context: it means "whole ring" (N) when measuring a
  logbook span or a single profile, and "nothing" (0) when measuring a gap between two profiles.
- Reading n bytes backwards that end at address e: when the read reaches B, continue from E
  downwards.

## Download procedure

The three page-based protocols and the i330r protocol use the same high-level algorithm. Only the
memory read primitive and the source of the pointers differ.

```
read device info page (model code for reporting, serial)
read pointers → logbook span [lb_begin, lb_end)
if layout logbook ring is empty and profile ring is empty: no dives, stop

// Pass 1: logbook
validate lb_begin and lb_end are inside the logbook ring
    (direction B: invalid begin → use end, giving a full ring; invalid end → error)
    (direction F: invalid end → use begin, giving a full ring; invalid begin → error)
size = distance(lb_begin, lb_end) with 0 meaning "full ring"
read entries one at a time in ring direction:
    direction B: backwards, starting at lb_end
    direction F: forwards, starting at lb_begin
    each entry read is treated as newer than the next one read
    skip entries that are all FF
    stop when an entry equals the stored fingerprint (exclude it)
keep only the entries read before the match (the new dives)

// Pass 2: profile span
remaining = profile ring size; previous = undefined
for each kept entry, newest to oldest:
    skip if all FF
    (first, last) = decode profile pointers (form E0..E4)
    if first or last is outside the profile ring: mark data error, skip entry
    if this is the first valid entry: profile_end = previous = last
    len = distance(first, last) (0 → full ring)
    gap = distance(last, previous) (0 → 0)
    if len + gap > remaining: stop (older profiles have been overwritten)
    remaining -= len + gap; previous = first

// Pass 3: read profiles backwards from profile_end, in large blocks
for each kept entry, newest to oldest (same filtering as pass 2):
    read len + gap bytes ending where the previous dive began
    dive record = logbook entry ‖ the first len bytes of that span
    if the layout has high memory: truncate len to (pages + 1) × 16,
        with pages = ((v & 0xE000) >> 1) | (v & 0x0FFF), v = u16le at logbook entry +12
    deliver the dive record, newest first
```

### Fingerprint

The fingerprint of a dive is its complete logbook entry: 8, 16, 32, 64 or 512 bytes depending on
the layout. A host stores the fingerprint of the newest downloaded dive. The next download stops at
the first logbook entry that is byte-identical to it. The entry contains the dive date/time and
the profile pointers, so it is unique in practice.

Variant: another implementation instead stops at the first dive whose decoded date/time is not
later than the newest date already stored. Both methods work. The entry comparison does not depend
on per-model date decoding.

### Dive record

Every dive is delivered as one byte string:

```
[ logbook entry (entry size bytes) ][ profile bytes from the profile ring ]
```

All offsets in the dive-format sections of the family documents are relative to the start of
this combined record. The profile bytes start with a profile header (normally one or more 16-byte
pages), followed by fixed-size samples and one 16-byte footer page.

### Read granularity

The pointer, logbook and profile reads are made of aligned reads in multiples of 16 bytes. An
implementation SHOULD read in the largest block the protocol allows (4 pages for vtpro/veo250;
16, 128 or 256 bytes for atom2; up to 256 × 16 bytes per request for i330r) and cache the last
block, because the backwards logbook walk touches each block several times.

Confidence: **likely**.

## Shared dive-format conventions

These hold for every protocol unless the family document says otherwise:

- Depth is stored in feet: in whole feet (vtpro, veo250, Atom 1.0), in 1/16 ft (atom2), or in
  1/10 ft (i330R, DSX). Convert with 1 ft = 0.3048 m. The devices report depth already corrected
  for water type, so the host MUST NOT apply its own density correction.
- Temperature is stored in °F, in whole degrees or in 1/10 °F (i330R, DSX). °C = (°F − 32) × 5/9.
- Tank pressure is stored in psi, in units of 1, 2 or 5 psi. bar = psi × 0.0689476. The value
  10000 psi in a header means "no transmitter".
- Gas O2 is stored as an integer percentage. 0 means air (21 %).
- Decompression stop depths are stored in units of 10 ft (except i330R/DSX: 1 ft). NDL and stop
  times are stored in minutes.
- Date and time are local time without a time zone. Most models use a 12-hour clock with a PM flag.
- Two-digit years are relative to 2000. Several older models store only the last digit of the
  year (see the year-2010 rule in [pelagic-atom2.md](pelagic-atom2.md#date-and-time)).

Confidence: **likely**.

## Open questions

- Several layout values differ between the two sources: i770R memory size (0x640000 or 0x440000),
  Atom 1.0 logbook ring end (0x0440 or 0x0A40), VT4/VT 4.1/A300 AI/Vision/XP-Air/i100 v2 logbook
  begin (0x0420 or 0x0240), React Pro White profile end (0xFE00 or 0xFFF0), and F10 profile
  pointer form (E3 byte addresses or E2 page numbers). Device dumps are needed to settle them.
- One source describes an extra "factory dives" state: both logbook pointers equal the profile
  ring begin. The whole logbook ring is then read, and all-`FF` entries are skipped. It is unknown
  which devices produce this state.
- The model code `0x4451` (third OC1 code) has no known version string.
- Direction F (i330R) implies that the newest logbook entry is at the lowest address of the
  span [lb_begin, lb_end). This follows from how the reference implementation walks the ring,
  but no dump has confirmed it.
- The meaning of the profile-ring pointers in the global pointer page (+0x08/+0x0A) is not
  needed and has not been checked against the per-dive pointers.

## References

- libdivecomputer (LGPL-2.1), <https://github.com/libdivecomputer/libdivecomputer>, used as a
  factual reference; no code copied.
- Subsurface's libdivecomputer fork (LGPL-2.1),
  <https://github.com/subsurface/libdc>, used to cross-check variants; no code copied.
