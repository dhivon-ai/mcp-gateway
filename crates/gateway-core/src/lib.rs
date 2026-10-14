//! Shared policy and wire contracts for the enterprise MCP gateway.

pub const CONFIG_SCHEMA_VERSION: u32 = 1;
pub const MAX_CONFIG_AGE_SECONDS: i64 = 3_600;
pub const REFRESH_INTERVAL_SECONDS: u64 = 60;

pub mod contracts;
pub mod crypto;
pub mod policy;

pub use contracts::*;
pub use policy::*;
