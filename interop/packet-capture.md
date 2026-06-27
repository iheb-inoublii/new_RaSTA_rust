# Packet capture support

Operating-system capture tools are authoritative. The Rust `--trace-wire`
option is a convenience log, not a substitute for Wireshark or tcpdump.

## Determine configured ports

Use the ports printed by `rasta-node` at startup. Defaults:

- Node A local: channel 0 `5000`, channel 1 `6000`
- Node B local: channel 0 `5001`, channel 1 `6001`

If overrides are used, replace the filter ports below with the configured
values.

## Windows

List UDP endpoints:

```powershell
netstat -ano -p udp
```

Wireshark display filter for default ports:

```text
udp.port == 5000 || udp.port == 5001 || udp.port == 6000 || udp.port == 6001
```

PowerShell run example:

```powershell
cargo run -p rasta-node --release -- A <peer-ip> --trace-wire
```

## Linux

List UDP endpoints:

```bash
ss -lunp
```

Capture default ports:

```bash
sudo tcpdump -ni any 'udp and (port 5000 or port 5001 or port 6000 or port 6001)'
```

Capture to a file:

```bash
sudo tcpdump -ni any -w rasta-interop.pcap 'udp and (port 5000 or port 5001 or port 6000 or port 6001)'
```

Wireshark display filter:

```text
udp.port == 5000 || udp.port == 5001 || udp.port == 6000 || udp.port == 6001
```

## Checklist before blaming protocol bytes

- correct local IP;
- correct peer IP;
- no port collision;
- firewall permits inbound UDP on both channels;
- packets are sent from the source address/port expected by the peer;
- both channels are visible in capture.
