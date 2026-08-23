// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
pub mod dotenv;

use std::path::{Path, PathBuf};

pub fn repo_root(start: &Path) -> PathBuf {
    let mut cur = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if cur.join(".git").exists()
            || cur.join("lwoodz.toml").exists()
            || cur.join("Cargo.toml").exists()
        {
            return cur;
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => return start.to_path_buf(),
        }
    }
}

pub fn current_year() -> i32 {
    chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(2026)
}
