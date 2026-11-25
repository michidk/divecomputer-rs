# USB mass storage and MTP

Some dive computers do not run a download protocol at all: they show up as a USB drive (Mass
Storage Class) or as a Media Transfer Protocol device, and each dive is a file. The host's job
is to find the files, decide which are new, and hand their bytes to the family parser.

In practice the operating system mounts these devices; a host application, including a
browser page, then works with files. Section 1 is what a host needs in that normal case.
Sections 2-4 describe the raw protocols for hosts that must drive the USB device themselves
(for example an Android app without OS mounting).

## 1. File-level view

| Device | Transport | Location of dive files | Notes |
|--------|-----------|------------------------|-------|
| Garmin Descent Mk1 | MSC (`091E:0003`, `091E:2B2B`) | `GARMIN/ACTIVITY/*.FIT` | one FIT file per activity; not every activity is a dive |
| Garmin Descent Mk2/Mk2i/Mk2S/Mk3/Mk3i/G1/G2/X50i, fēnix 8 and similar | MTP | storage root → `Garmin` → `Activity` → `*.fit` | some devices show an extra top level `Primary/` |
| Divesoft Freedom (older USB access) | MSC | `DIVELOG/*.DLF` | one dive per file |
| Genz / XDeep Black | MSC | `LBDATA/<one directory per dive>/SUMMARY.BIN` + `LOGBOOK.BIN` | header and profile in separate files |
| Suunto EON Steel family | device file system reached through the HID/BLE protocol, not MSC | `0:/dives/<hex Unix seconds>.<ext>` | listed here because the naming carries the timestamp |

Path matching SHOULD be case-insensitive (FAT is case-insensitive; devices differ in the case
they report).

**File names carrying timestamps.** Garmin uses either long names
`YYYY-MM-DD-HH-MM-SS.fit` or 8.3 short names `ymdhMMSS.FIT` where:

| Char | Field | Encoding |
|------|-------|----------|
| 1 | year − 2010 | one base-36 digit (`0`-`9`, `A`-`Z`) |
| 2 | month | base-36 digit (1-9, `A`-`C`) |
| 3 | day | base-36 digit (1-9, `A`-`V`) |
| 4 | hour | base-36 digit (0-9, `A`-`N`) |
| 5-6 | minute | two decimal digits |
| 7-8 | second | two decimal digits |

Example: `C4ND0302.FIT` = 2022-04-23 13:03:02. Sorting by the decoded time gives the dive
order; the decoded time (or the FIT file's own timestamp) serves as the fingerprint.

**Incremental download.** Fingerprint-based: the host remembers the newest imported file's
timestamp (from the name or the file contents) and imports only newer files. File modification
times on FAT are local time with 2 s resolution and SHOULD NOT be used.

**Web notes.** Use the File System Access API (`showDirectoryPicker`) on the mounted volume.
Mass storage is a protected interface class in WebUSB and cannot be claimed. MTP devices are
normally claimed by the OS; the user copies files out and picks them with a file input.

**Confidence:** likely.

## 2. USB Mass Storage, Bulk-Only Transport

Interface class `0x08`, subclass `0x06` (SCSI transparent), protocol `0x50` (BOT); one bulk IN
and one bulk OUT endpoint. (One source also accepts a vendor-class `0xFF` interface with the
same endpoints.)

Class requests on the interface:

| Request | bmRequestType | bRequest | Data |
|---------|---------------|----------|------|
| Bulk-Only Mass Storage Reset | `0x21` | `0xFF` | none |
| Get Max LUN | `0xA1` | `0xFE` | 1 byte (highest LUN number) |

Each command is three stages: Command Block Wrapper (OUT), optional data (IN or OUT), Command
Status Wrapper (IN).

**CBW (31 bytes, little-endian fields):**

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | signature `55 53 42 43` ("USBC") |
| 4 | 4 | tag, echoed in the CSW |
| 8 | 4 | data transfer length in bytes |
| 12 | 1 | flags: `0x80` = data IN, `0x00` = data OUT / none |
| 13 | 1 | LUN (bits 0-3) |
| 14 | 1 | command block length (1-16) |
| 15 | 16 | SCSI command block, zero padded |

**CSW (13 bytes):**

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | signature `55 53 42 53` ("USBS") |
| 4 | 4 | tag (must equal the CBW's) |
| 8 | 4 | data residue |
| 12 | 1 | status: 0 passed, 1 failed (issue REQUEST SENSE), 2 phase error (reset recovery) |

Hosts SHOULD check signature, tag and status. One source checks none of them and still works
with the listed devices.

**Confidence:** verified (USB Mass Storage Class Bulk-Only Transport 1.0).

## 3. SCSI commands needed

All multi-byte SCSI fields are big-endian.

| Command | Opcode | CDB | Data IN |
|---------|--------|-----|---------|
| TEST UNIT READY | `00` | `00 00 00 00 00 00` | none |
| REQUEST SENSE | `03` | `03 00 00 00 12 00` | 18 bytes sense data |
| INQUIRY | `12` | `12 00 00 00 24 00` | 36 bytes |
| READ CAPACITY(10) | `25` | `25 00 00 00 00 00 00 00 00 00` | 8 bytes: last LBA `u32be`, block length `u32be` |
| READ(10) | `28` | `28 00 <LBA:u32be> 00 <blocks:u16be> 00` | `blocks × block length` bytes |

A minimal reader needs only READ CAPACITY(10) and READ(10); one source uses only these, with
one block per READ(10) and only the low 16 bits of the block length.

Worked example, READ(10) of one 512-byte block at LBA `0x800`, tag `0x20304050`:

```
H→D bulk  55 53 42 43  50 40 30 20  00 02 00 00  80  00  0A
          28 00 00 00 08 00 00 00 01 00  00 00 00 00 00 00
D→H bulk  <512 bytes>
D→H bulk  55 53 42 53  50 40 30 20  00 00 00 00  00
```

**Confidence:** verified (SCSI Block Commands, SPC).

## 4. FAT file system (summary)

Enough to locate files on these devices; the full rules are in the Microsoft FAT
specification.

1. Read sector 0. If it is a partition table (byte 13 of a boot sector, sectors-per-cluster,
   would be 0; or the `55 AA` MBR signature with a partition entry), read the boot sector at
   the start LBA of the first partition entry (`u32le` at offset `0x1C6`).
2. Boot sector (BPB) fields (`u16le`/`u32le`): bytes per sector @`0x0B`, sectors per cluster
   @`0x0D`, reserved sectors @`0x0E`, number of FATs @`0x10`, root entries @`0x11`, total
   sectors @`0x13` (16-bit) or @`0x20` (32-bit), sectors per FAT @`0x16` (FAT12/16) or
   @`0x24` (FAT32), root cluster @`0x2C` (FAT32).
3. FAT type from the number of data clusters: < 4085 → FAT12, < 65525 → FAT16, else FAT32.
4. Directory entries are 32 bytes: name 8+3 @0, attributes @`0x0B` (`0x10` directory, `0x0F`
   long-name entry), first cluster high @`0x14` (FAT32), first cluster low @`0x1A`, size
   `u32le` @`0x1C`. First name byte `0xE5` = deleted, `0x00` = end of directory.
5. Follow cluster chains through the FAT (12-bit packed, 16-bit or 28-bit entries).

**Confidence:** verified (Microsoft FAT specification), as a summary.

## 5. MTP

MTP runs PTP containers over a still-image-class interface (class `0x06`, subclass `0x01`,
protocol `0x01`) with bulk IN, bulk OUT and interrupt IN endpoints.

Container (`u32le` length, `u16le` type, `u16le` code, `u32le` transaction id, then
parameters/payload). Types: 1 command, 2 data, 3 response, 4 event.

Operations needed to fetch files:

| Code | Operation | Parameters | Result |
|------|-----------|------------|--------|
| `0x1001` | GetDeviceInfo | — | device info dataset |
| `0x1002` | OpenSession | session id | — |
| `0x1003` | CloseSession | — | — |
| `0x1004` | GetStorageIDs | — | array of `u32` storage ids |
| `0x1007` | GetObjectHandles | storage id, format (0 = any), parent (`0xFFFFFFFF` = root) | array of handles |
| `0x1008` | GetObjectInfo | handle | dataset incl. format, size, parent, filename |
| `0x1009` | GetObject | handle | file bytes |

Object formats of interest: `0x3001` association (folder), `0x3000` undefined (used for
`.fit` files). Walk from the root: find the folder whose name matches the next path component
(case-insensitive), list its children, and fetch every object whose name ends in `.fit`.

Garmin specifics: VID `0x091E`; the MTP product ID is `0x4000 | <FIT product id>` (examples:
`0x4CBA` → 3258 Descent Mk2(i); `0x51B6` → 4534 fēnix 8 43 mm). The mass-storage IDs of older
devices appear to follow `0x2000 | <FIT product id>` (`0x2B2B` → 2859 Descent Mk1); this second
rule is an inference from one example. One source uses only the first storage id.

**Confidence:** verified for the PTP/MTP operation codes (MTP 1.1); likely for the Garmin
rules.

## Open questions

- VID:PID ownership of the unidentified mass-storage IDs (`040E:0568`, `2544:0004`,
  `A600:E211`): which belongs to Divesoft, Genz or XDeep?
- Whether Garmin devices other than those listed expose `Primary/` as an extra path level.

## References

- libdivecomputer (LGPL-2.1) and its Subsurface fork (Garmin and USB storage support), used as factual references; no code copied. https://www.libdivecomputer.org/, https://github.com/subsurface/libdc
- USB Mass Storage Class Bulk-Only Transport 1.0, https://www.usb.org/document-library/mass-storage-bulk-only-10
- SCSI Primary Commands and SCSI Block Commands (T10), https://www.t10.org/
- Microsoft Extensible Firmware Initiative FAT32 File System Specification, https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc
- USB Media Transfer Protocol 1.1, https://www.usb.org/document-library/media-transfer-protocol-v11-spec-and-mtp-v11-adopters-agreement
- Garmin FIT SDK (product ids, file format), https://developer.garmin.com/fit/
