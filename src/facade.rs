use crate::{Analysis, Dimension, Ucum, UcumError, UnitExpr};

/// Parse a UCUM expression into an abstract syntax tree (case-sensitive).
///
/// This is a *total* function: it never panics and never hangs. Syntactically
/// malformed input yields a [`UcumError::Parse`] carrying the byte offset.
/// Atoms are **not** checked for existence here; use [`validate`] or
/// [`analyze`] for that.
///
/// ```
/// use ucum::UcumError;
/// assert!(ucum::parse("kg.m/s2").is_ok());
/// assert!(ucum::parse("/s").is_ok());        // leading-slash reciprocal
/// assert!(ucum::parse("(m/s)").is_ok());
/// // A malformed expression reports *where* it went wrong.
/// assert!(matches!(ucum::parse("m/"), Err(UcumError::Parse { pos: 2, .. })));
/// ```
#[inline]
pub fn parse(expr: &str) -> Result<UnitExpr, UcumError> {
    crate::parser::parse(expr)
}

/// Validate that an expression is well-formed UCUM *and* references only known
/// atoms (case-sensitive). Total.
///
/// ```
/// use ucum::UcumError;
/// assert!(ucum::validate("mg/dL").is_ok());
/// assert!(ucum::validate("[ft_i]").is_ok());  // bracketed customary unit
/// assert!(ucum::validate("1").is_ok());        // the dimensionless unity
/// // The failure tells you which atom is unknown.
/// assert!(matches!(
///     ucum::validate("flurble"),
///     Err(UcumError::UnknownAtom { code }) if code == "flurble"
/// ));
/// ```
#[inline]
pub fn validate(expr: &str) -> Result<(), UcumError> {
    Ucum::case_sensitive().validate(expr)
}

/// Return `true` if an expression is well-formed UCUM and references only known atoms.
///
/// A convenience wrapper around [`validate`] that returns a boolean.
///
/// ```
/// assert!(ucum::is_valid("mg/dL"));
/// assert!(!ucum::is_valid("flurble"));
/// ```
#[inline]
pub fn is_valid(expr: &str) -> bool {
    validate(expr).is_ok()
}

/// Analyze an expression into its dimension and conversion factor/offset
/// (case-sensitive). Total.
///
/// ```
/// let a = ucum::analyze("m2").unwrap();
/// assert_eq!(a.dimension, ucum::Dimension([2, 0, 0, 0, 0, 0, 0]));
///
/// let unity = ucum::analyze("1").unwrap();
/// assert!(unity.is_dimensionless);
/// ```
#[inline]
pub fn analyze(expr: &str) -> Result<Analysis, UcumError> {
    Ucum::case_sensitive().analyze(expr)
}

/// Extract the dimensional exponent vector directly from an expression (case-sensitive).
///
/// Returns the 7-element dimension vector corresponding to base units
/// (length, mass, time, electric current, thermodynamic temperature, amount of substance, luminous intensity).
///
/// ```
/// let dim = ucum::dimension("m/s").unwrap();
/// ```
#[inline]
pub fn dimension(expr: &str) -> Result<Dimension, UcumError> {
    Ok(analyze(expr)?.dimension)
}

/// Extract the multiplicative conversion factor directly from an expression (case-sensitive).
///
/// ```
/// let factor = ucum::factor("km").unwrap();
/// assert_eq!(factor, 1000.0);
/// ```
#[inline]
pub fn factor(expr: &str) -> Result<f64, UcumError> {
    Ok(analyze(expr)?.factor)
}

/// Extract the affine offset directly from an expression (case-sensitive).
///
/// Useful for units with a non-zero origin, such as temperature scales.
///
/// ```
/// let offset = ucum::offset("Cel").unwrap();
/// ```
#[inline]
pub fn offset(expr: &str) -> Result<f64, UcumError> {
    Ok(analyze(expr)?.offset)
}

/// Return `true` when two expressions share the same dimension and neither is
/// an arbitrary unit (case-sensitive). Total.
///
/// ```
/// assert!(ucum::is_comparable("km", "[ft_i]").unwrap());   // both length
/// assert!(!ucum::is_comparable("kg", "m").unwrap());        // mass vs length
/// ```
#[inline]
pub fn is_comparable(a: &str, b: &str) -> Result<bool, UcumError> {
    Ucum::case_sensitive().is_comparable(a, b)
}

/// Return `true` when two expressions are comparable, returning `false` on error or mismatch.
///
/// A convenience wrapper around [`is_comparable`] that swallows errors.
///
/// ```
/// assert!(ucum::comparable("km", "m"));
/// assert!(!ucum::comparable("kg", "m"));
/// assert!(!ucum::comparable("invalid", "m"));
/// ```
#[inline]
pub fn comparable(a: &str, b: &str) -> bool {
    is_comparable(a, b).unwrap_or(false)
}

/// Convert a magnitude between two commensurable units (case-sensitive),
/// handling affine offsets (temperature) and logarithmic units. Total.
///
/// Returns [`UcumError::NotComparable`] if the units have different dimensions,
/// or [`UcumError::UnsupportedSpecial`] if either side is an arbitrary unit or a
/// special unit used inside a compound term.
///
/// ```
/// use ucum::{convert, UcumError};
/// assert!((convert(1.0, "[ft_i]", "m").unwrap() - 0.3048).abs() < 1e-12);
/// assert!((convert(1.0, "bar", "Pa").unwrap() - 1e5).abs() < 1e-3);
/// assert!((convert(0.0, "Cel", "K").unwrap() - 273.15).abs() < 1e-9);
/// assert!((convert(2.0, "B", "1").unwrap() - 100.0).abs() < 1e-9);  // bels: log
///
/// // Units of different dimensions cannot be converted.
/// assert!(matches!(
///     convert(1.0, "kg", "m"),
///     Err(UcumError::NotComparable { .. })
/// ));
/// ```
#[inline]
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, UcumError> {
    Ucum::case_sensitive().convert(value, from, to)
}

/// Convert a magnitude between two commensurable units, returning `None` if conversion fails.
///
/// A convenience wrapper around [`convert`] that maps `Result` to an `Option`.
///
/// ```
/// assert_eq!(ucum::try_convert(1.0, "km", "m"), Some(1000.0));
/// assert_eq!(ucum::try_convert(1.0, "kg", "m"), None);
/// ```
#[inline]
pub fn try_convert(value: f64, from: &str, to: &str) -> Option<f64> {
    convert(value, from, to).ok()
}

/// Return a normalized UCUM string for display (case-sensitive). Total.
///
/// The result is the input re-serialized from its parse tree: redundant
/// parentheses are removed (`((m))` → `m`), while parentheses required to
/// preserve UCUM's left-associative grouping are kept (`kg/(m.s)`). Exponent
/// and number formatting are normalized.
///
/// This is a *syntactic* normalization, **not** a full algebraic canonical
/// form: it does not reorder commutative factors (`m.s` and `s.m` stay
/// distinct) nor reduce units to base dimensions. It also does **not** check
/// that atoms are known, only that the expression parses, so a well-formed
/// but unknown unit still round-trips. Use [`validate`] or [`analyze`] for
/// semantic checks, and [`analyze`] when you need dimensional equivalence.
///
/// ```
/// assert_eq!(ucum::canonical("kg.m/s2").unwrap(), "kg.m/s2");
/// assert_eq!(ucum::canonical("((m))").unwrap(), "m");      // redundant parens dropped
/// assert_eq!(ucum::canonical("kg/(m.s)").unwrap(), "kg/(m.s)"); // necessary parens kept
/// assert_eq!(ucum::canonical("flurble").unwrap(), "flurble"); // not validated!
/// ```
#[inline]
#[doc(alias = "normalize")]
pub fn canonical(expr: &str) -> Result<String, UcumError> {
    Ucum::case_sensitive().canonical(expr)
}

/// Generate a human-readable display name (case-sensitive). Total.
///
/// ```
/// assert_eq!(ucum::display_name("mm").unwrap(), "(millimeter)");
/// assert_eq!(ucum::display_name("rad2").unwrap(), "(radian ^ 2)");
/// ```
#[inline]
#[doc(alias = "pretty")]
#[doc(alias = "pretty_print")]
pub fn display_name(expr: &str) -> Result<String, UcumError> {
    Ucum::case_sensitive().display_name(expr)
}

/// Return a case-insensitive UCUM facade instance.
///
/// Allows executing validations and lookups matching atoms case-insensitively.
///
/// ```
/// let ci = ucum::ci();
/// assert!(ci.validate("MOL").is_ok());
/// ```
#[inline]
pub fn ci() -> Ucum {
    Ucum::case_insensitive()
}
