#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
extern crate std;

pub mod adapters;
pub mod application;
pub mod config;
pub mod core;
pub mod fixed_queue;
pub mod packet_io;
pub mod platform;
pub mod redundancy_crc;
pub mod serial;
pub mod srl;

#[deprecated(note = "renamed to adapters; use rasta_stack::adapters instead")]
pub mod backends {
    pub use crate::adapters::embedded_ethernet as embedded_eth;
    pub use crate::adapters::test as mock_transport;

    #[cfg(feature = "std")]
    pub use crate::adapters::socket_transport as udp_std;
    #[cfg(feature = "std")]
    pub use crate::adapters::standard_clock as std_clock;
    #[cfg(feature = "std")]
    pub use crate::adapters::standard_timer as std_timer;

    #[cfg(all(feature = "std", target_os = "linux"))]
    pub use crate::adapters::linux as tcp_linux;
}
