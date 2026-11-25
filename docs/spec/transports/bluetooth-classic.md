# Bluetooth classic transport (RFCOMM / SPP)

Dive computers with Bluetooth classic expose the Serial Port Profile. Once connected, the
link is a reliable, ordered byte stream and the family's serial protocol runs over it
unchanged, except that line settings (baud, parity) and modem-control lines have no effect.

## 1. Discovery and connection

1. **Inquiry.** Standard BR/EDR inquiry; each inquiry period is 1.28 s, and hosts typically
   run up to about 8 periods (~10 s). Filter results by device name (section 2).
2. **Service lookup.** Query the remote SDP server for the Serial Port service class
   (16-bit UUID `0x1101`, full form `00001101-0000-1000-8000-00805F9B34FB`) and read the
   RFCOMM channel number from its protocol descriptor list. If the family document gives a
   fixed channel, the SDP step MAY be skipped.
3. **Connect** an RFCOMM socket to that channel.
4. **Pairing.** Devices that require pairing use legacy PIN pairing or Secure Simple Pairing.
   A fixed PIN `0000` is commonly accepted where a PIN is asked for (one source enters it
   automatically for HW OSTC devices). Some devices (Shearwater, newer HW OSTC, Ratio,
   Pelagic) are reached with an **unauthenticated ("insecure") RFCOMM socket**; requesting
   authentication on these can make the connection fail.

One reference implementation waits about 500 ms between cancelling discovery, creating the
socket, connecting and opening the streams; the delays are not known to be required by any
device.

**Confidence:** likely.

## 2. Name filters

Name matching is case-insensitive. "Prefix" means the advertised name starts with the
string; "prefix + digits" means the remainder after the prefix consists only of decimal
digits; "exact" means the whole name.

| Vendor / family | Names | Match |
|-----------------|-------|-------|
| Heinrichs Weikamp (Frog, OSTC 2/3/cR/Plus/Sport/4/5) | `OSTC`, `FROG` | prefix |
| Heinrichs Weikamp, variants seen in another source | `OSTCs`, `BM+SR`, `OSTC #`, `OSTC2`, `OSTC+`, `OSTC3#`, `OSTC4` | prefix |
| Shearwater (Predator, Petrel, Petrel 2, Nerd, Perdix) | `Predator`, `Petrel`, `Petrel 3`, `NERD`, `NERD 2`, `Perdix`, `Perdix 2`, `Perdix 3`, `Teric`, `Peregrine`, `Peregrine TX`, `Tern` | exact in one source, prefix in another |
| Ratio / DiveSystem (iX3M GPS generation) | `DS`, `IX5M`, `RATIO-` | prefix + digits (one source: prefix `IX3M` or `DS`) |
| Tecdiving DiveComputer.eu | `DiveComputer` | exact |
| McLean Extreme | `McLean Extreme` | exact |
| Oceanic VTX (Pelagic) | `EW` | prefix |

The Bluetooth address of the chosen device SHOULD be remembered; several devices of the same
model in range are otherwise indistinguishable.

**Confidence:** likely.

## 3. Behaviour on the link

- Hosts write commands as-is; RFCOMM may split or merge writes, so the family protocol MUST
  NOT rely on write boundaries being preserved.
- Read timeouts from the family's serial description apply. Add margin for radio latency
  (round trips of 30-100 ms are normal).
- Serial `configure`, DTR, RTS and break calls are no-ops. Families whose serial protocol
  needs a baud-rate change mid-session (HW OSTC, Cochran) skip that step over Bluetooth.

**Confidence:** likely.

## 4. Web notes

Chromium desktop exposes paired RFCOMM devices as serial ports through Web Serial
(`navigator.serial.requestPort({allowedBluetoothServiceClassIds:
['00001101-0000-1000-8000-00805f9b34fb']})`). Pairing must be done in the operating system;
there is no browser API for inquiry or PIN entry. Web Bluetooth does not support classic
links.

**Confidence:** likely.

## References

- libdivecomputer (LGPL-2.1), used as a factual reference; no code copied. https://www.libdivecomputer.org/
- Bluetooth Core Specification (inquiry, SDP) and Serial Port Profile 1.2, https://www.bluetooth.com/specifications/specs/
- Bluetooth SIG Assigned Numbers (service class `0x1101`), https://www.bluetooth.com/specifications/assigned-numbers/
- Web Serial API, https://wicg.github.io/serial/
