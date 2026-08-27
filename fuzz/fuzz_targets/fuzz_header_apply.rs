#![no_main]
// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Fuzzes the pure header insert/update/strip logic against arbitrary file
//! content. `ensure_headers` runs this over every source file in a scanned
//! repository — including ones written by whoever lwoodz is auditing, not
//! lwoodz itself — so it needs to handle any byte sequence without
//! panicking: empty files, files that are all "copyright"/"spdx" noise,
//! truncated shebangs, embedded NULs, non-ASCII line noise, and so on.

use libfuzzer_sys::fuzz_target;
use lwoodz::license::header::{apply_header, HeaderConfig};
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    let content = String::from_utf8_lossy(data);
    let cfg = HeaderConfig {
        holder: "Acme Inc".to_string(),
        year: 2026,
        spdx: "MIT".to_string(),
        insert_spdx: true,
    };
    let _ = apply_header(&content, &cfg, Path::new("fuzz_target.rs"));
});
