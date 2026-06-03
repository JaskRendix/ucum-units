//! Total, bounded recursive-descent parser for the UCUM `c/s` grammar.
//!
//! The parser consumes its input strictly monotonically (every step either
//! advances the cursor or unwinds the recursion), so it terminates on every
//! input. A redundant step counter is wired in as a hard backstop: even if a
//! future edit introduced a non-advancing loop, the parser would return a
//! [`UcumError::Parse`] rather than hang. This totality guarantee is the single
//! most important property of the crate.

use crate::error::UcumError;
use core::fmt;

/// A node in the parsed unit expression tree.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    /// A bare integer factor (e.g. `1`, `100`). Dimensionless.
    Factor(f64),
    /// A simple unit: a token (`m`, `kg`, `[ft_i]`, `10*`) with an exponent.
    /// Prefix detection is deferred to analysis; `sym` is the raw token.
    Symbol { sym: String, exp: i32 },
    /// Multiplication: `a.b`.
    Mul(Box<Node>, Box<Node>),
    /// Division: `a/b`.
    Div(Box<Node>, Box<Node>),
    /// Leading-slash reciprocal: `/term`.
    Recip(Box<Node>),
    /// A parenthesized sub-term, preserved for faithful re-display.
    Group(Box<Node>),
}

/// A parsed UCUM unit expression (abstract syntax tree).
///
/// Produced by [`crate::parse`]. Exposed for advanced users who want to inspect
/// or re-display an expression; most callers will use the higher-level
/// functions ([`crate::analyze`], [`crate::convert`], …) directly.
///
/// The [`fmt::Display`] implementation re-serializes the expression in
/// normalized UCUM syntax, dropping redundant parentheses while preserving
/// those required by UCUM's left-associative grouping.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitExpr {
    pub(crate) root: Node,
}

impl UnitExpr {
    /// Borrow the root node (crate-internal).
    pub(crate) fn root_ref(&self) -> &Node {
        &self.root
    }
}

impl fmt::Display for UnitExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_node(&self.root, f, false)
    }
}

/// Strip redundant grouping: a `Group` only ever wraps a sub-term, and the tree
/// structure already encodes the grouping, so the explicit node carries no
/// information for display.
fn peel(mut node: &Node) -> &Node {
    while let Node::Group(inner) = node {
        node = inner;
    }
    node
}

/// Re-serialize a node in normalized UCUM syntax.
///
/// `paren_if_compound` requests parentheses when the node is a compound term
/// (`.`, `/`, or leading-slash). Because UCUM operators are equal-precedence
/// and left-associative, parentheses are only required around the *right*
/// operand of a `.`/`/` (e.g. `kg/(m.s)` ≠ `kg/m.s`); left operands and atoms
/// never need them. This drops redundant parentheses (`((m))` → `m`) while
/// preserving every semantically necessary one.
fn write_node(node: &Node, f: &mut fmt::Formatter<'_>, paren_if_compound: bool) -> fmt::Result {
    match peel(node) {
        Node::Factor(v) => {
            if v.fract() == 0.0 && v.abs() < 1e15 {
                write!(f, "{}", *v as i64)
            } else {
                write!(f, "{v}")
            }
        }
        Node::Symbol { sym, exp } => {
            if *exp == 1 {
                f.write_str(sym)
            } else {
                write!(f, "{sym}{exp}")
            }
        }
        Node::Mul(a, b) => write_compound(f, paren_if_compound, |f| {
            write_node(a, f, false)?;
            f.write_str(".")?;
            write_node(b, f, true)
        }),
        Node::Div(a, b) => write_compound(f, paren_if_compound, |f| {
            write_node(a, f, false)?;
            f.write_str("/")?;
            write_node(b, f, true)
        }),
        Node::Recip(t) => write_compound(f, paren_if_compound, |f| {
            f.write_str("/")?;
            write_node(t, f, false)
        }),
        // Unreachable: `peel` removed all `Group` wrappers.
        Node::Group(_) => Ok(()),
    }
}

fn write_compound<F>(f: &mut fmt::Formatter<'_>, parenthesize: bool, body: F) -> fmt::Result
where
    F: FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result,
{
    if parenthesize {
        f.write_str("(")?;
        body(f)?;
        f.write_str(")")
    } else {
        body(f)
    }
}

/// Parse a UCUM expression into an AST. Total: never panics, never hangs.
pub(crate) fn parse(input: &str) -> Result<UnitExpr, UcumError> {
    let mut p = Parser::new(input);
    let root = p.parse_main_term()?;
    p.skip_done()?;
    Ok(UnitExpr { root })
}

/// Maximum parenthesis nesting depth. Bounds parser (and downstream evaluator)
/// recursion so that pathological input such as `((((…))))` returns an error
/// instead of overflowing the stack.
const MAX_DEPTH: u32 = 256;

struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    steps: u64,
    max_steps: u64,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        let b = src.as_bytes();
        // Generous linear backstop: the parser is already O(n); this only fires
        // on a hypothetical non-advancing bug, never on legitimate input.
        let max_steps = (b.len() as u64).saturating_mul(16).saturating_add(1024);
        Parser {
            src,
            b,
            pos: 0,
            steps: 0,
            max_steps,
            depth: 0,
        }
    }

    #[inline]
    fn step(&mut self) -> Result<(), UcumError> {
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(self.err("expression too complex (step limit exceeded)"));
        }
        Ok(())
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    #[inline]
    fn err(&self, msg: &str) -> UcumError {
        UcumError::Parse {
            pos: self.pos,
            msg: msg.to_string(),
        }
    }

    fn err_at(&self, pos: usize, msg: &str) -> UcumError {
        UcumError::Parse {
            pos,
            msg: msg.to_string(),
        }
    }

    /// `main-term := "/" term | term`
    fn parse_main_term(&mut self) -> Result<Node, UcumError> {
        self.step()?;
        if self.peek() == Some(b'/') {
            self.pos += 1;
            let t = self.parse_term()?;
            Ok(Node::Recip(Box::new(t)))
        } else {
            self.parse_term()
        }
    }

    /// `term := component (("." | "/") component)*`  (left-associative)
    fn parse_term(&mut self) -> Result<Node, UcumError> {
        let mut left = self.parse_component()?;
        loop {
            self.step()?;
            match self.peek() {
                Some(b'.') => {
                    self.pos += 1;
                    let right = self.parse_component()?;
                    left = Node::Mul(Box::new(left), Box::new(right));
                }
                Some(b'/') => {
                    self.pos += 1;
                    let right = self.parse_component()?;
                    left = Node::Div(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// `component := "(" main-term ")" | annotation | annotatable annotation?`
    fn parse_component(&mut self) -> Result<Node, UcumError> {
        self.step()?;
        match self.peek() {
            None => Err(self.err("unexpected end of input, expected a unit")),
            Some(b'(') => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(self.err("expression nested too deeply"));
                }
                self.pos += 1;
                let inner = self.parse_main_term()?;
                if self.peek() != Some(b')') {
                    return Err(self.err("expected ')'"));
                }
                self.pos += 1;
                self.depth -= 1;
                // A '(' group may carry a trailing annotation but never an exponent.
                self.skip_annotation()?;
                Ok(Node::Group(Box::new(inner)))
            }
            Some(b')') => Err(self.err("unexpected ')'")),
            Some(b'{') => {
                // Standalone annotation is dimensionless unity.
                self.skip_annotation()?;
                Ok(Node::Factor(1.0))
            }
            Some(_) => {
                let start = self.pos;
                let chunk = self.read_chunk()?;
                let node = component_from_chunk(self.src, chunk, start)?;
                self.skip_annotation()?;
                Ok(node)
            }
        }
    }

    /// Read a simple-unit token: everything up to a bracket-depth-0 operator,
    /// grouping paren, annotation, or end of input. The contents of `[...]`
    /// (which may contain `.`, `/`, `(`, `)`, etc.) are consumed verbatim.
    /// Returns the byte range of the chunk.
    fn read_chunk(&mut self) -> Result<(usize, usize), UcumError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            self.step()?;
            match c {
                b'[' => {
                    // Consume up to and including the matching ']'.
                    let bracket_start = self.pos;
                    self.pos += 1;
                    loop {
                        match self.peek() {
                            Some(b']') => {
                                self.pos += 1;
                                break;
                            }
                            Some(_) => self.pos += 1,
                            None => {
                                return Err(self.err_at(bracket_start, "unterminated '[' in unit"));
                            }
                        }
                    }
                }
                b'.' | b'/' | b'(' | b')' | b'{' => break,
                _ => self.pos += 1,
            }
        }
        if self.pos == start {
            return Err(self.err("expected a unit"));
        }
        Ok((start, self.pos))
    }

    /// Skip an optional `{annotation}`; annotations are dimensionless comments.
    fn skip_annotation(&mut self) -> Result<(), UcumError> {
        if self.peek() != Some(b'{') {
            return Ok(());
        }
        let open = self.pos;
        self.pos += 1;
        loop {
            self.step()?;
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(());
                }
                Some(b'{') => return Err(self.err("nested '{' in annotation")),
                // UCUM annotations may only contain printable ASCII (33..=126),
                // i.e. graphic characters other than the curly braces.
                Some(c) if (33..=126).contains(&c) => self.pos += 1,
                Some(_) => {
                    return Err(self.err("annotation contains a non-ASCII or control character"));
                }
                None => return Err(self.err_at(open, "unterminated annotation '{'")),
            }
        }
    }

    /// Ensure all input was consumed.
    fn skip_done(&mut self) -> Result<(), UcumError> {
        match self.peek() {
            None => Ok(()),
            Some(_) => Err(self.err("unexpected trailing input")),
        }
    }
}

/// Split a chunk into a unit token + exponent (bracket-aware) and build a node.
fn component_from_chunk(src: &str, range: (usize, usize), start: usize) -> Result<Node, UcumError> {
    let chunk = &src[range.0..range.1];
    let cb = chunk.as_bytes();

    // Exponent digits/sign may only appear after the last ']' (if any).
    let scan_from = chunk.rfind(']').map(|i| i + 1).unwrap_or(0);

    // Consume a trailing run of digits.
    let mut i = cb.len();
    while i > scan_from && cb[i - 1].is_ascii_digit() {
        i -= 1;
    }
    let had_digits = i < cb.len();
    // Include an optional sign immediately before the digit run.
    let exp_start = if had_digits && i > scan_from && (cb[i - 1] == b'+' || cb[i - 1] == b'-') {
        i - 1
    } else if had_digits {
        i
    } else {
        cb.len()
    };

    let unit_part = &chunk[..exp_start];
    let exp_str = &chunk[exp_start..];

    if unit_part.is_empty() {
        // Pure number: a dimensionless factor. A leading sign is not allowed.
        if exp_str.starts_with('+') || exp_str.starts_with('-') {
            return Err(UcumError::Parse {
                pos: start,
                msg: "a numeric factor may not be signed".to_string(),
            });
        }
        let v: f64 = exp_str.parse().map_err(|_| UcumError::Parse {
            pos: start,
            msg: format!("invalid numeric factor '{exp_str}'"),
        })?;
        return Ok(Node::Factor(v));
    }

    let exp: i32 = if exp_str.is_empty() {
        1
    } else {
        exp_str.parse().map_err(|_| UcumError::Parse {
            pos: start + exp_start,
            msg: format!("invalid exponent '{exp_str}'"),
        })?
    };

    Ok(Node::Symbol {
        sym: unit_part.to_string(),
        exp,
    })
}
