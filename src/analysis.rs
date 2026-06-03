//! Dimensional analysis: resolve atoms to their canonical value/dimension and
//! evaluate a parsed expression into a [`Resolved`] result.
//!
//! Atom resolution is performed once, lazily, into an immutable map behind a
//! [`OnceLock`]; the map is `Send + Sync` and shared by all threads. Each atom
//! is resolved by recursively evaluating its UCUM reference expression down to
//! the seven base units, with memoization, a cycle guard, and a depth backstop.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::Case;
use crate::dimension::{Dimension, NDIM};
use crate::error::UcumError;
use crate::parser::{self, Node};
use crate::tables::{ATOMS, AtomDef, AtomKind, PREFIXES, PrefixDef};

/// A UCUM special (non-multiplicative) magnitude function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LogFn {
    /// Common (base-10) logarithm: `lg`, used by the bel `B`.
    Lg,
    /// Natural logarithm: `ln`, used by the neper `Np`.
    Ln,
    /// Binary logarithm: `ld`.
    Ld,
    /// Twice the common logarithm: `lgTimes2`, used by sound-pressure bels.
    Lg2,
    /// Negative common logarithm: `pH`.
    PH,
    /// `100·tan`: prism diopters / percent slope.
    Tan100,
    /// Homeopathic decimal potency (`hpX`, base 10).
    HpX,
    /// Homeopathic centesimal potency (`hpC`, base 100).
    HpC,
    /// Homeopathic millesimal potency (`hpM`, base 1000).
    HpM,
    /// Homeopathic quintamillesimal potency (`hpQ`, base 50000).
    HpQ,
    /// Square root: used by `[m/s2/Hz^(1/2)]`.
    Sqrt,
}

impl LogFn {
    /// Forward function: proper-unit count → special-unit magnitude.
    fn f(self, r: f64) -> f64 {
        match self {
            LogFn::Lg => r.log10(),
            LogFn::Ln => r.ln(),
            LogFn::Ld => r.log2(),
            LogFn::Lg2 => 2.0 * r.log10(),
            LogFn::PH => -r.log10(),
            LogFn::Tan100 => 100.0 * r.tan(),
            LogFn::HpX => -r.log10(),
            LogFn::HpC => -r.ln() / 100f64.ln(),
            LogFn::HpM => -r.ln() / 1000f64.ln(),
            LogFn::HpQ => -r.ln() / 50000f64.ln(),
            LogFn::Sqrt => r.sqrt(),
        }
    }

    /// Inverse function: special-unit magnitude → proper-unit count.
    fn f_inv(self, m: f64) -> f64 {
        match self {
            LogFn::Lg => 10f64.powf(m),
            LogFn::Ln => m.exp(),
            LogFn::Ld => 2f64.powf(m),
            LogFn::Lg2 => 10f64.powf(m / 2.0),
            LogFn::PH => 10f64.powf(-m),
            LogFn::Tan100 => (m / 100.0).atan(),
            LogFn::HpX => 10f64.powf(-m),
            LogFn::HpC => 100f64.powf(-m),
            LogFn::HpM => 1000f64.powf(-m),
            LogFn::HpQ => 50000f64.powf(-m),
            LogFn::Sqrt => m * m,
        }
    }
}

/// Classifies how an evaluated unit relates magnitudes to base units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Special {
    /// Ordinary multiplicative unit: `base = factor · value`.
    None,
    /// Affine unit (temperature): `base = factor · value + offset`.
    Affine,
    /// Logarithmic / trigonometric special unit, with an inner magnitude scale
    /// folded in from any prefix: `base = f⁻¹(value · inner) · factor`.
    Log(LogFn, f64),
    /// Arbitrary unit: not commensurable with anything, including itself.
    Arbitrary,
    /// A special unit degraded by use inside a compound term (e.g. `Cel/s`),
    /// or an unrecognized special function: dimensionally meaningful but not
    /// convertible.
    Opaque,
}

/// The canonical interpretation of a unit (atom or whole expression).
///
/// The relationship to base units is `base = factor · value + offset` for
/// ordinary and affine units, and `f⁻¹(value · inner) · factor` for logarithmic
/// ones. For [`Special::Arbitrary`] and [`Special::Opaque`] the linear fields
/// are informational only (used for dimensional comparison); conversion is
/// refused.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Resolved {
    pub dim: Dimension,
    pub factor: f64,
    pub offset: f64,
    pub special: Special,
}

impl Resolved {
    fn dimensionless(factor: f64) -> Resolved {
        Resolved {
            dim: Dimension::DIMENSIONLESS,
            factor,
            offset: 0.0,
            special: Special::None,
        }
    }

    /// Whether this unit can participate in a numeric conversion.
    pub(crate) fn is_convertible(&self) -> bool {
        !matches!(self.special, Special::Arbitrary | Special::Opaque)
    }

    /// Map a magnitude in this unit to the canonical base-unit magnitude.
    /// Callers must ensure [`Resolved::is_convertible`] first.
    pub(crate) fn magnitude_to_base(&self, v: f64) -> f64 {
        match self.special {
            Special::Log(func, inner) => func.f_inv(v * inner) * self.factor,
            _ => self.factor * v + self.offset,
        }
    }

    /// Map a canonical base-unit magnitude back to a magnitude in this unit.
    /// Callers must ensure [`Resolved::is_convertible`] first.
    pub(crate) fn magnitude_from_base(&self, b: f64) -> f64 {
        match self.special {
            Special::Log(func, inner) => func.f(b / self.factor) / inner,
            _ => (b - self.offset) / self.factor,
        }
    }
}

// --------------------------------------------------------------------------
// Static lookup tables
// --------------------------------------------------------------------------

/// Case-sensitive atom map, keyed by `c/s` code.
fn atom_map_cs() -> &'static HashMap<&'static str, &'static AtomDef> {
    static MAP: OnceLock<HashMap<&'static str, &'static AtomDef>> = OnceLock::new();
    MAP.get_or_init(|| ATOMS.iter().map(|a| (a.code, a)).collect())
}

/// Case-insensitive atom map, keyed by the upper-cased `c/i` code.
fn atom_map_ci() -> &'static HashMap<String, &'static AtomDef> {
    static MAP: OnceLock<HashMap<String, &'static AtomDef>> = OnceLock::new();
    MAP.get_or_init(|| {
        ATOMS
            .iter()
            .map(|a| (a.ci_code.to_uppercase(), a))
            .collect()
    })
}

/// Prefixes keyed by the lookup string for a given case (`c/s` code, or the
/// upper-cased `c/i` code), longest key first so `da`/`Ki` beat `d`/`K`.
fn prefixes(case: Case) -> &'static Vec<(String, &'static PrefixDef)> {
    static CS: OnceLock<Vec<(String, &'static PrefixDef)>> = OnceLock::new();
    static CI: OnceLock<Vec<(String, &'static PrefixDef)>> = OnceLock::new();
    let (lock, key): (_, fn(&'static PrefixDef) -> String) = match case {
        Case::Sensitive => (&CS, |p| p.code.to_string()),
        Case::Insensitive => (&CI, |p| p.ci_code.to_uppercase()),
    };
    lock.get_or_init(|| {
        let mut v: Vec<(String, &'static PrefixDef)> =
            PREFIXES.iter().map(|p| (key(p), p)).collect();
        v.sort_by_key(|(code, _)| std::cmp::Reverse(code.len()));
        v
    })
}

/// Generic "exact atom, else longest prefix + metric atom" search, shared by
/// both case modes. `atom_lookup` resolves a (case-normalized) key to an atom.
fn search(
    key: &str,
    atom_lookup: impl Fn(&str) -> Option<&'static AtomDef>,
    prefixes: &[(String, &'static PrefixDef)],
) -> Option<(Option<&'static PrefixDef>, &'static AtomDef)> {
    // Exact atom match takes precedence (so `cd` is candela, not centi-day).
    if let Some(def) = atom_lookup(key) {
        return Some((None, def));
    }
    for (pcode, p) in prefixes {
        if let Some(rest) = key.strip_prefix(pcode.as_str())
            && !rest.is_empty()
            && let Some(def) = atom_lookup(rest)
            && def.is_metric
        {
            return Some((Some(*p), def));
        }
    }
    None
}

/// Split a simple-unit token into an optional prefix and an atom, honoring the
/// case mode. Shared by analysis and display-name generation.
pub(crate) fn find_atom(
    sym: &str,
    case: Case,
) -> Option<(Option<&'static PrefixDef>, &'static AtomDef)> {
    match case {
        Case::Sensitive => search(sym, |k| atom_map_cs().get(k).copied(), prefixes(case)),
        Case::Insensitive => {
            let upper = sym.to_uppercase();
            search(&upper, |k| atom_map_ci().get(k).copied(), prefixes(case))
        }
    }
}

// --------------------------------------------------------------------------
// Atom resolution (lazy, memoized)
// --------------------------------------------------------------------------

const MAX_RESOLVE_DEPTH: u32 = 64;

/// Build context for the one-time resolution of every atom.
struct BuildCtx {
    memo: RefCell<HashMap<&'static str, Resolved>>,
    visiting: RefCell<HashSet<&'static str>>,
}

impl BuildCtx {
    fn resolve(&self, code: &str, depth: u32) -> Result<Resolved, UcumError> {
        if let Some(r) = self.memo.borrow().get(code).copied() {
            return Ok(r);
        }
        if depth > MAX_RESOLVE_DEPTH {
            return Err(UcumError::Parse {
                pos: 0,
                msg: format!("atom '{code}' definition nested too deeply"),
            });
        }
        let def = *atom_map_cs()
            .get(code)
            .ok_or_else(|| UcumError::UnknownAtom {
                code: code.to_string(),
            })?;

        // Look up the static &'static str key so the memo can be keyed by it.
        let key: &'static str = def.code;
        if !self.visiting.borrow_mut().insert(key) {
            return Err(UcumError::Parse {
                pos: 0,
                msg: format!("cyclic unit definition involving '{code}'"),
            });
        }

        let result = self.resolve_kind(def, depth);

        self.visiting.borrow_mut().remove(key);
        if let Ok(r) = result {
            self.memo.borrow_mut().insert(key, r);
        }
        result
    }

    fn resolve_kind(&self, def: &AtomDef, depth: u32) -> Result<Resolved, UcumError> {
        match &def.kind {
            AtomKind::Base(i) => Ok(Resolved {
                dim: unit_vector(*i),
                factor: 1.0,
                offset: 0.0,
                special: Special::None,
            }),
            AtomKind::Derived { value, unit } => {
                let ast = parser::parse(unit)?;
                // Reference expressions in the essence are always `c/s`.
                let e = eval_with(ast.root_ref(), Case::Sensitive, &mut |c| {
                    self.resolve(c, depth + 1)
                })?;
                // Arbitrary units (e.g. `[iU]`) carry no special *function*
                // (they are ordinary-looking derived units flagged `isArbitrary`)
                // but must remain incommensurable and non-convertible.
                let special = if def.is_arbitrary {
                    Special::Arbitrary
                } else {
                    e.special
                };
                Ok(Resolved {
                    dim: e.dim,
                    factor: e.factor * value,
                    offset: e.offset,
                    special,
                })
            }
            AtomKind::Special { func, value, unit } => {
                let ast = parser::parse(unit)?;
                let pe = eval_with(ast.root_ref(), Case::Sensitive, &mut |c| {
                    self.resolve(c, depth + 1)
                })?;
                Ok(build_special(
                    func,
                    pe.factor * value,
                    pe.dim,
                    def.is_arbitrary,
                ))
            }
        }
    }
}

/// The fully-resolved atom table, built once on first use.
fn resolved() -> &'static HashMap<&'static str, Resolved> {
    static RESOLVED: OnceLock<HashMap<&'static str, Resolved>> = OnceLock::new();
    RESOLVED.get_or_init(|| {
        let ctx = BuildCtx {
            memo: RefCell::new(HashMap::new()),
            visiting: RefCell::new(HashSet::new()),
        };
        for a in ATOMS {
            // Ignore individual failures; an atom that fails to resolve simply
            // stays out of the map and is reported as unknown at lookup time.
            // The `every_atom_resolves` test asserts this never happens.
            let _ = ctx.resolve(a.code, 0);
        }
        ctx.memo.into_inner()
    })
}

fn unit_vector(i: usize) -> Dimension {
    let mut d = [0i8; NDIM];
    d[i] = 1;
    Dimension(d)
}

/// Construct the [`Resolved`] value for a special unit from its function name.
fn build_special(
    func: &str,
    proper_factor: f64,
    proper_dim: Dimension,
    arbitrary: bool,
) -> Resolved {
    if arbitrary {
        return Resolved {
            dim: proper_dim,
            factor: proper_factor,
            offset: 0.0,
            special: Special::Arbitrary,
        };
    }

    // Affine temperature units: base = proper_factor·(value + c), where c is the
    // temperature of 0 K expressed in the proper-unit's count scale.
    //
    // NOTE: the `func` strings below are load-bearing keys into the essence
    // file's <function name="…"> attribute. If a future revision renamed one,
    // the arm would fall through to `Opaque` and the unit would stop converting.
    // `every_atom_resolves` guards existence; the conversion regression tests
    // guard each arm's numerics.
    if let Some(c) = match func {
        "Cel" => Some(273.15),   // 0 K = -273.15 °C
        "degF" => Some(459.67),  // 0 K = -459.67 °F
        "degRe" => Some(218.52), // 0 K = -218.52 °Ré (proper unit = 5/4 K)
        _ => None,
    } {
        return Resolved {
            dim: proper_dim,
            factor: proper_factor,
            offset: proper_factor * c,
            special: Special::Affine,
        };
    }

    let special = match func {
        "lg" => Special::Log(LogFn::Lg, 1.0),
        "ln" => Special::Log(LogFn::Ln, 1.0),
        "ld" => Special::Log(LogFn::Ld, 1.0),
        "lgTimes2" => Special::Log(LogFn::Lg2, 1.0),
        "pH" => Special::Log(LogFn::PH, 1.0),
        "tanTimes100" | "100tan" => Special::Log(LogFn::Tan100, 1.0),
        "hpX" => Special::Log(LogFn::HpX, 1.0),
        "hpC" => Special::Log(LogFn::HpC, 1.0),
        "hpM" => Special::Log(LogFn::HpM, 1.0),
        "hpQ" => Special::Log(LogFn::HpQ, 1.0),
        "sqrt" => Special::Log(LogFn::Sqrt, 1.0),
        _ => Special::Opaque,
    };
    Resolved {
        dim: proper_dim,
        factor: proper_factor,
        offset: 0.0,
        special,
    }
}

// --------------------------------------------------------------------------
// Expression evaluation
// --------------------------------------------------------------------------

/// Evaluate a parsed expression against the runtime resolved-atom table.
pub(crate) fn evaluate(expr: &crate::parser::UnitExpr, case: Case) -> Result<Resolved, UcumError> {
    let table = resolved();
    eval_with(expr.root_ref(), case, &mut |c| {
        table.get(c).copied().ok_or_else(|| UcumError::UnknownAtom {
            code: c.to_string(),
        })
    })
}

/// Evaluate a node, resolving canonical atom codes through `lookup`.
fn eval_with<L>(node: &Node, case: Case, lookup: &mut L) -> Result<Resolved, UcumError>
where
    L: FnMut(&str) -> Result<Resolved, UcumError>,
{
    match node {
        Node::Factor(v) => Ok(Resolved::dimensionless(*v)),
        Node::Symbol { sym, exp } => {
            let base = resolve_symbol(sym, case, lookup)?;
            Ok(apply_exp(base, *exp))
        }
        Node::Mul(a, b) => {
            let x = eval_with(a, case, lookup)?;
            let y = eval_with(b, case, lookup)?;
            Ok(combine(x, y))
        }
        Node::Div(a, b) => {
            let x = eval_with(a, case, lookup)?;
            let y = eval_with(b, case, lookup)?;
            Ok(combine(x, invert(y)))
        }
        Node::Recip(t) => {
            let x = eval_with(t, case, lookup)?;
            Ok(invert(x))
        }
        Node::Group(t) => eval_with(t, case, lookup),
    }
}

/// Resolve a simple-unit token into a [`Resolved`], applying prefix logic.
fn resolve_symbol<L>(sym: &str, case: Case, lookup: &mut L) -> Result<Resolved, UcumError>
where
    L: FnMut(&str) -> Result<Resolved, UcumError>,
{
    match find_atom(sym, case) {
        Some((prefix, def)) => {
            let mut r = lookup(def.code)?;
            if let Some(p) = prefix {
                apply_prefix(&mut r, p.factor);
            }
            Ok(r)
        }
        None => Err(UcumError::UnknownAtom {
            code: sym.to_string(),
        }),
    }
}

/// Apply a metric prefix to a resolved atom. The composition differs by kind:
/// a logarithmic unit folds the prefix into its inner magnitude scale (so
/// `dB` = decibel scales the argument), whereas ordinary and affine units scale
/// the linear factor (so `mCel` keeps its zero-offset).
fn apply_prefix(r: &mut Resolved, p: f64) {
    match r.special {
        Special::Log(func, inner) => r.special = Special::Log(func, inner * p),
        Special::Arbitrary => {} // arbitrary units are never convertible anyway
        _ => r.factor *= p,
    }
}

/// Saturating cast of an `i32` exponent to `i8` for dimension arithmetic.
fn clamp_exp(exp: i32) -> i8 {
    exp.clamp(i8::MIN as i32, i8::MAX as i32) as i8
}

/// The core special-unit rule, in one place: any operator other than the
/// identity (exponent 1) applied to a special unit destroys its special meaning.
/// Arbitrary units stay arbitrary (still incommensurable); affine/log/opaque
/// units become [`Special::Opaque`]; ordinary units stay ordinary.
fn degrade(s: Special) -> Special {
    match s {
        Special::None => Special::None,
        Special::Arbitrary => Special::Arbitrary,
        _ => Special::Opaque,
    }
}

fn apply_exp(r: Resolved, exp: i32) -> Resolved {
    // Exponent 1 is the identity and deliberately preserves special units
    // (e.g. a bare `Cel` stays convertible); every other power degrades.
    if exp == 1 {
        return r;
    }
    Resolved {
        dim: r.dim.powi(clamp_exp(exp)),
        factor: r.factor.powi(exp),
        offset: 0.0,
        special: degrade(r.special),
    }
}

fn invert(r: Resolved) -> Resolved {
    Resolved {
        dim: r.dim.inv(),
        factor: 1.0 / r.factor,
        offset: 0.0,
        special: degrade(r.special),
    }
}

/// Multiply two evaluated sub-expressions. Any combination involving a special
/// unit yields a non-convertible result, since affine/logarithmic units have no
/// meaning inside a compound term; an arbitrary operand contaminates to
/// arbitrary so the result remains incommensurable.
fn combine(a: Resolved, b: Resolved) -> Resolved {
    let special = match (a.special, b.special) {
        (Special::None, Special::None) => Special::None,
        (Special::Arbitrary, _) | (_, Special::Arbitrary) => Special::Arbitrary,
        _ => Special::Opaque,
    };
    Resolved {
        dim: a.dim.mul(b.dim),
        factor: a.factor * b.factor,
        offset: 0.0,
        special,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every atom in the generated table must resolve to a canonical
    /// value/dimension. A failure here means a data regression in
    /// `ucum-essence.xml` (a typo'd reference unit, a new cycle, …) that would
    /// otherwise silently degrade an atom to `UnknownAtom` at runtime.
    #[test]
    fn every_atom_resolves() {
        let table = resolved();
        let missing: Vec<&str> = ATOMS
            .iter()
            .map(|a| a.code)
            .filter(|code| !table.contains_key(code))
            .collect();
        assert!(
            missing.is_empty(),
            "{} of {} atoms failed to resolve: {:?}",
            missing.len(),
            ATOMS.len(),
            missing
        );
    }
}
