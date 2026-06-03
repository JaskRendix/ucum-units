//! The dimensional exponent vector.

use core::fmt;

/// Number of UCUM base quantities.
pub(crate) const NDIM: usize = 7;

/// Canonical UCUM base units, in vector order, used by [`Dimension::Display`].
pub(crate) const BASE_SYMBOLS: [&str; NDIM] = ["m", "s", "g", "rad", "K", "C", "cd"];

/// A dimensional exponent vector over the seven UCUM base quantities.
///
/// The exponent order is fixed and documented:
///
/// | index | quantity            | base unit |
/// |-------|---------------------|-----------|
/// | 0     | length              | `m`       |
/// | 1     | time                | `s`       |
/// | 2     | mass                | `g`       |
/// | 3     | plane angle         | `rad`     |
/// | 4     | temperature         | `K`       |
/// | 5     | electric charge     | `C`       |
/// | 6     | luminous intensity  | `cd`      |
///
/// All arithmetic saturates at the `i8` bounds rather than overflowing, so the
/// type can never panic, even on absurd inputs such as `m120.m120`.
///
/// ```
/// use ucum::Dimension;
/// let area = Dimension::DIMENSIONLESS.mul(Dimension([2, 0, 0, 0, 0, 0, 0]));
/// assert_eq!(area, Dimension([2, 0, 0, 0, 0, 0, 0]));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Dimension(pub [i8; NDIM]);

impl Dimension {
    /// The dimensionless dimension (all exponents zero).
    pub const DIMENSIONLESS: Dimension = Dimension([0; NDIM]);

    /// Returns `true` when every exponent is zero.
    #[must_use]
    pub fn is_dimensionless(&self) -> bool {
        self.0.iter().all(|&e| e == 0)
    }

    /// Multiplies two dimensions by adding their exponents (saturating).
    ///
    /// Named `mul` per the public UCUM API contract; it intentionally does not
    /// implement [`std::ops::Mul`], to keep the type's surface explicit.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Dimension) -> Dimension {
        let mut out = self.0;
        for (o, &b) in out.iter_mut().zip(other.0.iter()) {
            *o = o.saturating_add(b);
        }
        Dimension(out)
    }

    /// Inverts a dimension by negating its exponents (saturating).
    #[must_use]
    pub fn inv(self) -> Dimension {
        let mut out = self.0;
        for e in &mut out {
            *e = e.saturating_neg();
        }
        Dimension(out)
    }

    /// Raises a dimension to an integer power (saturating).
    #[must_use]
    pub fn powi(self, n: i8) -> Dimension {
        let mut out = self.0;
        for e in &mut out {
            *e = e.saturating_mul(n);
        }
        Dimension(out)
    }
}

impl fmt::Display for Dimension {
    /// Renders a dimension in UCUM base-unit syntax, e.g. `m3.s-1`.
    ///
    /// The dimensionless dimension renders as `1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return f.write_str("1");
        }
        let mut first = true;
        for (i, &e) in self.0.iter().enumerate() {
            if e == 0 {
                continue;
            }
            if !first {
                f.write_str(".")?;
            }
            first = false;
            if e == 1 {
                write!(f, "{}", BASE_SYMBOLS[i])?;
            } else {
                write!(f, "{}{}", BASE_SYMBOLS[i], e)?;
            }
        }
        Ok(())
    }
}
