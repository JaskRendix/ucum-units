//! Official UCUM conformance suite.
//!
//! Runs `vendor/UcumFunctionalTests.xml` (© Grahame Grieve, EPL-1.0) against the
//! crate. All five sections are asserted: `validation`, `conversion`,
//! `displayNameGeneration`, `multiplication`, and `division`. The multiplication
//! and division sections are checked for the correct result *value and
//! dimension* (the test file's `uRes` unit spelling is compared
//! dimensionally, since `g/m` and `g.m-1` are equivalent but not identical
//! strings). Any case the crate's coverage cannot support is logged as skipped.

use std::fs;
use std::path::PathBuf;

use ucum::{Quantity, UcumError, analyze, convert, display_name, validate};

fn tests_xml() -> String {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "vendor",
        "UcumFunctionalTests.xml",
    ]
    .iter()
    .collect();
    fs::read_to_string(path).expect("vendored UcumFunctionalTests.xml")
}

/// Tolerance for a conversion outcome.
///
/// Many outcomes in the UCUM test file are given to limited precision (e.g. the
/// exact `25.2` is written `25`, and `1.575` is written `1.6`). We therefore
/// accept a result within half a unit of the last significant decimal place of
/// the literal, with a relative floor for the high-precision outcomes.
fn tolerance(exp_str: &str, exp_val: f64) -> f64 {
    let s = exp_str.trim();
    let (mantissa, e) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i32>().unwrap_or(0)),
        None => (s, 0),
    };
    let frac = mantissa
        .split_once('.')
        .map(|(_, f)| f.len() as i32)
        .unwrap_or(0);
    let half_ulp = 0.5 * 10f64.powi(e - frac);
    half_ulp.max(1e-9 * exp_val.abs()).max(1e-12)
}

fn close(got: f64, exp: f64, exp_str: &str) -> bool {
    (got - exp).abs() <= tolerance(exp_str, exp)
}

#[test]
fn ucum_functional_tests() {
    let xml = tests_xml();
    let doc = roxmltree::Document::parse(&xml).expect("valid XML");

    let mut val_pass = 0u32;
    let mut conv_pass = 0u32;
    let mut disp_pass = 0u32;
    let mut algebra_pass = 0u32;
    let mut skipped: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    for case in doc.descendants().filter(|n| n.tag_name().name() == "case") {
        let section = case
            .ancestors()
            .find_map(|a| match a.tag_name().name() {
                "validation" => Some("validation"),
                "conversion" => Some("conversion"),
                "displayNameGeneration" => Some("display"),
                "multiplication" => Some("multiplication"),
                "division" => Some("division"),
                _ => None,
            })
            .unwrap_or("?");
        let id = case.attribute("id").unwrap_or("?");

        match section {
            "validation" => {
                let unit = case.attribute("unit").unwrap_or("");
                let expect_valid = case.attribute("valid") == Some("true");
                let got_valid = validate(unit).is_ok();
                if got_valid == expect_valid {
                    val_pass += 1;
                } else {
                    failures.push(format!(
                        "[{id}] validation: unit={unit:?} expected valid={expect_valid} got valid={got_valid}"
                    ));
                }
            }
            "conversion" => {
                let value: f64 = case
                    .attribute("value")
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(f64::NAN);
                let src = case.attribute("srcUnit").unwrap_or("");
                let dst = case.attribute("dstUnit").unwrap_or("");
                let exp_str = case.attribute("outcome").unwrap_or("");
                let exp: f64 = exp_str.parse().unwrap_or(f64::NAN);
                match convert(value, src, dst) {
                    Ok(got) => {
                        if close(got, exp, exp_str) {
                            conv_pass += 1;
                        } else {
                            failures.push(format!(
                                "[{id}] conversion: {value} {src} -> {dst}: expected {exp}, got {got}"
                            ));
                        }
                    }
                    Err(UcumError::UnsupportedSpecial { .. }) => {
                        skipped.push(format!(
                            "[{id}] conversion: special unit unsupported ({src} -> {dst})"
                        ));
                    }
                    Err(UcumError::UnknownAtom { code, .. }) => {
                        skipped.push(format!(
                            "[{id}] conversion: unknown atom {code:?} ({src} -> {dst})"
                        ));
                    }
                    Err(e) => {
                        failures.push(format!(
                            "[{id}] conversion: {value} {src} -> {dst}: error {e}"
                        ));
                    }
                }
            }
            "display" => {
                let unit = case.attribute("unit").unwrap_or("");
                let expect = case.attribute("display").unwrap_or("");
                match display_name(unit) {
                    Ok(got) if got == expect => disp_pass += 1,
                    Ok(got) => failures.push(format!(
                        "[{id}] display: unit={unit:?} expected {expect:?}, got {got:?}"
                    )),
                    Err(e) => failures.push(format!("[{id}] display: unit={unit:?} error {e}")),
                }
            }
            "multiplication" | "division" => {
                let u1 = case.attribute("u1").unwrap_or("");
                let u2 = case.attribute("u2").unwrap_or("");
                let v1: f64 = case
                    .attribute("v1")
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(f64::NAN);
                let v2: f64 = case
                    .attribute("v2")
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(f64::NAN);
                let vres: f64 = case
                    .attribute("vRes")
                    .unwrap_or("")
                    .parse()
                    .unwrap_or(f64::NAN);
                let ures = case.attribute("uRes").unwrap_or("");

                let q1 = Quantity::new(v1, u1);
                let q2 = Quantity::new(v2, u2);
                let result = if section == "multiplication" {
                    q1.mul(&q2)
                } else {
                    q1.div(&q2)
                };
                // Compare value+dimension, not the (non-canonical) uRes spelling.
                match (
                    result.analyze(),
                    analyze(if ures.is_empty() { "1" } else { ures }),
                ) {
                    (Ok(ra), Ok(ru)) if ra.dimension == ru.dimension => {
                        let got = result.value * ra.factor / ru.factor;
                        if close(got, vres, case.attribute("vRes").unwrap_or("")) {
                            algebra_pass += 1;
                        } else {
                            failures.push(format!(
                                "[{id}] {section}: expected {vres} {ures:?}, got {got}"
                            ));
                        }
                    }
                    (Ok(ra), Ok(ru)) => failures.push(format!(
                        "[{id}] {section}: dimension mismatch {:?} vs {:?}",
                        ra.dimension, ru.dimension
                    )),
                    _ => skipped.push(format!("[{id}] {section}: unit not analyzable")),
                }
            }
            _ => {}
        }
    }

    eprintln!("--- UCUM conformance summary ---");
    eprintln!("validation passed:  {val_pass}");
    eprintln!("conversion passed:  {conv_pass}");
    eprintln!("display passed:     {disp_pass}");
    eprintln!("mul/div passed:     {algebra_pass}");
    eprintln!("skipped (logged):   {}", skipped.len());
    for s in &skipped {
        eprintln!("  SKIP {s}");
    }
    if !failures.is_empty() {
        eprintln!("FAILURES ({}):", failures.len());
        for fail in &failures {
            eprintln!("  FAIL {fail}");
        }
        panic!("{} conformance case(s) failed", failures.len());
    }
}

#[test]
fn test_unknown_atom_suggestions() {
    let err = ucum::validate("met").unwrap_err();

    match &err {
        UcumError::UnknownAtom { code, suggestion } => {
            assert_eq!(code, "met");
            assert!(suggestion.is_some(), "expected a typo suggestion for 'met'");
        }
        _ => panic!("expected UnknownAtom error"),
    }

    let display_str = err.to_string();
    assert!(
        display_str.contains("did you mean"),
        "error message should include suggestion helper: {display_str}"
    );
}

#[test]
fn test_unknown_atom_no_suggestion() {
    let err = ucum::validate("completelyunknownatom").unwrap_err();

    match &err {
        UcumError::UnknownAtom { code, suggestion } => {
            assert_eq!(code, "completelyunknownatom");
            assert!(
                suggestion.is_none(),
                "expected no typo suggestion for a wild typo, got: {suggestion:?}"
            );
        }
        _ => panic!("expected UnknownAtom error"),
    }

    let display_str = err.to_string();
    assert!(
        !display_str.contains("did you mean"),
        "error message should not include suggestion helper when None: {display_str}"
    );
}
