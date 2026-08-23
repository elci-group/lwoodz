// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use std::path::Path;

/// Load a local .env file (best-effort, never fails hard).
pub fn load() -> anyhow::Result<()> {
    for name in [".env", ".env.local"] {
        if Path::new(name).exists() {
            let content = std::fs::read_to_string(name)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                    if std::env::var(k).is_err() {
                        // SAFETY: single-threaded at startup; test suite is serialised.
                        unsafe {
                            std::env::set_var(k, v);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
