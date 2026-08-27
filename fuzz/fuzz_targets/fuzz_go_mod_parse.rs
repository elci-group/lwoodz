#![no_main]
// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

//! Fuzzes the hand-rolled `go.mod` `require` block parser against arbitrary
//! text. Unlike the Cargo/npm/Python manifest formats, `go.mod` has no
//! off-the-shelf Rust parser lwoodz can lean on — this line-based parser is
//! entirely hand-written, and it runs against `go.mod` files from any repo
//! lwoodz is pointed at, so it needs to degrade to "parsed nothing" rather
//! than panic on malformed input.

use libfuzzer_sys::fuzz_target;
use lwoodz::manifest::go::parse_go_mod;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = parse_go_mod(&text);
});
