// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
//! Regenerates `man/lwoodz.1` and `man/lwoodz-cli.1` from the real `clap`
//! definitions in `src/cli.rs`, so the shipped man pages can never drift
//! from the actual CLI surface. Run after changing either `Cli` struct:
//!
//!   cargo run --example gen-man
use clap::CommandFactory;
use lwoodz::cli::{LwoodzArgs, LwoodzCliArgs};
use std::path::Path;

fn write_man(cmd: clap::Command, dest: &Path) -> std::io::Result<()> {
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    std::fs::write(dest, buf)
}

fn main() -> std::io::Result<()> {
    let man_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("man");
    std::fs::create_dir_all(&man_dir)?;

    write_man(LwoodzArgs::command(), &man_dir.join("lwoodz.1"))?;
    write_man(LwoodzCliArgs::command(), &man_dir.join("lwoodz-cli.1"))?;

    println!("Wrote {}", man_dir.join("lwoodz.1").display());
    println!("Wrote {}", man_dir.join("lwoodz-cli.1").display());
    Ok(())
}
