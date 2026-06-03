#![no_main]
//! Fuzz target proving totality: feeding arbitrary bytes to the parser and
//! analyser must never panic, abort, or hang.
//!
//! Run a smoke pass with:
//!   cargo +nightly fuzz run parse -- -max_total_time=60
//! Longer campaigns: drop the time limit and let it run; corpus lives under
//! `fuzz/corpus/parse`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let _ = ucum::parse(&s);
    let _ = ucum::validate(&s);
    let _ = ucum::analyze(&s);
    let _ = ucum::canonical(&s);
});
