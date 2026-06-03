//! Regression tests for the specific bugs and footguns called out in the brief:
//! the exponent-suffix hang, the `c/s` case-sensitivity rules, annotations,
//! reciprocals, the unity, and conversion accuracy (including affine).

use ucum::{Dimension, UcumError, analyze, canonical, convert, is_comparable, validate};

const LEN: Dimension = Dimension([1, 0, 0, 0, 0, 0, 0]);
const MASS: Dimension = Dimension([0, 0, 1, 0, 0, 0, 0]);
const TIME_INV: Dimension = Dimension([0, -1, 0, 0, 0, 0, 0]);

fn approx(got: f64, exp: f64) {
    let tol = 1e-9 * exp.abs().max(1.0);
    assert!(
        (got - exp).abs() <= tol,
        "expected {exp}, got {got} (tol {tol})"
    );
}

// --- The hang regressions (the bug that disqualified octofhir-ucum-core) ---

#[test]
fn exponent_suffix_units_analyze_promptly() {
    assert_eq!(
        analyze("m2").unwrap().dimension,
        Dimension([2, 0, 0, 0, 0, 0, 0])
    );
    assert_eq!(
        analyze("m3").unwrap().dimension,
        Dimension([3, 0, 0, 0, 0, 0, 0])
    );
    assert_eq!(
        analyze("m3/s").unwrap().dimension,
        Dimension([3, -1, 0, 0, 0, 0, 0])
    );
    assert_eq!(analyze("s-1").unwrap().dimension, TIME_INV);
}

#[test]
fn unity_is_dimensionless() {
    let a = analyze("1").unwrap();
    assert!(a.is_dimensionless);
    assert_eq!(a.dimension, Dimension::DIMENSIONLESS);
    assert_eq!(a.factor, 1.0);
    assert!(validate("1").is_ok());
}

// --- Footguns ---

#[test]
fn ft_is_femtotonne_not_foot() {
    // `ft` = femto (f) × tonne (t) → a mass.
    assert_eq!(analyze("ft").unwrap().dimension, MASS);
    // The international foot is `[ft_i]` → a length.
    assert_eq!(analyze("[ft_i]").unwrap().dimension, LEN);
}

#[test]
fn case_sensitivity() {
    assert!(validate("m").is_ok()); // meter
    assert!(validate("M").is_err()); // mega prefix alone is not a unit
    assert!(validate("Pa").is_ok()); // pascal
    // `PA` is *not* pascal: it parses as peta-ampere (`A` = ampère), a current.
    assert_ne!(
        analyze("PA").unwrap().dimension,
        analyze("Pa").unwrap().dimension
    );
    // `m` (meter) and `M`-prefixed differ; check a prefixed atom resolves.
    approx(
        analyze("kPa").unwrap().factor,
        analyze("Pa").unwrap().factor * 1e3,
    );
}

#[test]
fn annotations_are_dimensionless() {
    let kg = analyze("kg").unwrap();
    let kg_wet = analyze("kg{wet}").unwrap();
    assert_eq!(kg.dimension, kg_wet.dimension);
    approx(kg_wet.factor, kg.factor);

    // A bare annotation is unity.
    let rbc = analyze("{rbc}").unwrap();
    assert!(rbc.is_dimensionless);
    approx(rbc.factor, 1.0);

    // Non-ASCII annotation content is rejected.
    assert!(validate("kg{wét}").is_err());
}

#[test]
fn leading_slash_is_reciprocal() {
    assert_eq!(analyze("/s").unwrap().dimension, TIME_INV);
    approx(convert(1.0, "/min", "/s").unwrap(), 1.0 / 60.0);
}

#[test]
fn prefix_only_on_metric_atoms() {
    // `[ft_i]` is non-metric; a prefix must not attach.
    assert!(validate("k[ft_i]").is_err());
    // `g` is metric; `kg` is kilo-gram.
    assert!(validate("kg").is_ok());
}

// --- Conversion accuracy ---

#[test]
fn conversion_accuracy() {
    approx(convert(1.0, "[ft_i]", "m").unwrap(), 0.3048);
    approx(convert(1.0, "[in_i]", "cm").unwrap(), 2.54);
    approx(convert(1.0, "bar", "Pa").unwrap(), 1e5);
    approx(convert(1.0, "L/s", "m3/s").unwrap(), 1e-3);
    approx(convert(1.0, "mg/L", "kg/m3").unwrap(), 1e-3);
    approx(convert(1.0, "[gal_us]", "L").unwrap(), 3.785411784);
    approx(convert(1.0, "min", "s").unwrap(), 60.0);
    approx(convert(1.0, "N", "kg.m/s2").unwrap(), 1.0);
}

#[test]
fn affine_temperature_conversion() {
    approx(convert(0.0, "Cel", "K").unwrap(), 273.15);
    approx(convert(100.0, "Cel", "K").unwrap(), 373.15);
    approx(convert(273.15, "K", "Cel").unwrap(), 0.0);
    approx(convert(32.0, "[degF]", "Cel").unwrap(), 0.0);
    approx(convert(212.0, "[degF]", "Cel").unwrap(), 100.0);
    approx(convert(0.0, "Cel", "[degF]").unwrap(), 32.0);

    // Réaumur: 0 °Ré = 0 °C, 80 °Ré = 100 °C (covers the `degRe` affine arm).
    approx(convert(0.0, "[degRe]", "Cel").unwrap(), 0.0);
    approx(convert(80.0, "[degRe]", "Cel").unwrap(), 100.0);
    approx(convert(100.0, "Cel", "[degRe]").unwrap(), 80.0);
}

#[test]
fn roundtrip_conversions() {
    for (a, b) in [
        ("[ft_i]", "m"),
        ("bar", "Pa"),
        ("Cel", "K"),
        ("[lb_av]", "g"),
    ] {
        let there = convert(7.5, a, b).unwrap();
        let back = convert(there, b, a).unwrap();
        approx(back, 7.5);
    }
}

#[test]
fn logarithmic_units_convert() {
    // Bel/decibel/neper are ratios via lg/ln; pH is -log10 of a concentration.
    approx(convert(2.0, "B", "1").unwrap(), 100.0); // 2 B = 10^2
    approx(convert(20.0, "dB", "1").unwrap(), 100.0); // deci folds into the arg
    approx(convert(1.0, "Np", "1").unwrap(), std::f64::consts::E);
    approx(convert(100.0, "1", "B").unwrap(), 2.0);
    approx(convert(7.0, "[pH]", "mol/L").unwrap(), 1e-7);
    assert!(analyze("[pH]").unwrap().is_special);
}

#[test]
fn arbitrary_and_compound_specials_are_not_convertible() {
    // Arbitrary units are commensurable with nothing — not even unity.
    assert!(matches!(
        convert(1.0, "[iU]", "1"),
        Err(UcumError::UnsupportedSpecial { .. })
    ));
    assert!(!is_comparable("[iU]", "1").unwrap());
    assert!(!is_comparable("[iU]", "[iU]").unwrap());
    // A special unit inside a compound term loses its meaning.
    assert!(matches!(
        convert(1.0, "Cel/s", "K/s"),
        Err(UcumError::UnsupportedSpecial { .. })
    ));
    // But all of these still parse, validate, and analyze.
    assert!(validate("dB").is_ok());
    assert!(validate("[iU]").is_ok());
    assert!(validate("Cel/s").is_ok());
}

#[test]
fn incommensurable_units_error() {
    assert!(matches!(
        convert(1.0, "kg", "m"),
        Err(UcumError::NotComparable { .. })
    ));
    assert!(!is_comparable("kg", "m").unwrap());
    assert!(is_comparable("km", "[ft_i]").unwrap());
}

#[test]
fn canonical_display() {
    assert_eq!(canonical("kg.m/s2").unwrap(), "kg.m/s2");
    assert_eq!(canonical("/s").unwrap(), "/s");
    assert_eq!(canonical("m3.s-1").unwrap(), "m3.s-1");

    // Redundant parentheses are dropped...
    assert_eq!(canonical("(m/s)").unwrap(), "m/s");
    assert_eq!(canonical("((m))").unwrap(), "m");
    assert_eq!(canonical("(kg)").unwrap(), "kg");
    // ...but parentheses needed to preserve grouping are kept.
    assert_eq!(canonical("kg/(m.s)").unwrap(), "kg/(m.s)");
    assert_eq!(canonical("s/(m/s)").unwrap(), "s/(m/s)");
}

// --- Determinism & thread-safety ---

#[test]
fn concurrent_calls_are_consistent() {
    use std::thread;
    let handles: Vec<_> = (0..16)
        .map(|_| {
            thread::spawn(|| {
                let a = analyze("kg.m/s2").unwrap();
                (a.dimension, a.factor)
            })
        })
        .collect();
    let reference = {
        let a = analyze("kg.m/s2").unwrap();
        (a.dimension, a.factor)
    };
    for h in handles {
        assert_eq!(h.join().unwrap(), reference);
    }
}
