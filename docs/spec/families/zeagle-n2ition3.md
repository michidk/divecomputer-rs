# Zeagle N2iTiON3 family

The Zeagle N2iTiON3 and its rebadged versions talk over a 4800-baud serial link with
small framed packets: STX, length, payload, a negated 8-bit sum, ETX. The device echoes
every command before its answer. Memory is 32 KiB, read in pieces of up to 64 bytes. It
holds a 60-entry logbook of pointers into a profile ring. The dive data uses
nibble-packed decimal (BCD) fields in the same layout as the Cressi Edy family. The
reference implementation decodes both families with the same parser.

No public vendor documentation is known. The only source is libdivecomputer, so the
best confidence here is **likely**.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|
| Zeagle | N2iTiON3 | 0 | serial (interface cable) | |
| Apeks | Quantum X | 0 | serial | same protocol |
| Dive Rite | NiTek Trio | 0 | serial | same protocol |
| Scubapro | XTender 5 | 0 | serial | same protocol |

The protocol does not tell the models apart; the model code is always 0. There are no
USB, Bluetooth or IrDA identities. The cable is a serial or USB-serial interface.

Confidence: **likely**.

## Transport parameters

- 4800 baud, 8N1, no flow control.
- Read timeout 1000 ms.
- Flush both directions after opening.

Confidence: **likely**.

## Session

After opening, the host sends one "init" packet and reads the answer. The reference
ignores both the answer and any error from this step:

```
H→D  02 01 00 41 BF 03
D→H  02 01 00 41 BF 03  <13-byte answer frame: payload 8 bytes, content unknown>
```

There is no explicit shutdown command.

Confidence: **likely** (the purpose of the init packet is unknown).

## Commands

| Command | Payload (inside the frame) | Answer payload | Notes |
|---------|----------------------------|----------------|-------|
| Init | `41` | 8 bytes, unknown | sent once after open |
| Read memory | `4D a0 a1 n 00 00 00 00` | 1 status byte (unknown meaning) + n data bytes | a = 16-bit LE address, n = 1..64 |

`41` and `4D` are ASCII 'A' and 'M'.

## Framing and integrity

Frame, in both directions:

| Field | Size | Value |
|-------|------|-------|
| STX | 1 | `02` |
| length | 2 | u16le, number of payload bytes |
| payload | length | |
| checksum | 1 | two's complement of the [sum8](../common/checksums.md#sum8) of the payload: (0x100 − (Σ payload mod 256)) mod 256 |
| ETX | 1 | `03` |

The device first echoes the host's frame byte for byte, then sends its own frame. A host
reading a reply with an n-byte answer payload therefore expects command length + n + 5
bytes. It should check:

1. that the echo equals the command;
2. STX and ETX of the answer frame;
3. that the length field + command length + 5 equals the number of bytes received;
4. that the checksum is correct.

(The reference accepts a frame unless *both* the STX and the ETX are wrong. A strict
check of each is recommended.)

Worked example: read 64 bytes from 0x7EC0.

```
payload  4D C0 7E 40 00 00 00 00     sum8 = 0xCB → checksum 0x35
H→D      02 08 00 4D C0 7E 40 00 00 00 00 35 03
D→H      02 08 00 4D C0 7E 40 00 00 00 00 35 03        (echo, 13 bytes)
         02 41 00 <s> <64 data bytes> <cks> 03           (answer, 70 bytes)
```

The data starts at byte 17 of the combined reply (13 echo + 4).

Init checksum: payload `41` → 0x100 − 0x41 = `BF`.

Confidence: **likely**.

## Memory layout

32768 bytes (0x0000–0x7FFF).

| Range | Size | Content |
|-------|------|---------|
| 0x3FA0–0x7EBF | 16160 | profile ring buffer |
| 0x7EC0–0x7F37 | 120 | logbook: 60 × u16le, start address of each dive's profile |
| 0x7F3C | 1 | index of the newest logbook entry (0..59) |
| 0x7F3D | 1 | index of the oldest logbook entry (0..59) |
| 0x7F3E | 2 | u16le end-of-profile pointer: where the next dive will start |

A single read of 128 bytes from 0x7EC0 returns all of the logbook and pointer fields:
at offsets 0x00–0x77 within it, then 0x7C, 0x7D and 0x7E. Bytes 0x78–0x7B are unknown.

Empty logbook: the newest index is `FF`. Any other index of 60 or more, or a pointer
outside the profile ring, means the data is corrupt.

Confidence: **likely**.

## Download procedure

```
cfg = read(0x7EC0, 128)                 # two 64-byte reads
newest, oldest = cfg[0x7C], cfg[0x7D]
if newest == 0xFF: no dives
count = ((newest − oldest) mod 60) + 1
eop = u16le(cfg[0x7E])
end = eop; total = 0; idx = newest
for i in 0 .. count−1:
    start = u16le(cfg[2·idx])
    len = ring_len(start, end)          # (end − start) mod 16160; equal counts as a full ring
    if total + len > 16160: stop        # older dives were overwritten in the profile ring
    total += len
    dive = read the profile ring bytes [start, end), wrapping at 0x7EC0 → 0x3FA0
    if dive[0..16] == fingerprint: stop
    emit dive (newest first)
    end = start; idx = (idx − 1) mod 60
```

Reads are at most 64 bytes long. A full dump is 512 reads of 64 bytes from address 0.

Fingerprint: the first 16 bytes of the dive record.

Confidence: **likely**.

## Dive data format

A dive record is a 32-byte header followed by samples. The fields are packed decimal
digits. Positions below are **nibble indices** counted from the start of the record:
nibble 2k is the high nibble of byte k, and nibble 2k+1 is its low nibble. "d digits at
nibble p" means the decimal number formed by nibbles p … p+d−1, most significant first.

### Header

| Nibble | Digits | Meaning | Conversion |
|--------|--------|---------|------------|
| 1 | 3 | average depth | 0.1 m |
| 5 | 3 | maximum depth | 0.1 m |
| 8 | 2 | year | + 2000 |
| 10 | 1 | month | 1..9; the digit is a hex nibble, so 10..12 appear as A..C (inferred) |
| 11 | 2 | day | |
| 22 | 3 | minimum water temperature | 0.1 °C |
| 25 | 3 | dive time | minutes |
| 28 | 2 | hour | |
| 30 | 2 | minute | |
| 46, 44, 42 | 2 each | O₂ % of mix 1, 2, 3 | fraction = value/100 |

Gas mixes: mix i (i = 0, 1, 2) sits at nibble 46 − 2i. If the first nibble of a mix field
is `F`, that mix is disabled. The enabled mixes come first, so counting stops at the
first disabled mix. Helium is not recorded.

Units are always metric, and the date has no seconds or time zone.

### Samples

Samples start at byte 32. Each sample is 2 bytes, or 6 bytes when bit 7 of its first byte
is set; the extra 4 bytes are undecoded.

| Element | Location | Meaning |
|---------|----------|---------|
| end marker | first byte = `FF` | no more samples |
| extended flag | bit 7 of byte 0 | 4 extra bytes follow the 2-byte sample |
| gas | bits 5–6 of byte 0 | mix index 0..2 |
| depth | nibbles 1, 2, 3 of the sample (low nibble of byte 0, then byte 1) | 3 digits, 0.1 m |

Sample interval: 30 s fixed (the reference uses 30 s for every model of this family).

Worked sample: `21 23` → gas bits 01 → mix 1; depth digits 1, 2, 3 → 12.3 m.

Confidence: **uncertain**. The format is inferred from sharing the parser with the
Cressi Edy family. No dumps from these models were checked, and the 30 s interval in
particular is unconfirmed.

## Model-specific variations

None known; all four models report model code 0.

## Open questions

1. The content of the init answer and of the status byte before read data.
2. Bytes 0x78–0x7B of the logbook block.
3. The sample interval and the meaning of the 4 extra bytes in extended samples (alarms?
   tank pressure?).
4. Whether the month field encodes 10–12 as one hex nibble.

## References

- libdivecomputer (LGPL-2.1), https://www.libdivecomputer.org/, used as a factual reference; no code copied.
- See also `cressi-edy.md` (same dive data layout), if present in this specification.
