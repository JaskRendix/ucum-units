#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod analysis;
mod dimension;
mod display;
mod error;
mod parser;
mod quantity;
mod tables;

pub use dimension::Dimension;
pub use error::UcumError;
pub use parser::UnitExpr;
pub use quantity::{FhirQuantity, Quantity};

use analysis::{Resolved, Special};

/// Case-sensitivity mode for parsing and lookup.
///
/// UCUM defines a case-sensitive (`c/s`) and a case-insensitive (`c/i`) form.
/// `c/s` is the default for data interchange and the one the free functions
/// ([`parse`], [`analyze`], …) use. Use [`Ucum::case_insensitive`] to opt into
/// `c/i`, where codes are matched against the upper-case `CODE` column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Case {
    /// The `c/s` form: `m` (meter) ≠ `M` (mega). The default.
    #[default]
    Sensitive,
    /// The `c/i` form: case-insensitive matching (`L`, `l`, and `MOL` resolve).
    Insensitive,
}

/// The result of analyzing a unit expression: its dimension and the linear
/// (and affine) relationship of its magnitude to the canonical UCUM base units.
///
/// For an ordinary unit, a magnitude `v` in this unit equals `factor · v` in
/// base units. For an affine unit (e.g. `Cel`), it equals `factor · v + offset`.
/// For a logarithmic or arbitrary special unit, `is_special` is `true` and the
/// linear fields are informational only; see [`Ucum::convert`] for the
/// conversion rules.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Analysis {
    /// The dimensional exponent vector.
    pub dimension: Dimension,
    /// Multiplicative factor to canonical base units.
    pub factor: f64,
    /// Affine offset to canonical base units (`0.0` for multiplicative units).
    pub offset: f64,
    /// Whether the unit is dimensionless.
    pub is_dimensionless: bool,
    /// Whether the unit is a special (non-multiplicative) UCUM unit.
    pub is_special: bool,
}

impl Analysis {
    fn from_resolved(r: Resolved) -> Analysis {
        Analysis {
            dimension: r.dim,
            factor: r.factor,
            offset: r.offset,
            is_dimensionless: r.dim.is_dimensionless(),
            is_special: !matches!(r.special, Special::None),
        }
    }
}

/// A configured UCUM facade carrying a [`Case`] mode.
///
/// The crate-level free functions are shorthands for the case-sensitive
/// instance. Construct a [`Ucum`] when you need case-insensitive (`c/i`)
/// handling:
///
/// ```
/// use ucum::Ucum;
/// let ci = Ucum::case_insensitive();
/// assert!(ci.validate("MOL").is_ok());     // mole, case-insensitively
/// assert!(ci.validate("L").is_ok());
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Ucum {
    case: Case,
}

impl Ucum {
    /// A case-sensitive (`c/s`) facade, the UCUM default.
    #[must_use]
    pub const fn case_sensitive() -> Self {
        Ucum {
            case: Case::Sensitive,
        }
    }

    /// A case-insensitive (`c/i`) facade.
    #[must_use]
    pub const fn case_insensitive() -> Self {
        Ucum {
            case: Case::Insensitive,
        }
    }

    /// The configured case mode.
    #[must_use]
    pub const fn case(&self) -> Case {
        self.case
    }

    /// Parse a UCUM expression into an AST. Total. Parsing is case-independent;
    /// atom identity is resolved later.
    pub fn parse(&self, expr: &str) -> Result<UnitExpr, UcumError> {
        parser::parse(expr)
    }

    /// Validate syntax and that all atoms are known. Total.
    pub fn validate(&self, expr: &str) -> Result<(), UcumError> {
        let ast = parser::parse(expr)?;
        analysis::evaluate(&ast, self.case)?;
        Ok(())
    }

    /// Analyze an expression into its dimension and conversion factor/offset.
    /// Total.
    pub fn analyze(&self, expr: &str) -> Result<Analysis, UcumError> {
        let ast = parser::parse(expr)?;
        Ok(Analysis::from_resolved(analysis::evaluate(
            &ast, self.case,
        )?))
    }

    /// Return `true` when two expressions share the same dimension and neither
    /// is an arbitrary unit (arbitrary units are commensurable with nothing).
    /// Total.
    pub fn is_comparable(&self, a: &str, b: &str) -> Result<bool, UcumError> {
        let ra = analysis::evaluate(&parser::parse(a)?, self.case)?;
        let rb = analysis::evaluate(&parser::parse(b)?, self.case)?;
        if matches!(ra.special, Special::Arbitrary) || matches!(rb.special, Special::Arbitrary) {
            return Ok(false);
        }
        Ok(ra.dim == rb.dim)
    }

    /// Convert a magnitude between two commensurable units, handling affine
    /// offsets (temperature) and logarithmic units (`B`, `dB`, `Np`, `[pH]`, …).
    /// Total.
    ///
    /// Returns [`UcumError::NotComparable`] if the dimensions differ, or
    /// [`UcumError::UnsupportedSpecial`] if either side is an arbitrary unit or
    /// a special unit used inside a compound term (where its meaning is lost).
    pub fn convert(&self, value: f64, from: &str, to: &str) -> Result<f64, UcumError> {
        let a = analysis::evaluate(&parser::parse(from)?, self.case)?;
        let b = analysis::evaluate(&parser::parse(to)?, self.case)?;

        if !a.is_convertible() {
            return Err(UcumError::UnsupportedSpecial {
                unit: from.to_string(),
            });
        }
        if !b.is_convertible() {
            return Err(UcumError::UnsupportedSpecial {
                unit: to.to_string(),
            });
        }
        if a.dim != b.dim {
            return Err(UcumError::NotComparable {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        Ok(b.magnitude_from_base(a.magnitude_to_base(value)))
    }

    /// Return a normalized UCUM string for display. Total. See [`canonical`].
    pub fn canonical(&self, expr: &str) -> Result<String, UcumError> {
        Ok(parser::parse(expr)?.to_string())
    }

    /// Generate a human-readable display name, e.g. `mm` → `(millimeter)`.
    /// Total.
    pub fn display_name(&self, expr: &str) -> Result<String, UcumError> {
        display::display_name(expr, self.case)
    }
}

// --------------------------------------------------------------------------
// Case-sensitive free-function shorthands
// --------------------------------------------------------------------------

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
pub fn parse(expr: &str) -> Result<UnitExpr, UcumError> {
    parser::parse(expr)
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
pub fn validate(expr: &str) -> Result<(), UcumError> {
    Ucum::case_sensitive().validate(expr)
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
pub fn analyze(expr: &str) -> Result<Analysis, UcumError> {
    Ucum::case_sensitive().analyze(expr)
}

/// Return `true` when two expressions share the same dimension and neither is
/// an arbitrary unit (case-sensitive). Total.
///
/// ```
/// assert!(ucum::is_comparable("km", "[ft_i]").unwrap());   // both length
/// assert!(!ucum::is_comparable("kg", "m").unwrap());        // mass vs length
/// ```
pub fn is_comparable(a: &str, b: &str) -> Result<bool, UcumError> {
    Ucum::case_sensitive().is_comparable(a, b)
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
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, UcumError> {
    Ucum::case_sensitive().convert(value, from, to)
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
pub fn canonical(expr: &str) -> Result<String, UcumError> {
    Ucum::case_sensitive().canonical(expr)
}

/// Generate a human-readable display name (case-sensitive). Total.
///
/// ```
/// assert_eq!(ucum::display_name("mm").unwrap(), "(millimeter)");
/// assert_eq!(ucum::display_name("rad2").unwrap(), "(radian ^ 2)");
/// ```
pub fn display_name(expr: &str) -> Result<String, UcumError> {
    Ucum::case_sensitive().display_name(expr)
}
