//! Tests for the case-insensitive mode, display-name generation, and the
//! Quantity arithmetic layer.

use ucum::{Quantity, Ucum, display_name};

fn approx(got: f64, exp: f64) {
    let tol = 1e-9 * exp.abs().max(1.0);
    assert!((got - exp).abs() <= tol, "expected {exp}, got {got}");
}

// --- c/i (case-insensitive) mode ---

#[test]
fn case_insensitive_mode() {
    let ci = Ucum::case_insensitive();
    // Mole's c/i code is MOL; meter is M; liter is L.
    assert!(ci.validate("MOL").is_ok());
    assert!(ci.validate("mol").is_ok());
    assert!(ci.validate("L").is_ok());
    assert!(ci.validate("l").is_ok());
    // Prefixed: kilo (K) + pascal (PAL) = kilopascal.
    assert!(ci.validate("KPAL").is_ok());
    // Conversion works through the c/i tables.
    approx(ci.convert(1.0, "M", "CM").unwrap(), 100.0);
    approx(ci.convert(1.0, "KG", "G").unwrap(), 1000.0);

    // The case-sensitive facade rejects the upper-case spellings.
    let cs = Ucum::case_sensitive();
    assert!(cs.validate("MOL").is_err());
}

// --- display-name generation ---

#[test]
fn display_names() {
    assert_eq!(display_name("").unwrap(), "(unity)");
    assert_eq!(display_name("m").unwrap(), "(meter)");
    assert_eq!(display_name("mm").unwrap(), "(millimeter)");
    assert_eq!(display_name("m[H2O]").unwrap(), "(meter of water column)");
    assert_eq!(
        display_name("10*23").unwrap(),
        "(the number ten for arbitrary powers ^ 23)"
    );
    assert_eq!(display_name("rad2").unwrap(), "(radian ^ 2)");
    assert_eq!(
        display_name("m3.kg-1.s-2").unwrap(),
        "(meter ^ 3) * (kilogram ^ -1) * (second ^ -2)"
    );
    assert_eq!(display_name("Pa").unwrap(), "(pascal)");
    assert!(display_name("flurble").is_err());
}

// --- Quantity arithmetic ---

#[test]
fn quantity_arithmetic() {
    let a = Quantity::new(1.5, "g");
    let b = Quantity::new(2.0, "m");
    let prod = a.mul(&b);
    assert_eq!(prod.value, 3.0);
    assert!(prod.is_comparable("g.m").unwrap());

    let quot = a.div(&b);
    assert_eq!(quot.value, 0.75);
    assert!(quot.is_comparable("g/m").unwrap());

    // Commensurable division collapses to a dimensionless ratio.
    let r = Quantity::new(1.0, "[lb_av]/h").div(&Quantity::new(1.0, "kg/s"));
    assert!(r.dimension().unwrap().is_dimensionless());
    let in_unity = r.value * r.analyze().unwrap().factor;
    approx(in_unity, 0.000_125_997_880_555_56);

    // convert_to round-trips a quantity into another unit.
    let c = Quantity::new(2.0, "[ft_i]").convert_to("m").unwrap();
    approx(c.value, 0.6096);
    assert_eq!(c.unit, "m");
}
