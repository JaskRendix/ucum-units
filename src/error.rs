//! The crate error type.

/// Errors produced by parsing, validation, analysis and conversion.
///
/// Every public function in this crate is *total*: it returns one of these
/// errors rather than panicking or looping forever on malformed input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UcumError {
    /// The input is not syntactically a valid UCUM expression.
    ///
    /// `pos` is the byte offset into the input at which the problem was
    /// detected.
    #[error("parse error at byte {pos}: {msg}")]
    Parse {
        /// Byte offset into the original input.
        pos: usize,
        /// Human-readable description of the problem.
        msg: String,
    },

    /// A unit token was syntactically well-formed but is not a known UCUM atom
    /// (optionally with a prefix).
    #[error("unknown unit atom: {code}")]
    UnknownAtom {
        /// The offending token.
        code: String,
    },

    /// Two units were compared or converted but have different dimensions.
    #[error("units not commensurable: {from} vs {to}")]
    NotComparable {
        /// The source unit expression.
        from: String,
        /// The target unit expression.
        to: String,
    },

    /// Conversion was requested for a special (non-multiplicative) unit whose
    /// conversion is not supported, e.g. a logarithmic, arbitrary, or
    /// otherwise non-affine unit, or an affine unit used inside a compound
    /// term.
    #[error("conversion unsupported for special unit: {unit}")]
    UnsupportedSpecial {
        /// The unit expression that could not be converted.
        unit: String,
    },
}
