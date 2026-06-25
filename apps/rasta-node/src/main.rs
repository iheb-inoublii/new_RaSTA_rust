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

    // Test-only interoperability profile. Not approved for production or
    // railway operational use.
    let profile = DIN_RASTA_03_03_INTEROPERABILITY_TEST_PROFILE;
    if let Err(error) = profile.validate() {
        eprintln!("Invalid interoperability-test profile: {:?}", error);
        return;
    }
    let config = RastaConfig {
        sender_id,
        remote_id,
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

        if mode == "B" && api.has_received_data() {
            let mut data = [0u8; 256];
            match api.receive_data(&mut data) {
                Ok(length) => match std::str::from_utf8(&data[..length]) {
                    Ok(text) => println!("Received data: {text:?}"),
                    Err(_) => println!("Received {length} non-UTF-8 data bytes"),
                },
                Err(error) => eprintln!("Failed to receive data: {:?}", error),
            }
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

        if current_state == ConnectionStatus::Down && mode == "A" && data_sent {
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}
