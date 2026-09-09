use ucum::{Case, UcumError};

#[test]
fn test_parse_and_canonical() {
    let expr = "kg.m/s2";
    let ast = ucum::parse(expr);
    assert!(ast.is_ok());

    let normalized = ucum::canonical("((kg.m/s2))");
    assert!(normalized.is_ok());
    assert_eq!(normalized.unwrap(), "kg.m/s2");
}

#[test]
fn test_validation() {
    assert!(ucum::validate("mg/dL").is_ok());
    assert!(ucum::is_valid("mg/dL"));

    assert!(ucum::validate("invalid_unit_foo").is_err());
    assert!(!ucum::is_valid("invalid_unit_foo"));
}

#[test]
fn test_analysis_and_components() {
    let analysis = ucum::analyze("m2");
    assert!(analysis.is_ok());
    let a = analysis.unwrap();
    assert_eq!(a.dimension, ucum::Dimension([2, 0, 0, 0, 0, 0, 0]));

    let dim = ucum::dimension("cm");
    assert!(dim.is_ok());

    let factor = ucum::factor("km");
    assert!(factor.is_ok());
    assert_eq!(factor.unwrap(), 1000.0);

    let offset = ucum::offset("Cel");
    assert!(offset.is_ok());
}

#[test]
fn test_comparability() {
    let comp = ucum::is_comparable("km", "m");
    assert!(comp.is_ok());
    assert!(comp.unwrap());

    assert!(ucum::comparable("km", "m"));
    assert!(!ucum::comparable("kg", "m"));
    assert!(!ucum::comparable("invalid", "m"));
}

#[test]
fn test_conversion() {
    let res = ucum::convert(1.0, "km", "m");
    assert!(res.is_ok());
    assert!((res.unwrap() - 1000.0).abs() < 1e-9);

    let safe_res = ucum::try_convert(1.0, "km", "m");
    assert_eq!(safe_res, Some(1000.0));

    let failed_res = ucum::try_convert(1.0, "kg", "m");
    assert_eq!(failed_res, None);
}

#[test]
fn test_display_name() {
    let name = ucum::display_name("mm");
    assert!(name.is_ok());
    assert_eq!(name.unwrap(), "(millimeter)");
}

#[test]
fn test_case_insensitive_facade() {
    let ci = ucum::ci();
    assert_eq!(ci.case(), Case::Insensitive);
    assert!(ci.validate("MOL").is_ok());
}

#[test]
fn test_parse_malformed_expressions() {
    // Trailing operator
    assert!(matches!(ucum::parse("m/"), Err(UcumError::Parse { .. })));

    // Empty parentheses
    assert!(matches!(ucum::parse("()"), Err(UcumError::Parse { .. })));

    // Invalid nesting
    assert!(ucum::parse("(m/s").is_err());
}

#[test]
fn test_unknown_atom_validation() {
    let err = ucum::validate("flurble");
    assert!(matches!(err, Err(UcumError::UnknownAtom { code }) if code == "flurble"));
}

#[test]
fn test_arbitrary_units_are_not_comparable() {
    // [arb] is an arbitrary UCUM unit
    assert!(!ucum::comparable("[arb]", "[arb]"));
    assert!(!ucum::comparable("[arb]", "m"));
}

#[test]
fn test_special_units_inside_compound_are_rejected() {
    // Special units cannot appear inside compound expressions
    let err = ucum::convert(1.0, "Cel.m", "K.m");
    assert!(matches!(err, Err(UcumError::UnsupportedSpecial { .. })));
}

#[test]
fn test_affine_conversion_temperature() {
    // Celsius → Kelvin
    let k = ucum::convert(0.0, "Cel", "K").unwrap();
    assert!((k - 273.15).abs() < 1e-9);

    // Kelvin → Celsius
    let c = ucum::convert(273.15, "K", "Cel").unwrap();
    assert!((c - 0.0).abs() < 1e-9);
}

#[test]
fn test_logarithmic_units() {
    // bels: log scale
    let res = ucum::convert(2.0, "B", "1").unwrap();
    assert!((res - 100.0).abs() < 1e-9);

    // decibels (20 dB = 2 B -> 10^2 = 100.0)
    let res_db = ucum::convert(20.0, "dB", "1").unwrap();
    assert!((res_db - 100.0).abs() < 1e-9);
}

#[test]
fn test_case_insensitive_behavior() {
    let ci = ucum::ci();

    // "mol" and "MOL" resolve in case-insensitive mode
    assert!(ci.validate("mol").is_ok());
    assert!(ci.validate("MOL").is_ok());

    // But not in case-sensitive mode
    assert!(ucum::validate("MOL").is_err());
}

#[test]
fn test_canonical_parentheses_behavior() {
    // Redundant parentheses removed
    assert_eq!(ucum::canonical("(((m)))").unwrap(), "m");

    // Necessary parentheses preserved
    assert_eq!(ucum::canonical("kg/(m.s)").unwrap(), "kg/(m.s)");

    // Unknown atoms still round-trip
    assert_eq!(ucum::canonical("flurble").unwrap(), "flurble");
}

#[test]
fn test_dimensionless_units() {
    let a = ucum::analyze("1").unwrap();
    assert!(a.is_dimensionless);

    let dim = ucum::dimension("1").unwrap();
    assert_eq!(dim, ucum::Dimension([0, 0, 0, 0, 0, 0, 0]));
}

#[test]
fn test_multiple_prefixes_and_units() {
    // km → m
    let km_to_m = ucum::convert(1.0, "km", "m").unwrap();
    assert!((km_to_m - 1000.0).abs() < 1e-9);

    // mm → m
    let mm_to_m = ucum::convert(1000.0, "mm", "m").unwrap();
    assert!((mm_to_m - 1.0).abs() < 1e-9);

    // µm → m
    let um_to_m = ucum::convert(1_000_000.0, "um", "m").unwrap();
    assert!((um_to_m - 1.0).abs() < 1e-9);
}

#[test]
fn test_try_convert_edge_cases() {
    // Not comparable
    assert_eq!(ucum::try_convert(1.0, "kg", "m"), None);

    // Unknown atom
    assert_eq!(ucum::try_convert(1.0, "flurble", "m"), None);

    // Special unit inside compound
    assert_eq!(ucum::try_convert(1.0, "Cel.m", "K.m"), None);
}

#[test]
fn test_display_name_edge_cases() {
    // Powers
    assert_eq!(ucum::display_name("rad2").unwrap(), "(radian ^ 2)");

    // Dimensionless
    assert_eq!(ucum::display_name("1").unwrap(), "1");

    // Unknown atom → returns an error
    assert!(ucum::display_name("flurble").is_err());
}
