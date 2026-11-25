# Checksums, CRCs and other integrity mechanisms

Every integrity check used by any family in this specification, with full parameters and a
test vector. Family documents refer to the anchors below (lowercase catalogue name,
non-alphanumerics replaced by `-`).

CRC parameters follow the Rocksoft/RevEng model: `width`, `poly` (normal, MSB-first form),
`init`, `refin`, `refout`, `xorout`, `check` (CRC of the ASCII bytes `123456789`) and
`residue` (register value after running the CRC, without final XOR, over a message followed by
its correct CRC). All test vectors were computed for this document with an independent
bit-by-bit implementation.

## Summary

| Anchor | Kind | check("123456789") | Families |
|--------|------|-------------------|----------|
| [crc-8-smbus](#crc-8-smbus) | CRC-8, poly `07` | `F4` | Halcyon Symbios |
| [crc-8-msb-0x12](#crc-8-msb-0x12) | CRC-8, poly `12` (non-standard) | `EA` | Pelagic i330R / Apeks DSX (BLE packets) |
| [crc-16-ibm-3740](#crc-16-ibm-3740) | CRC-16 (a.k.a. CCITT-FALSE) | `29B1` | Cressi Leonardo, Cressi Goa (dive header), ReefNet Sensus Pro and Ultra, Ratio/DiveSystem iDive, Liquivision (replies), Seac Screen, Pelagic i330R (data) |
| [crc-16-xmodem](#crc-16-xmodem) | CRC-16 | `31C3` | Cressi Goa (frames), Oceans S1 (XMODEM-CRC), McLean Extreme, Tecdiving DiveComputer.eu, Mares Genius-class records |
| [crc-16-ibm-sdlc](#crc-16-ibm-sdlc) | CRC-16 (a.k.a. X-25) | `906E` | IrDA IrLAP FCS (Uwatec Smart), Divesoft Freedom (packets) |
| [crc-16-arc](#crc-16-arc) | CRC-16 | `BB3D` | Garmin FIT files, Garmin GFDI over BLE |
| [crc-16-modbus](#crc-16-modbus) | CRC-16 | `4B37` | Divesoft Freedom (dive header) |
| [crc-32-iso-hdlc](#crc-32-iso-hdlc) | CRC-32 (zlib/Ethernet) | `CBF43926` | Suunto EON Steel family |
| [crc-32-bzip2](#crc-32-bzip2) | CRC-32, MSB-first | `FC891918` | Liquivision Xen/Xeo/Lynx/Kaon (dive header, keyed) |
| [sum8](#sum8) | 8-bit additive | `DD` | Oceanic/Pelagic (many), Mares Nemo, Mares Puck/Darwin (ASCII protocol), Suunto Eon, Deepblu (verify) |
| [sum8-neg](#sum8-neg) | two's complement of sum8 | `23` | Zeagle N2iTiON3, Deepblu Cosmiq (send), HW OSTC (firmware blocks), Intel HEX |
| [sum8-inv](#sum8-inv) | ones' complement of sum8 | `22` | Deep Six Excursion |
| [sum16](#sum16) | 16-bit additive of bytes | `01DD` | Atomic Cobalt, Oceanic Atom 2 (256-byte pages), Uwatec Aladin, ReefNet Sensus, Sporasub SP2 |
| [nibble-sum](#nibble-sum) | 8-bit sum of nibbles | `48` | Oceanic VT Pro family, Pelagic (some handshakes) |
| [xor8](#xor8) | 8-bit XOR | `31` | Suunto Vyper/Vyper2/D9 families, Uwatec Memomouse, Uwatec Smart over serial (Meridian/Chromis/Mantis) |
| [ostc-firmware-sum](#ostc-firmware-sum) | 32-bit Fletcher-like | `091501DD` | HW OSTC 3 firmware files |

**Confidence:** verified for all `check` values (computed); likely for the family assignments.

## crc-8-smbus

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 8 | `07` | `00` | false | false | `00` | `F4` | `00` |

Plain MSB-first CRC-8. Halcyon Symbios computes it over the payload of each packet (from the
byte after the start byte up to, but excluding, the CRC byte).

**Confidence:** likely.

## crc-8-msb-0x12

Not in the RevEng catalogue (the polynomial has no `x^0` term, so it is not a proper CRC
generator).

| width | poly | init | refin | refout | xorout | check |
|-------|------|------|-------|--------|--------|-------|
| 8 | `12` | `00` | false | false | `00` | `EA` |

Byte-wise formulation (equivalent, checked on 2000 random messages):

```
for each byte b:
    a = crc XOR b
    t = a XOR (a >> 4) XOR (a >> 7)
    crc = ((t << 4) XOR (t << 1)) AND 0xFF
```

Pelagic i330R / Apeks DSX: computed over the whole BLE packet (start byte `CD` through the end
of the payload) with the CRC byte position (offset 3) set to `00` during calculation.

**Confidence:** likely (two sources describe the same algorithm; equivalence to the
polynomial form verified numerically).

## crc-16-ibm-3740

Also known as CRC-16/CCITT-FALSE, CRC-16/AUTOSAR.

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 16 | `1021` | `FFFF` | false | false | `0000` | `29B1` | `0000` |

Users (coverage and byte order; details in the family documents):

| Family | Covered bytes | Wire order |
|--------|---------------|-----------|
| Cressi Leonardo | ASCII hex payload characters (command and reply); also the 32000-byte binary dump | 4 ASCII hex chars, big-endian |
| Cressi Goa | dive header record (parser) | little-endian |
| ReefNet Sensus Pro | handshake; full memory dump | little-endian |
| ReefNet Sensus Ultra | handshake (24 bytes); each 512-byte data page; user data | little-endian |
| Ratio / DiveSystem iDive | start byte, length and payload | big-endian |
| Liquivision | reply payload | big-endian |
| Seac Screen | packet; also stored records | big-endian |
| Pelagic i330R | each flash data block | big-endian |

**Confidence:** likely.

## crc-16-xmodem

Same as above but `init = 0000`.

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 16 | `1021` | `0000` | false | false | `0000` | `31C3` | `0000` |

| Family | Covered bytes | Wire order |
|--------|---------------|-----------|
| Cressi Goa (serial and BLE frames) | length, command and arguments (after the 3 `AA` preamble bytes) | little-endian |
| Oceans S1 | XMODEM-CRC blocks: the data bytes after the 3-byte block header | big-endian |
| McLean Extreme, Tecdiving | from the byte after the start byte through the payload; the CRC is followed by two `00` bytes | big-endian |
| Mares Icon HD family (Genius-class dive records) | record body | little-endian |

Worked example (Cressi Goa frame, dive request for dive id 1): CRC over `02 22 01 00` is
`0x36FF`, giving the frame `AA AA AA 02 22 01 00 FF 36 55`.

**Confidence:** likely.

## crc-16-ibm-sdlc

Also known as CRC-16/X-25, CRC-16/ISO-HDLC.

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 16 | `1021` | `FFFF` | true | true | `FFFF` | `906E` | `F0B8` |

Reflected form of the polynomial: `8408`. Transmitted least significant byte first.

- IrDA IrLAP FCS (see [../transports/irda.md](../transports/irda.md)). Example: frame `55 11`
  → FCS `A300`, sent `00 A3`.
- Divesoft Freedom packets: over the packet header and payload, little-endian.

**Confidence:** verified for IrLAP (public standard + computed example); likely for Divesoft.

## crc-16-arc

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 16 | `8005` | `0000` | true | true | `0000` | `BB3D` | `0000` |

The Garmin FIT SDK computes it with a 16-entry nibble table; the result is identical.

- FIT files: over the header (14-byte header CRC) and over the whole file (trailing CRC),
  little-endian.
- Garmin GFDI messages over BLE: over length, message id and payload, little-endian.

**Confidence:** verified for FIT (Garmin FIT SDK); likely for GFDI.

## crc-16-modbus

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 16 | `8005` | `FFFF` | true | true | `0000` | `4B37` | `0000` |

Divesoft Freedom dive files: over the header from offset 6 to its end; stored little-endian
at offset 4.

**Confidence:** likely.

## crc-32-iso-hdlc

The zlib / Ethernet CRC-32.

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 32 | `04C11DB7` | `FFFFFFFF` | true | true | `FFFFFFFF` | `CBF43926` | `DEBB20E3` |

Suunto EON Steel family: over the 12-byte message header and payload, little-endian (USB HID
and BLE).

**Confidence:** likely.

## crc-32-bzip2

| width | poly | init | refin | refout | xorout | check | residue |
|-------|------|------|-------|--------|--------|-------|---------|
| 32 | `04C11DB7` | `FFFFFFFF` | false | false | `FFFFFFFF` | `FC891918` | `C704DD7B` |

Liquivision dive headers are protected by this CRC computed over a buffer formed by a 4-byte
**model key** followed by the header bytes from offset 4 onward (the first 4 header bytes hold
the stored CRC, `u32le`). Coverage ends 2 bytes before the header's end (6 bytes for some Xeo
firmware). The key is a 4-byte value reported by the device's info command (Xen, Xeo, Lynx)
or a fixed value (Kaon); see the Liquivision family document.

**Confidence:** likely.

## sum8

Sum of all covered bytes modulo 256, `init = 00`. check = `DD`.

Users include: Oceanic/Pelagic version replies and 16- and 128-byte pages (Atom 2, Veo 250,
VT Pro); Oceanic Atom 2 handshake; Mares Nemo (two copies per packet); Mares Puck/Darwin/Nemo
ASCII protocol (sum of the ASCII characters between `<` and the checksum, sent as two hex
characters); Suunto Eon; Deepblu Cosmiq (receive: sum of all bytes including the checksum
must be `00`).

Example (Mares ASCII read command `51 00 01 20`): sum of the ASCII characters `"51000120"` is
`0x189` → checksum `89`.

**Confidence:** likely.

## sum8-neg

Two's complement of [sum8](#sum8): `(0x100 − sum8) & 0xFF`, so that the sum of data plus
checksum is `00`. check = `23`.

Users: Zeagle N2iTiON3 (command and reply payloads); Deepblu Cosmiq (transmit, where the sum
is seeded with `command + 2 × length`); HW OSTC 2 firmware upload blocks; Intel HEX records.

**Confidence:** likely.

## sum8-inv

Ones' complement of [sum8](#sum8): `sum8 XOR FF`. check = `22`.

Users: Deep Six Excursion (over header and payload of each packet).

**Confidence:** likely.

## sum16

Sum of all covered **bytes** (each 0..255) modulo 65536, `init = 0000`. check = `01DD`.

| Family | Coverage | Wire order |
|--------|----------|-----------|
| Atomic Aquatics Cobalt | version reply; each dive record | little-endian |
| Oceanic Atom 2 family | 256-byte page reads | little-endian |
| Uwatec Aladin | full memory dump | little-endian |
| ReefNet Sensus | memory dump | little-endian |
| Sporasub SP2 | command packets (seeded with the command byte); reply payload | big-endian |

**Confidence:** likely.

## nibble-sum

Sum of the high and low nibble of every covered byte, modulo 256, `init = 00`. check = `48`.

Users: Oceanic VT Pro family (each 8-byte half page); Pelagic handshake replies in one source.

**Confidence:** likely.

## xor8

XOR of all covered bytes, `init = 00`. check = `31`.

Users: Suunto Vyper, Vyper2, D9 families (commands and replies; covers everything before the
checksum); Uwatec Memomouse; Uwatec Smart over the serial cradle (Meridian/Chromis/Mantis:
over length and command on transmit, over status, length and payload on receive).

Example (Suunto version query): `05 00 16 14` → XOR `07`, frame `05 00 16 14 07`.

**Confidence:** likely.

## ostc-firmware-sum

Used only to verify HW OSTC 3 firmware files (not dive data). Two 16-bit accumulators, both
starting at 0: for each byte `b`, `low = (low + b) mod 65536`, `high = (high + low) mod 65536`;
result = `high << 16 | low`, stored `u32le` at the end of the file. Like Fletcher-32 but over
bytes and modulo 2^16 instead of 2^16 − 1. check = `091501DD`.

**Confidence:** likely.

## Other integrity and obfuscation mechanisms

| Mechanism | Family | Description |
|-----------|--------|-------------|
| Command complement byte | Mares Icon HD family | each command byte is followed by `command XOR A5` |
| Echo checking | several serial families | the device echoes the command; the host compares |
| SLIP framing only | Shearwater | no checksum; see the Shearwater family document |
| XOR-delta + run-length compression | Shearwater (Petrel-class logs) | each 32-byte block XORed with the previous block after RLE expansion; see the Shearwater family document |
| HDLC-style framing | Suunto EON Steel (BLE), Divesoft | see [../transports/ble.md](../transports/ble.md) |
| COBS framing | Garmin GFDI (BLE) | see the Garmin family document |
| AES-128 | HW OSTC 3 firmware files | firmware images are encrypted with AES-128 used as a 128-bit-feedback stream cipher: keystream block = AES(previous ciphertext block), first block AES(IV); concerns firmware update only, not dive download |

No dive-log download in this specification uses encryption. Pelagic i330R uses a PIN /
access-code exchange for pairing, which is authentication, not encryption.

**Confidence:** likely.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Greg Cook, Catalogue of parametrised CRC algorithms (RevEng), https://reveng.sourceforge.io/crc-catalogue/
- Ross Williams, A Painless Guide to CRC Error Detection Algorithms (parameter model)
- Garmin FIT SDK, file CRC, https://developer.garmin.com/fit/protocol/
- IrDA IrLAP 1.1 (FCS), Infrared Data Association
- Intel Hexadecimal Object File Format Specification
