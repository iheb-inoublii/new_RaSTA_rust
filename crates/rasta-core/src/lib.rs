#![no_std]

/// Temporary marker for the workspace migration.
pub const WORKSPACE_SKELETON_VERSION: u8 = 1;

pub mod io;
pub mod port;
pub mod queue;
pub mod serial;
pub mod time;
