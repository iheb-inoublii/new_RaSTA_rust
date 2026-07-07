use rasta_core::application::ApplicationMessage;
use rasta_core::config::RastaProfile;
use rasta_core::endpoint::{ConnectionStatus, RastaEndpoint, config_from_profile};
use rasta_platform::std_clock::StdClock;
use rasta_platform::udp::UdpSocketTransport;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_RUN_SECONDS: u64 = 30;
const DEFAULT_ROUNDS: u32 = 10;
const MAX_RUN_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Active,
    Passive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeProfile {
    Academic,
    LibrastaLocal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Settings {
    role: Role,
    remote_ip: IpAddr,
    profile: RuntimeProfile,
    local_addr_a: String,
    remote_addr_a: String,
    local_addr_b: String,
    remote_addr_b: String,
    sender_id: u32,
    remote_id: u32,
    rounds: u32,
    run_seconds: u64,
    trace: bool,
}

#[derive(Clone, Copy)]
struct EndpointDefaults {
    channel_0_local_port: u16,
    channel_0_remote_port: u16,
    channel_1_local_port: u16,
    channel_1_remote_port: u16,
    sender_id: u32,
    remote_id: u32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let settings = match parse_settings(&args) {
        Ok(settings) => settings,
        Err("missing arguments") => {
            println!(
                "Usage: {} <active|passive> <remote_ip> [--rounds N] [--run-seconds N] [--profile academic|librasta-local] [--trace|--trace-wire]",
                args[0]
            );
            return;
        }
        Err(_) => {
            println!("Invalid arguments. Use active/passive, a remote IP, and valid options.");
            return;
        }
    };

    println!("Starting ping-pong-node {:?}", settings.role);
    println!("Local ID: {}", settings.sender_id);
    println!("Remote ID: {}", settings.remote_id);
    println!("Profile: {:?}", settings.profile);
    println!("Rounds: {}", settings.rounds);
    println!("Run seconds: {}", settings.run_seconds);
    println!(
        "Channel A: {} -> {}",
        settings.local_addr_a, settings.remote_addr_a
    );
    println!(
        "Channel B: {} -> {}",
        settings.local_addr_b, settings.remote_addr_b
    );

    let mut endpoint = match build_endpoint(&settings) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("Failed to create endpoint: {error}");
            return;
        }
    };

    if let Err(error) = endpoint.connect() {
        eprintln!("Failed to open connection: {error}");
        return;
    }

    let start = Instant::now();
    let mut up_since: Option<Instant> = None;
    let mut last_status = endpoint.status();
    let mut next_ping = 1u32;
    let mut next_pong = 1u32;
    let mut next_ping_at = Instant::now();
    let mut waiting_for_pong = false;

    loop {
        if let Err(error) = endpoint.poll() {
            println!("Error during poll: {error}");
            endpoint
                .drain_diagnostics(|diagnostic| eprintln!("RaSTA diagnostic: {:?}", diagnostic));
            break;
        }
        drain_trace(&mut endpoint, settings.trace);

        let status = endpoint.status();
        if status != last_status {
            println!("State transition: {:?} -> {:?}", last_status, status);
            last_status = status;
        }
        if status == ConnectionStatus::Up && up_since.is_none() {
            up_since = Some(Instant::now());
        }

        if status == ConnectionStatus::Up {
            match settings.role {
                Role::Active => run_active(
                    &mut endpoint,
                    settings.rounds,
                    &mut next_ping,
                    &mut next_pong,
                    &mut waiting_for_pong,
                    &mut next_ping_at,
                ),
                Role::Passive => run_passive(&mut endpoint),
            }
        }

        if settings.role == Role::Active && next_pong > settings.rounds {
            println!("Completed {} ping-pong rounds", settings.rounds);
            println!("Graceful disconnect...");
            if let Err(error) = endpoint.close() {
                eprintln!("Failed to disconnect: {error}");
            }
            break;
        }

        if start.elapsed() >= Duration::from_secs(settings.run_seconds) {
            if settings.role == Role::Active {
                println!("Run duration expired; graceful disconnect...");
                if let Err(error) = endpoint.close() {
                    eprintln!("Failed to disconnect: {error}");
                }
            }
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn run_active<T1, T2, C>(
    endpoint: &mut RastaEndpoint<T1, T2, C>,
    rounds: u32,
    next_ping: &mut u32,
    next_pong: &mut u32,
    waiting_for_pong: &mut bool,
    next_ping_at: &mut Instant,
) where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    if !*waiting_for_pong && *next_ping <= rounds && Instant::now() >= *next_ping_at {
        send_message(
            endpoint,
            ApplicationMessage::Ping {
                counter: *next_ping,
            },
        );
        println!("Ping({}) sent", *next_ping);
        *waiting_for_pong = true;
    }

    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    while endpoint.has_received_data() {
        match endpoint.receive(&mut buffer) {
            Ok(length) => match ApplicationMessage::decode(&buffer[..length]) {
                Ok(ApplicationMessage::Pong { counter }) => {
                    println!("Pong({counter}) received");
                    if counter == *next_pong {
                        *next_ping = next_ping.saturating_add(1);
                        *next_pong = next_pong.saturating_add(1);
                        *waiting_for_pong = false;
                        *next_ping_at = Instant::now() + Duration::from_millis(250);
                    } else {
                        eprintln!(
                            "Unexpected Pong counter: expected {}, got {counter}",
                            *next_pong
                        );
                    }
                }
                Ok(other) => eprintln!("Unexpected active message: {:?}", other),
                Err(error) => eprintln!("Malformed active message: {:?}", error),
            },
            Err(error) => {
                eprintln!("Receive failed: {error}");
                break;
            }
        }
    }
}

fn run_passive<T1, T2, C>(endpoint: &mut RastaEndpoint<T1, T2, C>)
where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    while endpoint.has_received_data() {
        match endpoint.receive(&mut buffer) {
            Ok(length) => match ApplicationMessage::decode(&buffer[..length]) {
                Ok(ApplicationMessage::Ping { counter }) => {
                    println!("Ping({counter}) received");
                    send_message(endpoint, ApplicationMessage::Pong { counter });
                    println!("Pong({counter}) sent");
                }
                Ok(other) => eprintln!("Unexpected passive message: {:?}", other),
                Err(error) => eprintln!("Malformed passive message: {:?}", error),
            },
            Err(error) => {
                eprintln!("Receive failed: {error}");
                break;
            }
        }
    }
}

fn send_message<T1, T2, C>(endpoint: &mut RastaEndpoint<T1, T2, C>, message: ApplicationMessage)
where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    match message.encode(&mut buffer) {
        Ok(length) => {
            if let Err(error) = endpoint.send(&buffer[..length]) {
                eprintln!("Send failed: {error}");
            }
        }
        Err(error) => eprintln!("Encode failed: {:?}", error),
    }
}

fn build_endpoint(
    settings: &Settings,
) -> Result<
    RastaEndpoint<UdpSocketTransport, UdpSocketTransport, StdClock>,
    rasta_core::endpoint::RastaError,
> {
    let transport_a = UdpSocketTransport::new(&settings.local_addr_a, &settings.remote_addr_a)
        .map_err(|_| rasta_core::endpoint::RastaError::Transport)?;
    let transport_b = UdpSocketTransport::new(&settings.local_addr_b, &settings.remote_addr_b)
        .map_err(|_| rasta_core::endpoint::RastaError::Transport)?;
    let profile = match settings.profile {
        RuntimeProfile::Academic => RastaProfile::academic_default()?,
        RuntimeProfile::LibrastaLocal => RastaProfile::librasta_local()?,
    };
    let config = config_from_profile(
        settings.sender_id,
        settings.remote_id,
        profile,
        settings.profile == RuntimeProfile::LibrastaLocal,
    )?;
    RastaEndpoint::from_config(transport_a, transport_b, StdClock::new(), config)
}

fn drain_trace<T1, T2, C>(endpoint: &mut RastaEndpoint<T1, T2, C>, enabled: bool)
where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    if enabled {
        endpoint.drain_trace_events(|event| println!("trace {:?}", event));
    }
}

fn parse_settings(args: &[String]) -> Result<Settings, &'static str> {
    if args.len() < 3 {
        return Err("missing arguments");
    }
    let role = match args[1].as_str() {
        "active" => Role::Active,
        "passive" => Role::Passive,
        _ => return Err("invalid role"),
    };
    let remote_ip = parse_ip(&args[2])?;
    let local_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let mut profile = RuntimeProfile::Academic;
    let mut endpoint = defaults(role, profile);
    let mut rounds = DEFAULT_ROUNDS;
    let mut run_seconds = DEFAULT_RUN_SECONDS;
    let mut trace = false;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                profile = parse_profile(args.get(index).ok_or("missing option value")?)?;
                endpoint = defaults(role, profile);
                index += 1;
            }
            "--rounds" => {
                index += 1;
                rounds = parse_rounds(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--run-seconds" => {
                index += 1;
                run_seconds = parse_run_seconds(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--trace" | "--trace-wire" => {
                trace = true;
                index += 1;
            }
            _ => return Err("invalid option"),
        }
    }
    Ok(Settings {
        role,
        remote_ip,
        profile,
        local_addr_a: socket_addr(local_ip, endpoint.channel_0_local_port),
        remote_addr_a: socket_addr(remote_ip, endpoint.channel_0_remote_port),
        local_addr_b: socket_addr(local_ip, endpoint.channel_1_local_port),
        remote_addr_b: socket_addr(remote_ip, endpoint.channel_1_remote_port),
        sender_id: endpoint.sender_id,
        remote_id: endpoint.remote_id,
        rounds,
        run_seconds,
        trace,
    })
}

fn defaults(role: Role, profile: RuntimeProfile) -> EndpointDefaults {
    match (role, profile) {
        (Role::Active, RuntimeProfile::Academic) => EndpointDefaults {
            channel_0_local_port: 5000,
            channel_0_remote_port: 5001,
            channel_1_local_port: 6000,
            channel_1_remote_port: 6001,
            sender_id: 0x1234,
            remote_id: 0x5678,
        },
        (Role::Passive, RuntimeProfile::Academic) => EndpointDefaults {
            channel_0_local_port: 5001,
            channel_0_remote_port: 5000,
            channel_1_local_port: 6001,
            channel_1_remote_port: 6000,
            sender_id: 0x5678,
            remote_id: 0x1234,
        },
        (Role::Active, RuntimeProfile::LibrastaLocal) => EndpointDefaults {
            channel_0_local_port: 9998,
            channel_0_remote_port: 8888,
            channel_1_local_port: 9999,
            channel_1_remote_port: 8889,
            sender_id: 0x60,
            remote_id: 0x61,
        },
        (Role::Passive, RuntimeProfile::LibrastaLocal) => EndpointDefaults {
            channel_0_local_port: 8888,
            channel_0_remote_port: 9998,
            channel_1_local_port: 8889,
            channel_1_remote_port: 9999,
            sender_id: 0x61,
            remote_id: 0x60,
        },
    }
}

fn parse_profile(value: &str) -> Result<RuntimeProfile, &'static str> {
    match value {
        "academic" => Ok(RuntimeProfile::Academic),
        "librasta-local" => Ok(RuntimeProfile::LibrastaLocal),
        _ => Err("invalid profile"),
    }
}

fn parse_ip(value: &str) -> Result<IpAddr, &'static str> {
    value.parse::<IpAddr>().map_err(|_| "invalid ip")
}

fn socket_addr(ip: IpAddr, port: u16) -> String {
    SocketAddr::new(ip, port).to_string()
}

fn parse_rounds(value: &str) -> Result<u32, &'static str> {
    let rounds = value.parse::<u32>().map_err(|_| "invalid rounds")?;
    if rounds == 0 {
        return Err("invalid rounds");
    }
    Ok(rounds)
}

fn parse_run_seconds(value: &str) -> Result<u64, &'static str> {
    let seconds = value.parse::<u64>().map_err(|_| "invalid run seconds")?;
    if seconds == 0 || seconds > MAX_RUN_SECONDS {
        return Err("invalid run seconds");
    }
    Ok(seconds)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_ROUNDS, Role, RuntimeProfile, parse_settings};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_active_defaults() {
        let settings = parse_settings(&args(&["ping-pong-node", "active", "127.0.0.1"])).unwrap();
        assert_eq!(settings.role, Role::Active);
        assert_eq!(settings.profile, RuntimeProfile::Academic);
        assert_eq!(settings.rounds, DEFAULT_ROUNDS);
        assert_eq!(settings.local_addr_a, "0.0.0.0:5000");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:5001");
        assert_eq!(settings.sender_id, 0x1234);
        assert_eq!(settings.remote_id, 0x5678);
    }

    #[test]
    fn parses_passive_librasta_local_and_rounds() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "passive",
            "127.0.0.1",
            "--profile",
            "librasta-local",
            "--rounds",
            "12",
            "--run-seconds",
            "20",
            "--trace",
        ]))
        .unwrap();
        assert_eq!(settings.role, Role::Passive);
        assert_eq!(settings.profile, RuntimeProfile::LibrastaLocal);
        assert_eq!(settings.rounds, 12);
        assert_eq!(settings.run_seconds, 20);
        assert!(settings.trace);
        assert_eq!(settings.local_addr_a, "0.0.0.0:8888");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:9998");
    }
}
