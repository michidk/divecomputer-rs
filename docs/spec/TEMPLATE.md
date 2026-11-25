# Vendor family name (template)

One-paragraph summary: which devices, which transports, and the shape of the protocol.

## Devices

| Vendor | Model | Model code | Transports | Notes |
|--------|-------|------------|------------|-------|

Identification for each transport: USB VID:PID, BLE advertised-name pattern, Bluetooth
classic name, IrDA nickname.

## Transport parameters

Per transport: baud rate, data bits, parity, stop bits, flow control, DTR/RTS handling,
timeouts and inter-byte delays; BLE service, write and notify characteristic UUIDs, write
type, packet size and MTU; USB HID report sizes. Refer to `../transports/` for shared
details.

## Session

Wake-up, handshake, authentication or pairing, version/identity query, shutdown. Exact byte
sequences with directions.

## Commands

| Command | H→D bytes | Arguments | D→H response | Notes |
|---------|-----------|-----------|--------------|-------|

## Framing and integrity

Packet layout, escaping, compression, encryption, checksums (refer to
`../common/checksums.md`). Include at least one complete worked example frame.

## Memory layout

Address map, page/block sizes, ring buffers and their pointers, logbook vs profile areas.

## Download procedure

Step-by-step algorithm a host follows, including incremental download (fingerprint) and
end conditions. Fresh, language-neutral pseudocode is allowed.

## Dive data format

### Header
Table: offset, type, raw unit, conversion, meaning — per model group where layouts differ.

### Samples
Sample record layouts, sample interval, depth, temperature, tank pressure, deco/NDL,
ppO2, events/alarms, gas switches.

### Date and time
Encoding and epoch; time-zone handling.

## Model-specific variations

## Open questions

## References

Public documents and projects consulted (no code copied).
