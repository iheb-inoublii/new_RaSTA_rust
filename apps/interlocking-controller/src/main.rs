use rasta_core::application::{ApplicationMessage, movement_authority_for_signal};
use rasta_core::config::RastaProfile;
use rasta_core::endpoint::{ConnectionStatus, RastaEndpoint, config_from_profile};
use rasta_platform::std_clock::StdClock;
use rasta_platform::udp::UdpSocketTransport;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_RUN_SECONDS: u64 = 15;
const MAX_RUN_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeProfile {
    Academic,
    LibrastaLocal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Settings {
    remote_ip: IpAddr,
    profile: RuntimeProfile,
    local_addr_a: String,
    remote_addr_a: String,
    local_addr_b: String,
    remote_addr_b: String,
    sender_id: u32,
    remote_id: u32,
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
                "Usage: {} <remote_ip> [--profile academic|librasta-local] [--run-seconds N] [--trace|--trace-wire]",
                args[0]
            );
            return;
        }
        Err(_) => {
            println!("Invalid arguments. Use a valid remote IP and options.");
            return;
        }
    };

    println!("Starting interlocking-controller");
    println!("Local ID: {}", settings.sender_id);
    println!("Remote ID: {}", settings.remote_id);
    println!("Profile: {:?}", settings.profile);
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

    println!("Opening interlocking connection...");
    if let Err(error) = endpoint.connect() {
        eprintln!("Failed to open connection: {error}");
        return;
    }

    let mut last_status = endpoint.status();
    let mut up_since: Option<Instant> = None;

    loop {
        if let Err(error) = endpoint.poll() {
            println!("Error during poll: {error}");
            endpoint
                .drain_diagnostics(|diagnostic| eprintln!("RaSTA diagnostic: {:?}", diagnostic));
            break;
        }
        drain_trace(&mut endpoint, settings.trace);
        endpoint.drain_diagnostics(|diagnostic| eprintln!("RaSTA diagnostic: {:?}", diagnostic));

        let status = endpoint.status();
        if status != last_status {
            println!("State transition: {:?} -> {:?}", last_status, status);
            last_status = status;
        }
        if status == ConnectionStatus::Up && up_since.is_none() {
            up_since = Some(Instant::now());
        }

        if status == ConnectionStatus::Up {
            run_interlocking_flow(&mut endpoint);
        }

        if up_since
            .is_some_and(|start| start.elapsed() >= Duration::from_secs(settings.run_seconds))
        {
            println!("Graceful disconnect...");
            if let Err(error) = endpoint.close() {
                eprintln!("Failed to disconnect: {error}");
            }
            break;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn run_interlocking_flow<T1, T2, C>(endpoint: &mut RastaEndpoint<T1, T2, C>)
where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    while endpoint.has_received_data() {
        match endpoint.receive(&mut buffer) {
            Ok(length) => match ApplicationMessage::decode(&buffer[..length]) {
                Ok(ApplicationMessage::SignalStatus { signal_id, aspect }) => {
                    println!(
                        "Interlocking received: SignalStatus(signal_id={signal_id}, aspect={aspect:?})"
                    );
                    let response = movement_authority_for_signal(signal_id, aspect);
                    if let ApplicationMessage::MovementAuthority {
                        allow_green,
                        reason_code,
                        ..
                    } = response
                    {
                        println!(
                            "Interlocking sent: MovementAuthority(signal_id={signal_id}, allow_green={allow_green}, reason_code={reason_code})"
                        );
                    }
                    send_message(endpoint, response);
                }
                Ok(ApplicationMessage::Ping { counter }) => {
                    println!("Interlocking received: Ping({counter})");
                    println!("Interlocking sent: Pong({counter})");
                    send_message(endpoint, ApplicationMessage::Pong { counter });
                }
                Ok(other) => println!("Interlocking received unexpected message: {:?}", other),
                Err(error) => eprintln!("Interlocking received malformed message: {:?}", error),
            },
            Err(error) => {
                eprintln!("Interlocking receive failed: {error}");
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
                eprintln!("Interlocking send failed: {error}");
            }
        }
        Err(error) => eprintln!("Interlocking encode failed: {:?}", error),
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
    if args.len() < 2 {
        return Err("missing arguments");
    }
    let remote_ip = parse_ip(&args[1])?;
    let local_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let mut profile = RuntimeProfile::Academic;
    let mut endpoint = defaults(profile);
    let mut run_seconds = DEFAULT_RUN_SECONDS;
    let mut trace = false;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                profile = parse_profile(args.get(index).ok_or("missing option value")?)?;
                endpoint = defaults(profile);
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
        remote_ip,
        profile,
        local_addr_a: socket_addr(local_ip, endpoint.channel_0_local_port),
        remote_addr_a: socket_addr(remote_ip, endpoint.channel_0_remote_port),
        local_addr_b: socket_addr(local_ip, endpoint.channel_1_local_port),
        remote_addr_b: socket_addr(remote_ip, endpoint.channel_1_remote_port),
        sender_id: endpoint.sender_id,
        remote_id: endpoint.remote_id,
        run_seconds,
        trace,
    })
}

fn defaults(profile: RuntimeProfile) -> EndpointDefaults {
    match profile {
        RuntimeProfile::Academic => EndpointDefaults {
            channel_0_local_port: 5001,
            channel_0_remote_port: 5000,
            channel_1_local_port: 6001,
            channel_1_remote_port: 6000,
            sender_id: 0x5678,
            remote_id: 0x1234,
        },
        RuntimeProfile::LibrastaLocal => EndpointDefaults {
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

fn parse_run_seconds(value: &str) -> Result<u64, &'static str> {
    let seconds = value.parse::<u64>().map_err(|_| "invalid run seconds")?;
    if seconds == 0 || seconds > MAX_RUN_SECONDS {
        return Err("invalid run seconds");
    }
    Ok(seconds)
}

#[cfg(test)]
mod tests {
    use super::{RuntimeProfile, parse_settings};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_default_interlocking_settings() {
        let settings = parse_settings(&args(&["interlocking-controller", "127.0.0.1"])).unwrap();
        assert_eq!(settings.profile, RuntimeProfile::Academic);
        assert_eq!(settings.local_addr_a, "0.0.0.0:5001");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:5000");
        assert_eq!(settings.sender_id, 0x5678);
        assert_eq!(settings.remote_id, 0x1234);
    }

    #[test]
    fn parses_librasta_local_interlocking_settings() {
        let settings = parse_settings(&args(&[
            "interlocking-controller",
            "127.0.0.1",
            "--profile",
            "librasta-local",
            "--run-seconds",
            "5",
            "--trace-wire",
        ]))
        .unwrap();
        assert_eq!(settings.profile, RuntimeProfile::LibrastaLocal);
        assert_eq!(settings.local_addr_a, "0.0.0.0:8888");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:9998");
        assert_eq!(settings.run_seconds, 5);
        assert!(settings.trace);
    }
}
