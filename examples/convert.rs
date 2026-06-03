//! Parse → analyze → convert, end to end, plus display names, logarithmic
//! units, case-insensitive mode, and quantity arithmetic.
//!
//! Run with: `cargo run --example convert`

use ucum::{Quantity, Ucum, analyze, canonical, convert, display_name, is_comparable};

fn main() {
    // Parse and inspect a compound unit.
    let expr = "kg.m/s2"; // newton
    let a = analyze(expr).unwrap();
    println!("{expr}");
    println!("  canonical:  {}", canonical(expr).unwrap());
    println!("  dimension:  {}", a.dimension);
    println!("  to base:    × {}", a.factor);
    println!();

    // Dimensional comparison.
    println!(
        "km comparable with [ft_i]? {}",
        is_comparable("km", "[ft_i]").unwrap()
    );
    println!(
        "kg comparable with m?      {}",
        is_comparable("kg", "m").unwrap()
    );
    println!();

    // A few conversions, including an affine (temperature) one.
    for (v, from, to) in [
        (1.0, "[ft_i]", "m"),
        (1.0, "bar", "Pa"),
        (1.0, "[gal_us]", "L"),
        (100.0, "Cel", "K"),
        (98.6, "[degF]", "Cel"),
        (20.0, "dB", "1"), // logarithmic: 20 dB = a ratio of 100
    ] {
        match convert(v, from, to) {
            Ok(out) => println!("{v} {from:>8} = {out} {to}"),
            Err(e) => println!("{v} {from:>8} -> {to}: {e}"),
        }
    }
    println!();

    // Display names.
    for u in ["mm", "m3.kg-1.s-2", "Cel"] {
        println!("display_name({u:?}) = {}", display_name(u).unwrap());
    }
    println!();

    // Case-insensitive (c/i) mode.
    let ci = Ucum::case_insensitive();
    println!("c/i: 1 KM = {} M", ci.convert(1.0, "KM", "M").unwrap());
    println!();

    // Quantity arithmetic.
    let speed = Quantity::new(100.0, "km").div(&Quantity::new(2.0, "h"));
    println!(
        "100 km / 2 h = {} {} ({} m/s)",
        speed.value,
        speed.unit,
        speed.convert_to("m/s").unwrap().value
    );
}
