pub mod error;
pub mod instruction;
pub mod state;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

pub use solana_program;

// solana_program::declare_id!("BanKpA2LBaEfelI3A68m4djNLqgtticKg6CnyNwgAC9");
