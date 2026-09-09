//! Prefix and atom tables, generated from the vendored `ucum-essence.xml` by
//! `build.rs`. The generated file (`$OUT_DIR/ucum_tables.rs`) is `include!`d at
//! the bottom of this module and references the types defined here.

// The generated table embeds faithful UCUM constants verbatim (e.g. the value
// of `[pi]`); these are data, not approximations to be replaced with std consts.
#![allow(clippy::approx_constant)]

/// How a unit atom's canonical value and dimension are obtained.
#[derive(Debug)]
pub(crate) enum AtomKind {
    /// A base unit: the value at the given index of the dimension vector is 1,
    /// the conversion factor is 1.
    Base(usize),
    /// A derived unit: `value` × (the canonical value of `unit`).
    Derived {
        /// Numeric multiplier.
        value: f64,
        /// UCUM reference-unit expression.
        unit: &'static str,
    },
    /// A special (non-multiplicative) unit defined by a named function applied
    /// to the proper unit `value` × `unit`.
    Special {
        /// UCUM special-function name (`Cel`, `degF`, `lg`, `ln`, `pH`, …).
        func: &'static str,
        /// Numeric multiplier of the proper unit.
        value: f64,
        /// UCUM proper-unit expression.
        unit: &'static str,
    },
}

/// A single unit-atom definition.
#[derive(Debug)]
pub(crate) struct AtomDef {
    /// The case-sensitive (`c/s`) UCUM code.
    pub code: &'static str,
    /// The case-insensitive (`c/i`) UCUM code (the upper-case `CODE` column).
    pub ci_code: &'static str,
    /// The printable English name (e.g. `meter`, `pascal`).
    pub name: &'static str,
    /// Whether a metric prefix may be attached.
    pub is_metric: bool,
    /// Whether this is an arbitrary unit (not commensurable with anything).
    pub is_arbitrary: bool,
    /// How to resolve the atom to a canonical value and dimension.
    pub kind: AtomKind,
}

/// A single prefix definition.
#[derive(Debug)]
pub(crate) struct PrefixDef {
    /// The case-sensitive (`c/s`) prefix code (e.g. `k`, `M`, `da`).
    pub code: &'static str,
    /// The case-insensitive (`c/i`) prefix code (the upper-case `CODE` column).
    pub ci_code: &'static str,
    /// The printable English name (e.g. `kilo`, `milli`).
    pub name: &'static str,
    /// The multiplicative factor (e.g. `1e3` for kilo).
    pub factor: f64,
}

include!(concat!(env!("OUT_DIR"), "/ucum_tables.rs"));

/// Compute the Levenshtein distance between two strings for typo suggestions.
pub(crate) fn levenshtein_distance(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();

    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let old = row[j + 1];
            let cost = if ca == cb { 0 } else { 1 };
            row[j + 1] = std::cmp::min(std::cmp::min(row[j + 1] + 1, row[j] + 1), prev + cost);
            prev = old;
        }
    }
    row[b.len()]
}

/// Find the closest matching known atom code if available.
pub(crate) fn suggest_atom(unknown: &str) -> Option<String> {
    let threshold = 3;
    let mut best_match: Option<(&'static str, usize)> = None;

    for atom in ATOMS {
        let dist = levenshtein_distance(unknown, atom.code);
        if dist < threshold {
            if let Some((_, best_dist)) = best_match {
                if dist < best_dist {
                    best_match = Some((atom.code, dist));
                }
            } else {
                best_match = Some((atom.code, dist));
            }
        }
    }

    best_match.map(|(code, _)| code.to_string())
}
