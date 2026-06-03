//! Benchmarks for the hot paths: parsing, analysis, validation, and conversion.
//!
//! Run with `cargo bench`. The atom-resolution table is built once (lazily) on
//! first use; we warm it up before measuring so the benchmarks reflect
//! steady-state cost rather than one-off initialization.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use ucum::{analyze, convert, display_name, parse, validate};

/// A spread of representative inputs: a base unit, a derived/compound unit, a
/// common clinical unit, an exponent-suffixed term, and a deliberately gnarly
/// expression mixing factors, the number pi, scientific notation, and division.
const EXPRS: &[&str] = &["m", "kg.m/s2", "mg/dL", "m3/s", "4.[pi].10*-7.N/A2"];

fn warm_up() {
    // Trigger the one-time resolution of the full atom table.
    let _ = analyze("kg.m/s2");
}

fn bench_parse(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse");
    for &expr in EXPRS {
        g.bench_function(expr, |b| b.iter(|| parse(black_box(expr)).unwrap()));
    }
    g.finish();
}

fn bench_validate(c: &mut Criterion) {
    warm_up();
    let mut g = c.benchmark_group("validate");
    for &expr in EXPRS {
        g.bench_function(expr, |b| b.iter(|| validate(black_box(expr)).unwrap()));
    }
    g.finish();
}

fn bench_analyze(c: &mut Criterion) {
    warm_up();
    let mut g = c.benchmark_group("analyze");
    for &expr in EXPRS {
        g.bench_function(expr, |b| b.iter(|| analyze(black_box(expr)).unwrap()));
    }
    g.finish();
}

fn bench_convert(c: &mut Criterion) {
    warm_up();
    let mut g = c.benchmark_group("convert");
    let cases: &[(&str, &str)] = &[
        ("[ft_i]", "m"),   // multiplicative, customary
        ("mg/L", "kg/m3"), // multiplicative, compound
        ("Cel", "K"),      // affine (temperature)
        ("dB", "1"),       // logarithmic
    ];
    for &(from, to) in cases {
        g.bench_function(format!("{from} -> {to}"), |b| {
            b.iter(|| convert(black_box(1.0), black_box(from), black_box(to)).unwrap())
        });
    }
    g.finish();
}

fn bench_display_name(c: &mut Criterion) {
    warm_up();
    c.bench_function("display_name/m3.kg-1.s-2", |b| {
        b.iter(|| display_name(black_box("m3.kg-1.s-2")).unwrap())
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_validate,
    bench_analyze,
    bench_convert,
    bench_display_name
);
criterion_main!(benches);
