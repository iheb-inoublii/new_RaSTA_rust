#![no_std]

#[cfg(test)]
extern crate std;

/// Temporary marker for the workspace migration.
pub const WORKSPACE_SKELETON_VERSION: u8 = 1;

pub mod config;
pub mod connection;
pub mod io;
pub mod port;
pub mod queue;
pub mod redundancy;
pub mod serial;
pub mod service;
pub mod srl;
pub mod time;

#[cfg(test)]
mod tests;
