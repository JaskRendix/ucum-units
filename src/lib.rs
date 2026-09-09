#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod analysis;
mod dimension;
mod display;
mod error;
mod facade;
mod parser;
mod quantity;
mod tables;

pub use dimension::Dimension;
pub use error::UcumError;
pub use facade::*;
pub use parser::UnitExpr;
pub use quantity::Quantity;

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
