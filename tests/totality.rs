//! Totality tests: `parse`, `validate`, and `analyze` must return on *every*
//! input — no panics, no hangs, no stack overflows.
//!
//! `proptest` fuzzes with random and structured inputs; a thread-with-timeout
//! harness turns any hypothetical hang into a test failure instead of a hung CI
//! job.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use proptest::prelude::*;

/// Run `f` on a worker thread and fail if it does not finish within `secs`.
/// This converts a hang into a deterministic test failure.
fn within<F: FnOnce() + Send + 'static>(secs: u64, f: F) {
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(()) => {
            handle.join().unwrap();
        }
        Err(_) => panic!("call did not terminate within {secs}s — totality violated"),
    }
}

fn exercise(s: &str) {
    let _ = ucum::parse(s);
    let _ = ucum::validate(s);
    let _ = ucum::analyze(s);
    let _ = ucum::canonical(s);
}

#[test]
fn known_pathological_inputs_terminate() {
    let cases: Vec<String> = vec![
        "m2".into(),
        "m3".into(),
        "m3/s".into(),
        "1".into(),
        "/".repeat(10_000),
        ".".repeat(10_000),
        "(".repeat(10_000),
        ")".repeat(10_000),
        "{".repeat(10_000),
        "[".repeat(10_000),
        "m".repeat(50_000),
        "1".repeat(50_000),
        "kg.".repeat(20_000),
        "(((((((((((((((((((((((((((((((((((((((((((((((((".into(),
        "10*".repeat(5_000),
        "\u{0}\u{1}\u{2}".repeat(1_000),
        "m{".to_owned() + &"a".repeat(50_000),
    ];
    for c in cases {
        let c2 = c.clone();
        within(5, move || exercise(&c2));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    /// Arbitrary UTF-8 strings never panic or hang.
    #[test]
    fn arbitrary_strings_are_total(s in ".*") {
        within(5, move || exercise(&s));
    }

    /// Arbitrary byte strings (including invalid UTF-8 handled as lossy) too.
    #[test]
    fn arbitrary_bytes_are_total(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let s = String::from_utf8_lossy(&bytes).into_owned();
        within(5, move || exercise(&s));
    }

    /// Structured inputs built from real UCUM tokens stress the grammar paths.
    #[test]
    fn structured_inputs_are_total(
        tokens in proptest::collection::vec(
            prop_oneof![
                Just("m"), Just("kg"), Just("s"), Just("Pa"), Just("Cel"),
                Just("[ft_i]"), Just("10*"), Just("%"), Just("{x}"), Just("1"),
                Just("2"), Just("-3"), Just("("), Just(")"), Just("."), Just("/"),
                Just("mol"), Just("B"), Just("rad"),
            ],
            0..40,
        )
    ) {
        let s: String = tokens.concat();
        within(5, move || exercise(&s));
    }
}
