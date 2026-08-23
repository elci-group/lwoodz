// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
pub mod audit;
pub mod cli;
pub mod config;
pub mod daemon;
pub mod generate;
pub mod inference;
pub mod license;
pub mod manifest;
pub mod remedy;
pub mod util;

pub use cli::LwoodzCommand;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
