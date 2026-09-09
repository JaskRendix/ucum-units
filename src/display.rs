//! Human-readable display-name generation, matching the UCUM functional-test
//! `displayNameGeneration` format, e.g. `mm` → `(millimeter)` and
//! `m3.kg-1.s-2` → `(meter ^ 3) * (kilogram ^ -1) * (second ^ -2)`.

use crate::Case;
use crate::analysis::find_atom;
use crate::error::UcumError;
use crate::parser::{self, Node};

/// Generate the display name for a unit expression.
pub(crate) fn display_name(expr: &str, case: Case) -> Result<String, UcumError> {
    // The empty term is the unity, per the UCUM display-name convention.
    if expr.is_empty() {
        return Ok("(unity)".to_string());
    }
    let ast = parser::parse(expr)?;
    let mut out = String::new();
    render(ast.root_ref(), case, &mut out)?;
    Ok(out)
}

fn render(node: &Node, case: Case, out: &mut String) -> Result<(), UcumError> {
    match node {
        Node::Factor(v) => {
            out.push_str(&format_number(*v));
            Ok(())
        }
        Node::Symbol { sym, exp } => {
            let (prefix, atom) = find_atom(sym, case).ok_or_else(|| UcumError::UnknownAtom {
                code: sym.to_string(),
                suggestion: crate::tables::suggest_atom(sym),
            })?;
            let name = match prefix {
                Some(p) => format!("{}{}", p.name, atom.name),
                None => atom.name.to_string(),
            };
            if *exp == 1 {
                out.push_str(&format!("({name})"));
            } else {
                out.push_str(&format!("({name} ^ {exp})"));
            }
            Ok(())
        }
        Node::Mul(a, b) => {
            render(a, case, out)?;
            out.push_str(" * ");
            render(b, case, out)
        }
        Node::Div(a, b) => {
            render(a, case, out)?;
            out.push_str(" / ");
            render(b, case, out)
        }
        Node::Recip(t) => {
            out.push_str("1 / ");
            render(t, case, out)
        }
        Node::Group(t) => render(t, case, out),
    }
}

fn format_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
