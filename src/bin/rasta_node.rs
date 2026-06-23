use rasta_stack::adapters::socket_transport::UdpSocketTransport;
use rasta_stack::adapters::standard_clock::StdClock;
use rasta_stack::adapters::standard_timer::StdTimer;
use rasta_stack::application::service_interface::{ConnectionStatus, RastaService};
use rasta_stack::config::DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
use rasta_stack::core::connection::RastaConfig;
use rasta_stack::core::redundancy::RedundancyConfig;
use rasta_stack::core::safety_code::SafetyCodeConfig;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <A|B> <remote_ip>", args[0]);
        return;
    }

    let mode = &args[1];
    let remote_ip = &args[2];

    let (local_addr_a, remote_addr_a, local_addr_b, remote_addr_b, sender_id, remote_id) =
        if mode == "A" {
            (
                "0.0.0.0:5000",
                format!("{}:5001", remote_ip),
                "0.0.0.0:6000",
                format!("{}:6001", remote_ip),
                0x1234,
                0x5678,
            )
        } else if mode == "B" {
            (
                "0.0.0.0:5001",
                format!("{}:5000", remote_ip),
                "0.0.0.0:6001",
                format!("{}:6000", remote_ip),
                0x5678,
                0x1234,
            )
        } else {
            println!("Invalid mode. Use A or B.");
            return;
        };

    println!("Starting node {}", mode);
    println!("Channel A: {} -> {}", local_addr_a, remote_addr_a);
    println!("Channel B: {} -> {}", local_addr_b, remote_addr_b);

    let transport_a = match UdpSocketTransport::new(local_addr_a, &remote_addr_a) {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("Failed to bind redundancy channel A: {error}");
            return;
        }
    };
    let transport_b = match UdpSocketTransport::new(local_addr_b, &remote_addr_b) {
        Ok(transport) => transport,
        Err(error) => {
            eprintln!("Failed to bind redundancy channel B: {error}");
            return;
        }
    };

    let config = RastaConfig {
        sender_id,
        remote_id,
        safety_code: SafetyCodeConfig::md4_low8(
            DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE.md4_initial_value,
        ),
        redundancy: RedundancyConfig::default(),
        t_max: 2000,
        initial_seq: 0,
        heartbeat_interval_ms: 300,
        n_send_max: 20,
        mwa: 10,
    };

    let mut api =
        match RastaService::new(transport_a, transport_b, StdTimer::new(), StdClock, config) {
            Ok(api) => api,
            Err(error) => {
                eprintln!("Invalid RaSTA configuration: {:?}", error);
                return;
            }
        };

    if mode == "A" {
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
            break;
        }

        let current_state = api.status();
        if current_state != last_state {
            println!("State transition: {:?} -> {:?}", last_state, current_state);
            last_state = current_state;
        }

        if current_state == ConnectionStatus::Up && mode == "A" && !data_sent {
            println!("Sending data: 'Hello from A'");
            if let Err(error) = api.send_data(b"Hello from A") {
                eprintln!("Failed to send data: {:?}", error);
                break;
            }
            data_sent = true;
        }

        if mode == "A" && data_sent && start_time.elapsed() > Duration::from_secs(5) {
            println!("Graceful disconnect...");
            if let Err(error) = api.close_connection() {
                eprintln!("Failed to disconnect: {:?}", error);
            }
            break;
        }

        if mode == "B" && start_time.elapsed() > Duration::from_secs(10) {
            println!("Node B timeout, exiting.");
            break;
        }

        if current_state == ConnectionStatus::Down && mode == "A" && data_sent {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}
