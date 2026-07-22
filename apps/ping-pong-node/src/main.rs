use rasta_core::application::ApplicationMessage;
use rasta_core::config::RastaProfile;
use rasta_core::endpoint::{ConnectionStatus, RastaEndpoint, config_from_profile};
use rasta_platform::std_clock::StdClock;
use rasta_platform::udp::UdpSocketTransport;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_RUN_SECONDS: u64 = 30;
const DEFAULT_WARMUP: u32 = 100;
const DEFAULT_ROUNDS: u32 = 5_000;
const DEFAULT_CSV_PATH: &str = "ping-pong-rtt.csv";
const DEFAULT_PING_DELAY_MS: u64 = 0;
const SBB_LOCAL_PING_DELAY_MS: u64 = 300;
const PONG_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PING_DELAY_MS: u64 = 60_000;
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
    SbbLocal,
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
    warmup: u32,
    rounds: u32,
    csv_path: PathBuf,
    quiet: bool,
    run_seconds: u64,
    ping_delay_ms: u64,
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
                "Usage: {} <active|passive> <remote_ip> [--warmup N] [--rounds N] [--csv PATH] [--quiet] [--run-seconds N] [--ping-delay-ms N] [--profile academic|librasta-local|sbb-local] [--trace|--trace-wire] [channel port overrides]",
                args[0]
            );
            return;
        }
        Err(_) => {
            println!("Invalid arguments. Use active/passive, a remote IP, and valid options.");
            return;
        }
    };

    if !settings.quiet {
        println!("Starting ping-pong-node {:?}", settings.role);
        println!("Local ID: {}", settings.sender_id);
        println!("Remote ID: {}", settings.remote_id);
        println!("Profile: {:?}", settings.profile);
        println!("Warm-up rounds: {}", settings.warmup);
        println!("Measured rounds: {}", settings.rounds);
        println!("Run seconds: {}", settings.run_seconds);
        println!("Ping delay ms: {}", settings.ping_delay_ms);
        println!(
            "Channel A: {} -> {}",
            settings.local_addr_a, settings.remote_addr_a
        );
        println!(
            "Channel B: {} -> {}",
            settings.local_addr_b, settings.remote_addr_b
        );
    }

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
    let mut active_state =
        ActivePingPongState::new(Instant::now(), settings.warmup, settings.rounds);
    let ping_delay = Duration::from_millis(settings.ping_delay_ms);

    loop {
        if let Err(error) = endpoint.poll() {
            if settings.role == Role::Passive && !settings.quiet {
                eprintln!("Error during poll: {error}");
            }
            break;
        }
        let measuring = settings.role == Role::Active && active_state.is_measuring();
        drain_trace(
            &mut endpoint,
            settings.trace && !settings.quiet && !measuring,
        );

        let status = endpoint.status();
        if status != last_status && !settings.quiet && !measuring {
            println!("State transition: {:?} -> {:?}", last_status, status);
        }
        last_status = status;
        if status == ConnectionStatus::Up && up_since.is_none() {
            up_since = Some(Instant::now());
        }

        if status == ConnectionStatus::Up {
            match settings.role {
                Role::Active => run_active(&mut endpoint, &mut active_state, ping_delay),
                Role::Passive => run_passive(&mut endpoint),
            }
        }

        if settings.role == Role::Active && active_state.is_complete() {
            let _ = endpoint.close();
            break;
        }

        if start.elapsed() >= Duration::from_secs(settings.run_seconds) {
            if settings.role == Role::Active {
                let _ = endpoint.close();
            }
            break;
        }

        if status != ConnectionStatus::Up {
            thread::sleep(Duration::from_millis(10));
        }
    }

    if settings.role == Role::Active {
        let _ = write_csv(&settings.csv_path, active_state.rtts());
        let successful_rounds = active_state.rtts().len();
        println!(
            "benchmark summary: warmup_rounds={} measured_rounds={} successful_rounds={} failed_or_timed_out_rounds={} csv_output_path={}",
            settings.warmup,
            settings.rounds,
            successful_rounds,
            settings.rounds as usize - successful_rounds,
            settings.csv_path.display()
        );
    }
}

#[derive(Clone, Debug)]
struct ActivePingPongState {
    next_ping: u32,
    warmup: u32,
    rounds: u32,
    sent_at: Option<Instant>,
    next_ping_at: Instant,
    rtts_ns: Vec<u64>,
}

impl ActivePingPongState {
    fn new(now: Instant, warmup: u32, rounds: u32) -> Self {
        Self {
            next_ping: 1,
            warmup,
            rounds,
            sent_at: None,
            next_ping_at: now,
            rtts_ns: Vec::with_capacity(rounds as usize),
        }
    }

    fn total_rounds(&self) -> u32 {
        self.warmup + self.rounds
    }

    fn should_send_ping(&self, now: Instant) -> bool {
        self.sent_at.is_none() && self.next_ping <= self.total_rounds() && now >= self.next_ping_at
    }

    fn current_ping(&self) -> u32 {
        self.next_ping
    }

    fn record_ping_sent(&mut self, sent_at: Instant) {
        self.sent_at = Some(sent_at);
    }

    fn record_expected_pong(&mut self, counter: u32, now: Instant, ping_delay: Duration) -> bool {
        if counter != self.next_ping {
            return false;
        }

        let Some(sent_at) = self.sent_at.take() else {
            return false;
        };
        if counter > self.warmup {
            let nanos = now.duration_since(sent_at).as_nanos();
            self.rtts_ns.push(nanos.min(u128::from(u64::MAX)) as u64);
        }
        self.advance(now, ping_delay);
        true
    }

    fn record_failed_round(&mut self, now: Instant, ping_delay: Duration) {
        self.sent_at = None;
        self.advance(now, ping_delay);
    }

    fn advance(&mut self, now: Instant, ping_delay: Duration) {
        self.next_ping = self.next_ping.saturating_add(1);
        self.next_ping_at = now + ping_delay;
    }

    fn pong_timed_out(&self, now: Instant) -> bool {
        self.sent_at
            .is_some_and(|sent_at| now.duration_since(sent_at) >= PONG_TIMEOUT)
    }

    fn is_measuring(&self) -> bool {
        self.next_ping > self.warmup && self.next_ping <= self.total_rounds()
    }

    fn is_complete(&self) -> bool {
        self.next_ping > self.total_rounds()
    }

    fn rtts(&self) -> &[u64] {
        &self.rtts_ns
    }
}

fn run_active<T1, T2, C>(
    endpoint: &mut RastaEndpoint<T1, T2, C>,
    state: &mut ActivePingPongState,
    ping_delay: Duration,
) where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    let now = Instant::now();
    if state.pong_timed_out(now) {
        state.record_failed_round(now, ping_delay);
    }

    if state.should_send_ping(now) {
        let counter = state.current_ping();
        let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
        if let Ok(length) = (ApplicationMessage::Ping { counter }).encode(&mut buffer) {
            let sent_at = Instant::now();
            let send_result = endpoint.send(&buffer[..length]);
            state.record_ping_sent(sent_at);
            if send_result.is_err() {
                state.record_failed_round(Instant::now(), ping_delay);
            }
        }
    }

    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    while endpoint.has_received_data() {
        match endpoint.receive(&mut buffer) {
            Ok(length) => {
                if let Ok(ApplicationMessage::Pong { counter }) =
                    ApplicationMessage::decode(&buffer[..length])
                {
                    let _ = state.record_expected_pong(counter, Instant::now(), ping_delay);
                }
            }
            Err(_) => break,
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
                    let _ = send_message(endpoint, ApplicationMessage::Pong { counter });
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

fn send_message<T1, T2, C>(
    endpoint: &mut RastaEndpoint<T1, T2, C>,
    message: ApplicationMessage,
) -> Result<(), ()>
where
    T1: rasta_core::port::RastaTransport,
    T2: rasta_core::port::RastaTransport,
    C: rasta_core::time::MonotonicClock + rasta_core::time::ProtocolTimestampSource,
{
    let mut buffer = [0u8; ApplicationMessage::MAX_ENCODED_LEN];
    let length = message.encode(&mut buffer).map_err(|_| ())?;
    endpoint.send(&buffer[..length]).map_err(|_| ())
}

fn write_csv(path: &Path, rtts_ns: &[u64]) -> std::io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    writeln!(writer, "round,rtt_ns,rtt_us,rtt_ms")?;
    for (index, rtt_ns) in rtts_ns.iter().enumerate() {
        writeln!(
            writer,
            "{},{},{:.3},{:.6}",
            index + 1,
            rtt_ns,
            *rtt_ns as f64 / 1_000.0,
            *rtt_ns as f64 / 1_000_000.0
        )?;
    }
    writer.flush()
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
        RuntimeProfile::SbbLocal => RastaProfile::sbb_local()?,
    };
    let config = config_from_profile(
        settings.sender_id,
        settings.remote_id,
        profile,
        matches!(
            settings.profile,
            RuntimeProfile::LibrastaLocal | RuntimeProfile::SbbLocal
        ),
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
    let mut warmup = DEFAULT_WARMUP;
    let mut rounds = DEFAULT_ROUNDS;
    let mut csv_path = PathBuf::from(DEFAULT_CSV_PATH);
    let mut quiet = false;
    let mut run_seconds = DEFAULT_RUN_SECONDS;
    let mut ping_delay_ms = default_ping_delay_ms(profile);
    let mut trace = false;
    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                index += 1;
                profile = parse_profile(args.get(index).ok_or("missing option value")?)?;
                endpoint = defaults(role, profile);
                ping_delay_ms = default_ping_delay_ms(profile);
                index += 1;
            }
            "--rounds" => {
                index += 1;
                rounds = parse_rounds(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--warmup" => {
                index += 1;
                warmup = parse_warmup(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--csv" => {
                index += 1;
                csv_path = parse_csv_path(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--quiet" => {
                quiet = true;
                index += 1;
            }
            "--run-seconds" => {
                index += 1;
                run_seconds = parse_run_seconds(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--ping-delay-ms" => {
                index += 1;
                ping_delay_ms =
                    parse_ping_delay_ms(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--trace" | "--trace-wire" => {
                trace = true;
                index += 1;
            }
            "--channel-0-local-port" => {
                index += 1;
                endpoint.channel_0_local_port =
                    parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-0-remote-port" => {
                index += 1;
                endpoint.channel_0_remote_port =
                    parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-1-local-port" => {
                index += 1;
                endpoint.channel_1_local_port =
                    parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            "--channel-1-remote-port" => {
                index += 1;
                endpoint.channel_1_remote_port =
                    parse_port(args.get(index).ok_or("missing option value")?)?;
                index += 1;
            }
            _ => return Err("invalid option"),
        }
    }
    if endpoint.channel_0_local_port == endpoint.channel_1_local_port {
        return Err("duplicate local ports");
    }
    if warmup.checked_add(rounds).is_none() {
        return Err("too many total rounds");
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
        warmup,
        rounds,
        csv_path,
        quiet,
        run_seconds,
        ping_delay_ms,
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
        (Role::Active, RuntimeProfile::SbbLocal) => EndpointDefaults {
            channel_0_local_port: 7100,
            channel_0_remote_port: 7000,
            channel_1_local_port: 7101,
            channel_1_remote_port: 7001,
            sender_id: 0x61,
            remote_id: 0x62,
        },
        (Role::Passive, RuntimeProfile::SbbLocal) => EndpointDefaults {
            channel_0_local_port: 7000,
            channel_0_remote_port: 7100,
            channel_1_local_port: 7001,
            channel_1_remote_port: 7101,
            sender_id: 0x62,
            remote_id: 0x61,
        },
    }
}

fn parse_profile(value: &str) -> Result<RuntimeProfile, &'static str> {
    match value {
        "academic" => Ok(RuntimeProfile::Academic),
        "librasta-local" => Ok(RuntimeProfile::LibrastaLocal),
        "sbb-local" => Ok(RuntimeProfile::SbbLocal),
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

fn parse_warmup(value: &str) -> Result<u32, &'static str> {
    value.parse::<u32>().map_err(|_| "invalid warmup")
}

fn parse_csv_path(value: &str) -> Result<PathBuf, &'static str> {
    if value.is_empty() {
        return Err("invalid csv path");
    }
    Ok(PathBuf::from(value))
}

fn parse_run_seconds(value: &str) -> Result<u64, &'static str> {
    let seconds = value.parse::<u64>().map_err(|_| "invalid run seconds")?;
    if seconds == 0 || seconds > MAX_RUN_SECONDS {
        return Err("invalid run seconds");
    }
    Ok(seconds)
}

fn default_ping_delay_ms(profile: RuntimeProfile) -> u64 {
    match profile {
        RuntimeProfile::Academic | RuntimeProfile::LibrastaLocal => DEFAULT_PING_DELAY_MS,
        RuntimeProfile::SbbLocal => SBB_LOCAL_PING_DELAY_MS,
    }
}

fn parse_ping_delay_ms(value: &str) -> Result<u64, &'static str> {
    let delay = value.parse::<u64>().map_err(|_| "invalid ping delay ms")?;
    if delay > MAX_PING_DELAY_MS {
        return Err("invalid ping delay ms");
    }
    Ok(delay)
}

fn parse_port(value: &str) -> Result<u16, &'static str> {
    value.parse::<u16>().map_err(|_| "invalid port")
}

#[cfg(test)]
mod tests {
    use super::{
        ActivePingPongState, DEFAULT_CSV_PATH, DEFAULT_PING_DELAY_MS, DEFAULT_ROUNDS,
        DEFAULT_WARMUP, Role, RuntimeProfile, SBB_LOCAL_PING_DELAY_MS, parse_settings, write_csv,
    };
    use std::fs;
    use std::time::{Duration, Instant};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_active_defaults() {
        let settings = parse_settings(&args(&["ping-pong-node", "active", "127.0.0.1"])).unwrap();
        assert_eq!(settings.role, Role::Active);
        assert_eq!(settings.profile, RuntimeProfile::Academic);
        assert_eq!(settings.warmup, DEFAULT_WARMUP);
        assert_eq!(settings.rounds, DEFAULT_ROUNDS);
        assert_eq!(settings.csv_path.to_string_lossy(), DEFAULT_CSV_PATH);
        assert!(!settings.quiet);
        assert_eq!(settings.ping_delay_ms, DEFAULT_PING_DELAY_MS);
        assert_eq!(settings.local_addr_a, "0.0.0.0:5000");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:5001");
        assert_eq!(settings.sender_id, 0x1234);
        assert_eq!(settings.remote_id, 0x5678);
    }

    #[test]
    fn parses_benchmark_arguments() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "active",
            "127.0.0.1",
            "--warmup",
            "25",
            "--rounds",
            "2000",
            "--csv",
            "/tmp/rasta-rtt.csv",
            "--quiet",
        ]))
        .unwrap();

        assert_eq!(settings.warmup, 25);
        assert_eq!(settings.rounds, 2000);
        assert_eq!(settings.csv_path.to_string_lossy(), "/tmp/rasta-rtt.csv");
        assert!(settings.quiet);
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

    #[test]
    fn parses_active_sbb_local_defaults() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "active",
            "127.0.0.1",
            "--profile",
            "sbb-local",
        ]))
        .unwrap();
        assert_eq!(settings.role, Role::Active);
        assert_eq!(settings.profile, RuntimeProfile::SbbLocal);
        assert_eq!(settings.ping_delay_ms, SBB_LOCAL_PING_DELAY_MS);
        assert_eq!(settings.local_addr_a, "0.0.0.0:7100");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:7000");
        assert_eq!(settings.local_addr_b, "0.0.0.0:7101");
        assert_eq!(settings.remote_addr_b, "127.0.0.1:7001");
        assert_eq!(settings.sender_id, 0x61);
        assert_eq!(settings.remote_id, 0x62);
    }

    #[test]
    fn parses_passive_sbb_local_defaults() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "passive",
            "127.0.0.1",
            "--profile",
            "sbb-local",
        ]))
        .unwrap();
        assert_eq!(settings.role, Role::Passive);
        assert_eq!(settings.profile, RuntimeProfile::SbbLocal);
        assert_eq!(settings.local_addr_a, "0.0.0.0:7000");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:7100");
        assert_eq!(settings.local_addr_b, "0.0.0.0:7001");
        assert_eq!(settings.remote_addr_b, "127.0.0.1:7101");
        assert_eq!(settings.sender_id, 0x62);
        assert_eq!(settings.remote_id, 0x61);
        assert_eq!(settings.ping_delay_ms, SBB_LOCAL_PING_DELAY_MS);
    }

    #[test]
    fn parses_sbb_local_ping_delay_default() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "active",
            "127.0.0.1",
            "--profile",
            "sbb-local",
        ]))
        .unwrap();
        assert_eq!(settings.ping_delay_ms, SBB_LOCAL_PING_DELAY_MS);
    }

    #[test]
    fn parses_explicit_ping_delay_ms() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "active",
            "127.0.0.1",
            "--profile",
            "sbb-local",
            "--ping-delay-ms",
            "450",
        ]))
        .unwrap();
        assert_eq!(settings.ping_delay_ms, 450);
    }

    #[test]
    fn rejects_invalid_ping_delay_ms() {
        assert_eq!(
            parse_settings(&args(&[
                "ping-pong-node",
                "active",
                "127.0.0.1",
                "--ping-delay-ms",
                "not-a-number",
            ])),
            Err("invalid ping delay ms")
        );
    }

    #[test]
    fn rejects_duplicate_local_ports() {
        assert_eq!(
            parse_settings(&args(&[
                "ping-pong-node",
                "active",
                "127.0.0.1",
                "--channel-0-local-port",
                "9000",
                "--channel-1-local-port",
                "9000",
            ])),
            Err("duplicate local ports")
        );
    }

    #[test]
    fn parses_explicit_channel_port_overrides() {
        let settings = parse_settings(&args(&[
            "ping-pong-node",
            "active",
            "127.0.0.1",
            "--profile",
            "sbb-local",
            "--channel-0-local-port",
            "8100",
            "--channel-0-remote-port",
            "8000",
            "--channel-1-local-port",
            "8101",
            "--channel-1-remote-port",
            "8001",
        ]))
        .unwrap();
        assert_eq!(settings.local_addr_a, "0.0.0.0:8100");
        assert_eq!(settings.remote_addr_a, "127.0.0.1:8000");
        assert_eq!(settings.local_addr_b, "0.0.0.0:8101");
        assert_eq!(settings.remote_addr_b, "127.0.0.1:8001");
    }

    #[test]
    fn active_ping_pong_state_machine_sends_next_ping_only_after_pong() {
        let now = Instant::now();
        let delay = Duration::from_millis(SBB_LOCAL_PING_DELAY_MS);
        let mut state = ActivePingPongState::new(now, 0, 5);

        assert!(state.should_send_ping(now));
        assert_eq!(state.current_ping(), 1);

        state.record_ping_sent(now);
        assert!(!state.should_send_ping(now + delay));

        assert!(!state.record_expected_pong(2, now, delay));
        assert!(!state.should_send_ping(now + delay));

        assert!(state.record_expected_pong(1, now, delay));
        assert!(!state.should_send_ping(now + delay - Duration::from_millis(1)));
        assert!(state.should_send_ping(now + delay));
        assert_eq!(state.current_ping(), 2);
    }

    #[test]
    fn discards_warmup_and_creates_requested_measured_samples() {
        let start = Instant::now();
        let mut state = ActivePingPongState::new(start, 2, 3);

        for counter in 1..=5 {
            let sent_at = start + Duration::from_millis(u64::from(counter));
            state.record_ping_sent(sent_at);
            assert!(state.record_expected_pong(
                counter,
                sent_at + Duration::from_nanos(u64::from(counter)),
                Duration::ZERO,
            ));
        }

        assert!(state.is_complete());
        assert_eq!(state.rtts(), &[3, 4, 5]);
    }

    #[test]
    fn matches_ping_and_pong_sequence_numbers() {
        let now = Instant::now();
        let mut state = ActivePingPongState::new(now, 0, 1);
        state.record_ping_sent(now);

        assert!(!state.record_expected_pong(2, now + Duration::from_nanos(1), Duration::ZERO));
        assert!(state.record_expected_pong(1, now + Duration::from_nanos(2), Duration::ZERO));
        assert_eq!(state.rtts(), &[2]);
    }

    #[test]
    fn csv_has_expected_header_and_row_count() {
        let path =
            std::env::temp_dir().join(format!("ping-pong-node-{}-rtt.csv", std::process::id()));
        write_csv(&path, &[10, 20, 30]).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        fs::remove_file(path).unwrap();

        let rows: Vec<_> = contents.lines().collect();
        assert_eq!(rows[0], "round,rtt_ns,rtt_us,rtt_ms");
        assert_eq!(rows.len(), 4);
    }
}
