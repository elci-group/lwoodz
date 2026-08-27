// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! The two binaries' `clap` argument definitions, in one place — so man
//! pages and shell completions can be generated from the real, current
//! definitions (`examples/gen-man.rs`, `lwoodz-cli completions`) instead of
//! hand-authored docs that drift from the actual CLI.
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lwoodz",
    version,
    author = "Elci Group",
    about = "Lwoodz is to license document generation and maintenance what Kaptaind is to version management.\nThe canonical licensing daemon for a repository — governs legal state.",
    long_about = r#"
+-------------------+
|    .-=====-.      |
|   /  .---.  \     |
|  |--< LWDZ >--|    |
|   \  '---'  /     |
|    '---|---'      |
|    ___/ \___      |
|   /_LWOODZ_\    |
+-------------------+

Lwoodz governs a repository's legal state the way Kaptaind governs its temporal state.

  Kaptaind  -> versions,  VERSION, release lifecycle
  Lwoodz    -> licenses,  LICENSE, licensing lifecycle

Features:
  • Detects dependencies and checks license compatibility
  • Generates LICENSE, NOTICE, COPYRIGHT, THIRD_PARTY_NOTICES
  • Inserts/updates copyright + SPDX headers in source files
  • Produces SPDX manifests and machine-readable licensing metadata
  • Audits repositories before release
  • Maintains contributor assignments & DCO
  • Dual-license / commercial-license variants
  • Inference via GROQ_API_KEY for advice and generation

USAGE:
  lwoodz                  Run audit (default)
  lwoodz --daemon         Run as licensing daemon (watch mode)
  lwoodz --generate       Generate licensing files
  lwoodz --audit          Full audit with output
  lwoodz --check          Compatibility check only

ENVIRONMENT:
  GROQ_API_KEY            Groq API key for AI-assisted licensing advice
  LWOODZ_CONFIG           Path to lwoodz.toml (default: ./lwoodz.toml)

CONFIG FILE:
  Default: ./lwoodz.toml
  Generate with: lwoodz-cli init  (or lwoodz --init)
"#
)]
pub struct LwoodzArgs {
    /// Run as background licensing daemon (watches manifests for changes)
    #[arg(long)]
    pub daemon: bool,

    /// Generate LICENSE and related legal documents
    #[arg(long)]
    pub generate: bool,

    /// Run full audit (license file, SPDX, deps, headers)
    #[arg(long)]
    pub audit: bool,

    /// Check dependency license compatibility only
    #[arg(long)]
    pub check: bool,

    /// Initialize lwoodz.toml in the current repository
    #[arg(long)]
    pub init: bool,

    /// Path to lwoodz.toml
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<std::path::PathBuf>,

    /// Override startup guard (daemon)
    #[arg(long)]
    pub force: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Preview generated licensing files without writing them
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Parser)]
#[command(
    name = "lwoodz-cli",
    version,
    about = "CLI companion for the lwoodz licensing daemon"
)]
pub struct LwoodzCliArgs {
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<std::path::PathBuf>,
    /// Output as JSON (must come before the subcommand, e.g. `lwoodz-cli --json audit`)
    #[arg(long)]
    pub json: bool,
    #[command(subcommand)]
    pub command: LwoodzCliCommand,
}

#[derive(Subcommand)]
pub enum LwoodzCliCommand {
    /// Create lwoodz.toml in the current directory
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Generate LICENSE and related legal documents
    Generate {
        #[arg(long)]
        dry_run: bool,
    },
    /// Run a full compliance audit
    Audit,
    /// Check dependency license compatibility only
    Check {
        #[arg(long)]
        strict: bool,
    },
    /// List known SPDX license identifiers
    Licenses,
    /// Check whether two SPDX identifiers are compatible
    Compat { project: String, dep: String },
    /// Detect the SPDX license of a LICENSE file
    Detect {
        #[arg(default_value = "LICENSE")]
        path: std::path::PathBuf,
    },
    /// Explain an SPDX license (uses Groq if GROQ_API_KEY is set)
    Explain { spdx: String },
    /// Insert/update copyright + SPDX headers in source files
    Headers {
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate a dual-licensed variant
    Dual { license: String },
    /// Show daemon and repository status
    Status,
    /// Print the version
    Version,
    /// Print a shell completion script to stdout
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
