mod profile;

use profile::DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
use rasta_core::config::RastaConfig;
use rasta_core::connection::safety_code::SafetyCodeConfig;
use rasta_core::port::{Transport, TransportError};
use rasta_core::redundancy::{RedundancyCheckCode, RedundancyConfig};
use rasta_core::service::{ConnectionStatus, RastaService};
use rasta_platform::std_clock::StdClock;
use rasta_platform::udp::UdpSocketTransport;
use std::cell::Cell;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeRole {
    A,
    B,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NodeSettings {
    role: NodeRole,
    local_addr_a: String,
    remote_addr_a: String,
    local_addr_b: String,
    remote_addr_b: String,
    sender_id: u32,
    remote_id: u32,
    trace_wire: bool,
}

fn parse_node_settings(args: &[String]) -> Result<NodeSettings, &'static str> {
    if args.len() < 3 {
        return Err("missing arguments");
    }

    let remote_ip = parse_ip(&args[2])?;
    let mut local_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let mut trace_wire = false;
    let mut channel_0_local_port = match args[1].as_str() {
        "A" => 5000,
        "B" => 5001,
        _ => return Err("invalid role"),
    };
    let mut channel_0_remote_port = match args[1].as_str() {
        "A" => 5001,
        "B" => 5000,
        _ => return Err("invalid role"),
    };
    let mut channel_1_local_port = match args[1].as_str() {
        "A" => 6000,
        "B" => 6001,
        _ => return Err("invalid role"),
    };
    let mut channel_1_remote_port = match args[1].as_str() {
        "A" => 6001,
        "B" => 6000,
        _ => return Err("invalid role"),
    };
    let mut sender_id = match args[1].as_str() {
        "A" => 0x1234,
        "B" => 0x5678,
        _ => return Err("invalid role"),
    };
    let mut remote_id = match args[1].as_str() {
        "A" => 0x5678,
        "B" => 0x1234,
        _ => return Err("invalid role"),
    };

    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--trace-wire" => {
                trace_wire = true;
                index += 1;
            }
            "--local-ip" => {
                index += 1;
                local_ip = parse_ip(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-0-local-port" => {
                index += 1;
                channel_0_local_port = parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-0-remote-port" => {
                index += 1;
                channel_0_remote_port = parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-1-local-port" => {
                index += 1;
                channel_1_local_port = parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-1-remote-port" => {
                index += 1;
                channel_1_remote_port = parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--local-id" => {
                index += 1;
                sender_id = parse_node_id(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--remote-id" => {
                index += 1;
                remote_id = parse_node_id(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            _ => return Err("invalid option"),
        }
    }

    if channel_0_local_port == channel_1_local_port {
        return Err("duplicate local ports");
    }
    if sender_id == remote_id {
        return Err("invalid node ids");
    }

    match args[1].as_str() {
        "A" => Ok(NodeSettings {
            role: NodeRole::A,
            local_addr_a: socket_addr(local_ip, channel_0_local_port),
            remote_addr_a: socket_addr(remote_ip, channel_0_remote_port),
            local_addr_b: socket_addr(local_ip, channel_1_local_port),
            remote_addr_b: socket_addr(remote_ip, channel_1_remote_port),
            sender_id,
            remote_id,
            trace_wire,
        }),
        "B" => Ok(NodeSettings {
            role: NodeRole::B,
            local_addr_a: socket_addr(local_ip, channel_0_local_port),
            remote_addr_a: socket_addr(remote_ip, channel_0_remote_port),
            local_addr_b: socket_addr(local_ip, channel_1_local_port),
            remote_addr_b: socket_addr(remote_ip, channel_1_remote_port),
            sender_id,
            remote_id,
            trace_wire,
        }),
        _ => Err("invalid role"),
    }
}

fn parse_ip(value: &str) -> Result<IpAddr, &'static str> {
    value.parse::<IpAddr>().map_err(|_| "invalid ip")
}

fn socket_addr(ip: IpAddr, port: u16) -> String {
    SocketAddr::new(ip, port).to_string()
}

fn parse_port(value: &str) -> Result<u16, &'static str> {
    let port = value.parse::<u16>().map_err(|_| "invalid port")?;
    if port == 0 {
        return Err("invalid port");
    }
    Ok(port)
}

fn parse_node_id(value: &str) -> Result<u32, &'static str> {
    let id = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16).map_err(|_| "invalid node id")?
    } else {
        value.parse::<u32>().map_err(|_| "invalid node id")?
    };
    if id == 0 {
        return Err("invalid node id");
    }
    Ok(id)
}

#[derive(Clone)]
struct WireTrace {
    enabled: bool,
    channel: &'static str,
    order: Rc<Cell<u64>>,
}

struct TraceTransport<T> {
    inner: T,
    trace: WireTrace,
}

impl<T> TraceTransport<T> {
    fn new(inner: T, trace: WireTrace) -> Self {
        Self { inner, trace }
    }
}

impl<T: Transport> Transport for TraceTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        if self.trace.enabled {
            log_wire("TX", self.trace.channel, self.trace.order.get(), data);
            self.trace
                .order
                .set(self.trace.order.get().saturating_add(1));
        }
        self.inner.send(data)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        let length = self.inner.receive(buffer)?;
        if self.trace.enabled && length > 0 {
            log_wire(
                "RX",
                self.trace.channel,
                self.trace.order.get(),
                &buffer[..length],
            );
            self.trace
                .order
                .set(self.trace.order.get().saturating_add(1));
        }
        Ok(length)
    }
}

fn log_wire(direction: &str, channel: &str, order: u64, bytes: &[u8]) {
    eprintln!(
        "wire order={order} dir={direction} channel={channel} len={} {}",
        bytes.len(),
        decode_wire_summary(bytes)
    );
    eprintln!("wire hex={}", hex_bytes(bytes));
}

fn decode_wire_summary(bytes: &[u8]) -> String {
    if bytes.len() < 8 {
        return "decode=rl-too-short".to_string();
    }
    let rl_len = u16::from_le_bytes([bytes[0], bytes[1]]);
    let rl_reserve = u16::from_le_bytes([bytes[2], bytes[3]]);
    let rl_sequence = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let mut summary = format!("rl_len={rl_len} rl_reserve={rl_reserve} rl_sequence={rl_sequence}");
    if bytes.len() >= 12 {
        let srl_len = u16::from_le_bytes([bytes[8], bytes[9]]);
        let srl_type = u16::from_le_bytes([bytes[10], bytes[11]]);
        summary.push_str(&format!(" srl_len={srl_len} srl_type={srl_type}"));
    }
    summary
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(3));
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = match parse_node_settings(&args) {
        Ok(settings) => settings,
        Err("missing arguments") => {
            println!("Usage: {} <A|B> <remote_ip> [interop options]", args[0]);
            return;
        }
        Err(_) => {
            println!("Invalid arguments. Use A or B and valid interop options.");
            return;
        }
    };

    let mode = match settings.role {
        NodeRole::A => "A",
        NodeRole::B => "B",
    };

    println!("Starting node {}", mode);
    println!("Local ID: {}", settings.sender_id);
    println!("Remote ID: {}", settings.remote_id);
    println!("Wire tracing: {}", settings.trace_wire);
    println!(
        "Channel A: {} -> {}",
        settings.local_addr_a, settings.remote_addr_a
    );
    println!(
        "Channel B: {} -> {}",
        settings.local_addr_b, settings.remote_addr_b
    );

    let transport_a = match UdpSocketTransport::new(&settings.local_addr_a, &settings.remote_addr_a)
    {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("Failed to bind redundancy channel A: {error}");
            return;
        }
    };
    let transport_b = match UdpSocketTransport::new(&settings.local_addr_b, &settings.remote_addr_b)
    {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("Failed to bind redundancy channel B: {error}");
            return;
        }
    };
    let trace_order = Rc::new(Cell::new(0));
    let transport_a = TraceTransport::new(
        transport_a,
        WireTrace {
            enabled: settings.trace_wire,
            channel: "channel-0",
            order: trace_order.clone(),
        },
    );
    let transport_b = TraceTransport::new(
        transport_b,
        WireTrace {
            enabled: settings.trace_wire,
            channel: "channel-1",
            order: trace_order,
        },
    );

    // Test-only interoperability profile. Not approved for production or
    // railway operational use.
    let profile = DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
    if let Err(error) = profile.validate() {
        eprintln!("Invalid interoperability-test profile: {:?}", error);
        return;
    }
    let config = RastaConfig {
        sender_id: settings.sender_id,
        remote_id: settings.remote_id,
        safety_code: SafetyCodeConfig::md4_low8(profile.md4_initial_value),
        redundancy: RedundancyConfig {
            check_code: RedundancyCheckCode::OptionB,
            t_seq_ms: profile.t_seq_ms,
        },
        t_max: profile.t_max_ms,
        initial_seq: 0,
        heartbeat_interval_ms: profile.t_h_ms,
        n_send_max: profile.n_send_max as u16,
        mwa: profile.mwa as u16,
    };

    let mut api = match RastaService::new(transport_a, transport_b, StdClock::new(), config) {
        Ok(api) => api,
        Err(error) => {
            eprintln!("Invalid RaSTA configuration: {:?}", error);
            return;
        }
    };

    if settings.role == NodeRole::A {
        println!("Opening client connection...");
    } else {
        println!("Opening server connection...");
    }
    if let Err(error) = api.open_connection() {
        eprintln!("Failed to open connection: {:?}", error);
        return;
    }

    let mut last_state = api.status();
    let mut data_sent = false;
    let start_time = std::time::Instant::now();

    loop {
        if let Err(e) = api.poll() {
            println!("Error during poll: {:?}", e);
            while let Some(diagnostic) = api.take_diagnostic() {
                eprintln!("RaSTA diagnostic: {:?}", diagnostic);
            }
            break;
        }

        let current_state = api.status();
        if current_state != last_state {
            println!("State transition: {:?} -> {:?}", last_state, current_state);
            last_state = current_state;
        }

        if settings.role == NodeRole::B && api.has_received_data() {
            let mut data = [0u8; 256];
            match api.receive_data(&mut data) {
                Ok(length) => match std::str::from_utf8(&data[..length]) {
                    Ok(text) => println!("Received data: {text:?}"),
                    Err(_) => println!("Received {length} non-UTF-8 data bytes"),
                },
                Err(error) => eprintln!("Failed to receive data: {:?}", error),
            }
        }

        if current_state == ConnectionStatus::Up && settings.role == NodeRole::A && !data_sent {
            println!("Sending data: 'Hello from A'");
            if let Err(error) = api.send_data(b"Hello from A") {
                eprintln!("Failed to send data: {:?}", error);
                break;
            }
            data_sent = true;
        }

        if settings.role == NodeRole::A
            && data_sent
            && start_time.elapsed() > Duration::from_secs(5)
        {
            println!("Graceful disconnect...");
            if let Err(error) = api.close_connection() {
                eprintln!("Failed to disconnect: {:?}", error);
            }
            break;
        }

        if current_state == ConnectionStatus::Down && settings.role == NodeRole::A && data_sent {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeRole, decode_wire_summary, hex_bytes, parse_node_settings};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn rejects_missing_and_invalid_role_arguments() {
        assert_eq!(
            parse_node_settings(&args(&["rasta-node"])),
            Err("missing arguments")
        );
        assert_eq!(
            parse_node_settings(&args(&["rasta-node", "A"])),
            Err("missing arguments")
        );
        assert_eq!(
            parse_node_settings(&args(&["rasta-node", "C", "127.0.0.1"])),
            Err("invalid role")
        );
    }

    #[test]
    fn parses_node_a_and_b_port_assignments() {
        let a = parse_node_settings(&args(&["rasta-node", "A", "127.0.0.1"])).unwrap();
        assert_eq!(a.role, NodeRole::A);
        assert_eq!(a.local_addr_a, "0.0.0.0:5000");
        assert_eq!(a.remote_addr_a, "127.0.0.1:5001");
        assert_eq!(a.local_addr_b, "0.0.0.0:6000");
        assert_eq!(a.remote_addr_b, "127.0.0.1:6001");
        assert_eq!(a.sender_id, 0x1234);
        assert_eq!(a.remote_id, 0x5678);
        assert!(!a.trace_wire);

        let b = parse_node_settings(&args(&["rasta-node", "B", "127.0.0.1"])).unwrap();
        assert_eq!(b.role, NodeRole::B);
        assert_eq!(b.local_addr_a, "0.0.0.0:5001");
        assert_eq!(b.remote_addr_a, "127.0.0.1:5000");
        assert_eq!(b.local_addr_b, "0.0.0.0:6001");
        assert_eq!(b.remote_addr_b, "127.0.0.1:6000");
        assert_eq!(b.sender_id, a.remote_id);
        assert_eq!(b.remote_id, a.sender_id);
        assert!(!b.trace_wire);
    }

    #[test]
    fn parses_optional_wire_trace_and_interop_overrides() {
        let settings = parse_node_settings(&args(&[
            "rasta-node",
            "A",
            "127.0.0.1",
            "--trace-wire",
            "--local-ip",
            "127.0.0.1",
            "--channel-0-local-port",
            "15000",
            "--channel-0-remote-port",
            "15001",
            "--channel-1-local-port",
            "16000",
            "--channel-1-remote-port",
            "16001",
            "--local-id",
            "0x1111",
            "--remote-id",
            "8738",
        ]))
        .unwrap();

        assert!(settings.trace_wire);
        assert_eq!(settings.local_addr_a, "127.0.0.1:15000");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:15001");
        assert_eq!(settings.local_addr_b, "127.0.0.1:16000");
        assert_eq!(settings.remote_addr_b, "127.0.0.1:16001");
        assert_eq!(settings.sender_id, 0x1111);
        assert_eq!(settings.remote_id, 8738);
    }

    #[test]
    fn rejects_invalid_interop_options() {
        assert_eq!(
            parse_node_settings(&args(&["rasta-node", "A", "not-an-ip"])),
            Err("invalid ip")
        );
        assert_eq!(
            parse_node_settings(&args(&[
                "rasta-node",
                "A",
                "127.0.0.1",
                "--channel-0-local-port",
                "0"
            ])),
            Err("invalid port")
        );
        assert_eq!(
            parse_node_settings(&args(&[
                "rasta-node",
                "A",
                "127.0.0.1",
                "--channel-0-local-port",
                "5000",
                "--channel-1-local-port",
                "5000"
            ])),
            Err("duplicate local ports")
        );
        assert_eq!(
            parse_node_settings(&args(&["rasta-node", "A", "127.0.0.1", "--local-id", "0"])),
            Err("invalid node id")
        );
        assert_eq!(
            parse_node_settings(&args(&[
                "rasta-node",
                "A",
                "127.0.0.1",
                "--local-id",
                "7",
                "--remote-id",
                "7"
            ])),
            Err("invalid node ids")
        );
    }

    #[test]
    fn wire_summary_and_hex_formatter_are_deterministic() {
        let frame = [
            0x0c, 0x00, 0x00, 0x00, 0x44, 0x33, 0x22, 0x11, 0x24, 0x00, 0x60, 0x18,
        ];
        assert_eq!(
            decode_wire_summary(&frame),
            "rl_len=12 rl_reserve=0 rl_sequence=287454020 srl_len=36 srl_type=6240"
        );
        assert_eq!(hex_bytes(&frame), "0c 00 00 00 44 33 22 11 24 00 60 18");
    }
}
