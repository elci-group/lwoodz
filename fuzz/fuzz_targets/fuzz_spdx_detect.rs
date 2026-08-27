#![no_main]
// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Fuzzes SPDX text detection and expression parsing.
//!
//! `detect_spdx_from_text` now runs against `LICENSE` files read straight
//! off disk from every ecosystem's dependency tree (Cargo's registry cache,
//! `node_modules`, a Python venv's `dist-info`, a Go module cache) — none of
//! that text is something lwoodz controls the contents of. This target
//! asserts only that arbitrary bytes can never panic the detector or the
//! expression parser, whatever garbage a dependency's LICENSE file contains.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = lwoodz::license::spdx::detect_spdx_from_text(&text);

    let expr = lwoodz::license::spdx::SpdxExpression::parse(text.as_ref());
    let _ = expr.identifiers();
    let _ = expr.is_known();

    let _ = lwoodz::license::spdx::normalize_spdx(&text);
    let _ = lwoodz::license::spdx::is_valid_spdx(&text);
});
