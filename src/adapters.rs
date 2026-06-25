pub mod embedded_ethernet;

#[cfg(feature = "std")]
pub mod socket_transport;
#[cfg(feature = "std")]
pub mod standard_clock;
