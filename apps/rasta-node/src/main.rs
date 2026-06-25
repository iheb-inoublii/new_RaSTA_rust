mod profile;

use profile::DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
use rasta_core::config::RastaConfig;
use rasta_core::connection::safety_code::SafetyCodeConfig;
use rasta_core::redundancy::{RedundancyCheckCode, RedundancyConfig};
use rasta_core::service::{ConnectionStatus, RastaService};
use rasta_platform::std_clock::StdClock;
use rasta_platform::udp::UdpSocketTransport;
use std::env;
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
}

fn parse_node_settings(args: &[String]) -> Result<NodeSettings, &'static str> {
    if args.len() < 3 {
        return Err("missing arguments");
    }

    let mode = args[1].as_str();
    let remote_ip = args[2].clone();
    match mode {
        "A" => Ok(NodeSettings {
            role: NodeRole::A,
            local_addr_a: "0.0.0.0:5000".to_string(),
            remote_addr_a: format!("{remote_ip}:5001"),
            local_addr_b: "0.0.0.0:6000".to_string(),
            remote_addr_b: format!("{remote_ip}:6001"),
            sender_id: 0x1234,
            remote_id: 0x5678,
        }),
        "B" => Ok(NodeSettings {
            role: NodeRole::B,
            local_addr_a: "0.0.0.0:5001".to_string(),
            remote_addr_a: format!("{remote_ip}:5000"),
            local_addr_b: "0.0.0.0:6001".to_string(),
            remote_addr_b: format!("{remote_ip}:6000"),
            sender_id: 0x5678,
            remote_id: 0x1234,
        }),
        _ => Err("invalid role"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = match parse_node_settings(&args) {
        Ok(settings) => settings,
        Err("missing arguments") => {
            println!("Usage: {} <A|B> <remote_ip>", args[0]);
            return;
        }
        Err(_) => {
            println!("Invalid mode. Use A or B.");
            return;
        }
    };

    let mode = match settings.role {
        NodeRole::A => "A",
        NodeRole::B => "B",
    };

    if args.len() < 3 {
        println!("Usage: {} <A|B> <remote_ip>", args[0]);
        return;
    }

    println!("Starting node {}", mode);
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
    use super::{NodeRole, parse_node_settings};

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

        let b = parse_node_settings(&args(&["rasta-node", "B", "127.0.0.1"])).unwrap();
        assert_eq!(b.role, NodeRole::B);
        assert_eq!(b.local_addr_a, "0.0.0.0:5001");
        assert_eq!(b.remote_addr_a, "127.0.0.1:5000");
        assert_eq!(b.local_addr_b, "0.0.0.0:6001");
        assert_eq!(b.remote_addr_b, "127.0.0.1:6000");
        assert_eq!(b.sender_id, a.remote_id);
        assert_eq!(b.remote_id, a.sender_id);
    }
}
