// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
pub mod assessment;
pub mod audit;
pub mod cli;
pub mod config;
pub mod daemon;
pub mod generate;
pub mod inference;
pub mod license;
pub mod manifest;
pub(crate) mod telemetry;
pub mod util;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
