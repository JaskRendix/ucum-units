//! A typed quantity: a numeric magnitude paired with a UCUM unit expression,
//! with dimensional arithmetic and conversion.

use crate::error::UcumError;
use crate::{Analysis, Dimension};

/// A magnitude expressed in a UCUM unit.
///
/// `Quantity` keeps its unit *symbolic* (as a UCUM string). Arithmetic builds a
/// new compound unit expression rather than eagerly reducing, so no precision is
/// lost and any unit (even one this build doesn't know) round-trips. Errors
/// surface lazily, when you [`analyze`](Quantity::analyze) or
/// [`convert_to`](Quantity::convert_to).
///
/// ```
/// use ucum::Quantity;
///
/// let force = Quantity::new(2.0, "g").mul(&Quantity::new(3.0, "m/s2"));
/// assert_eq!(force.value, 6.0);
/// // The product is commensurable with the newton's base form.
/// assert!(force.is_comparable("N").unwrap());
///
/// // Dividing commensurable quantities yields a dimensionless ratio.
/// let ratio = Quantity::new(1.0, "[lb_av]/h").div(&Quantity::new(1.0, "kg/s"));
/// assert!(ratio.dimension().unwrap().is_dimensionless());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Quantity {
    /// The numeric magnitude.
    pub value: f64,
    /// The UCUM unit expression.
    pub unit: String,
}

impl Quantity {
    /// Create a quantity from a value and a UCUM unit string.
    pub fn new(value: f64, unit: impl Into<String>) -> Self {
        Quantity {
            value,
            unit: unit.into(),
        }
    }

    /// Analyze this quantity's unit. Total.
    pub fn analyze(&self) -> Result<Analysis, UcumError> {
        crate::analyze(&self.unit)
    }

    /// The dimension of this quantity's unit. Total.
    pub fn dimension(&self) -> Result<Dimension, UcumError> {
        Ok(self.analyze()?.dimension)
    }

    /// Whether this quantity's unit is commensurable with `unit`. Total.
    pub fn is_comparable(&self, unit: &str) -> Result<bool, UcumError> {
        crate::is_comparable(&self.unit, unit)
    }

    /// Multiply two quantities: values multiply and units concatenate. The
    /// result unit is `(self.unit).(other.unit)`.
    pub fn mul(&self, other: &Quantity) -> Quantity {
        Quantity {
            value: self.value * other.value,
            unit: format!("{}.{}", group(&self.unit), group(&other.unit)),
        }
    }

    /// Divide two quantities: values divide and the result unit is
    /// `(self.unit)/(other.unit)`.
    pub fn div(&self, other: &Quantity) -> Quantity {
        Quantity {
            value: self.value / other.value,
            unit: format!("{}/{}", group(&self.unit), group(&other.unit)),
        }
    }

    /// Convert this quantity to `unit`, returning a new quantity. Total.
    ///
    /// Errors with [`UcumError::NotComparable`] or
    /// [`UcumError::UnsupportedSpecial`] as for [`crate::convert`].
    pub fn convert_to(&self, unit: &str) -> Result<Quantity, UcumError> {
        Ok(Quantity {
            value: crate::convert(self.value, &self.unit, unit)?,
            unit: unit.to_string(),
        })
    }
}

/// Wrap a unit in parentheses for safe composition; an empty unit is the unity.
fn group(unit: &str) -> String {
    if unit.trim().is_empty() {
        "1".to_string()
    } else {
        format!("({unit})")
    }
}
