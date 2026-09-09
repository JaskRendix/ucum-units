use roxmltree::Document;
use std::fmt::Write as _;

pub fn generate_tables(xml: &str) -> Result<(String, String), String> {
    let doc = Document::parse(xml).map_err(|e| e.to_string())?;

    let mut prefixes = String::new();
    let mut atoms = String::new();

    for node in doc.descendants() {
        match node.tag_name().name() {
            "prefix" => {
                let code = attr(&node, "Code")?;
                let ci_code = attr(&node, "CODE")?;
                let name = child_text(&node, "name").unwrap_or_default();
                let value = node
                    .children()
                    .find(|c| c.tag_name().name() == "value")
                    .and_then(|v| v.attribute("value"))
                    .ok_or("prefix value missing")?;
                let factor: f64 = value.parse().map_err(|_| "prefix value f64 parse error")?;

                writeln!(
                    prefixes,
                    "    PrefixDef {{ code: {code:?}, ci_code: {ci_code:?}, name: {name:?}, factor: {factor:?} }},"
                )
                .expect("write to String");
            }

            "base-unit" => {
                let code = attr(&node, "Code")?;
                let ci_code = attr(&node, "CODE")?;
                let name = child_text(&node, "name").unwrap_or_default();
                let dim = attr(&node, "dim")?;
                let idx = dim_index(dim)?;

                writeln!(
                    atoms,
                    "    AtomDef {{ code: {code:?}, ci_code: {ci_code:?}, name: {name:?}, \
                     is_metric: true, is_arbitrary: false, kind: AtomKind::Base({idx}) }},"
                )
                .expect("write to String");
            }

            "unit" => {
                let code = attr(&node, "Code")?;
                let ci_code = attr(&node, "CODE")?;
                let name = child_text(&node, "name").unwrap_or_default();
                let is_metric = node.attribute("isMetric") == Some("yes");
                let is_special = node.attribute("isSpecial") == Some("yes");
                let is_arbitrary = node.attribute("isArbitrary") == Some("yes");

                let value = node
                    .children()
                    .find(|c| c.tag_name().name() == "value")
                    .ok_or("unit value element missing")?;

                let kind = if is_special {
                    let func = value
                        .children()
                        .find(|c| c.tag_name().name() == "function")
                        .ok_or("special unit function missing")?;

                    let fname = attr(&func, "name")?;
                    let fval: f64 = attr(&func, "value")?
                        .parse()
                        .map_err(|_| "function value parse error")?;
                    let funit = attr(&func, "Unit")?;

                    format!(
                        "AtomKind::Special {{ func: {fname:?}, value: {fval:?}, unit: {funit:?} }}"
                    )
                } else {
                    let unit = attr(&value, "Unit")?;
                    let v: f64 = attr(&value, "value")?
                        .parse()
                        .map_err(|_| "value f64 parse error")?;

                    format!("AtomKind::Derived {{ value: {v:?}, unit: {unit:?} }}")
                };

                writeln!(
                    atoms,
                    "    AtomDef {{ code: {code:?}, ci_code: {ci_code:?}, name: {name:?}, \
                     is_metric: {is_metric}, is_arbitrary: {is_arbitrary}, kind: {kind} }},"
                )
                .expect("write to String");
            }

            _ => {}
        }
    }

    Ok((prefixes, atoms))
}

fn attr<'i, 'd>(node: &roxmltree::Node<'i, 'd>, name: &str) -> Result<&'i str, String> {
    node.attribute(name)
        .ok_or_else(|| format!("missing attribute {name}"))
}

pub fn child_text<'i, 'd>(node: &roxmltree::Node<'i, 'd>, tag: &str) -> Option<String> {
    node.children()
        .find(|c| c.tag_name().name() == tag)
        .and_then(|c| c.text())
        .map(|s| s.trim().to_string())
}

pub fn dim_index(dim: &str) -> Result<usize, String> {
    match dim {
        "L" => Ok(0),
        "T" => Ok(1),
        "M" => Ok(2),
        "A" => Ok(3),
        "C" => Ok(4),
        "Q" => Ok(5),
        "F" => Ok(6),
        other => Err(format!("unknown base dimension letter: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_successful_parsing_combinations() {
        let xml = r#"
            <root>
                <prefix Code="k" CODE="K">
                    <name>kilo</name>
                    <value value="1000"/>
                </prefix>
                <prefix Code="M" CODE="MEGA">
                    <value value="1000000"/>
                </prefix>
                <base-unit Code="m" CODE="M" dim="L">
                    <name>meter</name>
                </base-unit>
                <unit Code="m2" CODE="M2" isMetric="yes">
                    <name>square meter</name>
                    <value Unit="m" value="2"/>
                </unit>
                <unit Code="Cel" CODE="CEL" isSpecial="yes">
                    <name>Celsius</name>
                    <value>
                        <function name="add" value="273.15" Unit="K"/>
                    </value>
                </unit>
                <unit Code="arb" CODE="ARB" isArbitrary="yes">
                    <name>arbitrary unit</name>
                    <value Unit="1" value="1"/>
                </unit>
            </root>
        "#;

        let (prefixes, atoms) = generate_tables(xml).unwrap();

        assert!(prefixes.contains("code: \"k\""));
        assert!(prefixes.contains("factor: 1000.0"));
        assert!(prefixes.contains("name: \"kilo\""));
        assert!(prefixes.contains("code: \"M\""));
        assert!(prefixes.contains("name: \"\"")); // Missing name defaults safely

        assert!(atoms.contains("code: \"m\""));
        assert!(atoms.contains("AtomKind::Base(0)"));
        assert!(atoms.contains("AtomKind::Derived"));
        assert!(atoms.contains("value: 2.0"));
        assert!(atoms.contains("unit: \"m\""));
        assert!(atoms.contains("AtomKind::Special"));
        assert!(atoms.contains("func: \"add\""));
        assert!(atoms.contains("value: 273.15"));
        assert!(atoms.contains("unit: \"K\""));
        assert!(atoms.contains("is_arbitrary: true"));
    }

    #[test]
    fn test_missing_required_attribute() {
        let xml = r#"
            <root>
                <prefix CODE="K">
                    <name>kilo</name>
                    <value value="1000"/>
                </prefix>
            </root>
        "#;
        let err = generate_tables(xml).unwrap_err();
        assert!(err.contains("missing attribute Code"));
    }

    #[test]
    fn test_invalid_numeric_value() {
        let xml = r#"
            <root>
                <prefix Code="k" CODE="K">
                    <name>kilo</name>
                    <value value="not-a-number"/>
                </prefix>
            </root>
        "#;
        let err = generate_tables(xml).unwrap_err();
        assert!(err.contains("prefix value f64 parse error"));
    }

    #[test]
    fn test_missing_value_element_in_unit() {
        let xml = r#"
            <root>
                <unit Code="g" CODE="G" isMetric="yes">
                    <name>gram</name>
                </unit>
            </root>
        "#;
        let err = generate_tables(xml).unwrap_err();
        assert!(err.contains("unit value element missing"));
    }

    #[test]
    fn test_special_unit_missing_function() {
        let xml = r#"
            <root>
                <unit Code="Cel" CODE="CEL" isSpecial="yes">
                    <name>Celsius</name>
                    <value Unit="K" value="1"/>
                </unit>
            </root>
        "#;
        let err = generate_tables(xml).unwrap_err();
        assert!(err.contains("special unit function missing"));
    }

    #[test]
    fn test_unknown_dimension_letter() {
        let xml = r#"
            <root>
                <base-unit Code="x" CODE="X" dim="Z">
                    <name>weird</name>
                </base-unit>
            </root>
        "#;
        let err = generate_tables(xml).unwrap_err();
        assert!(err.contains("unknown base dimension letter"));
    }
}
