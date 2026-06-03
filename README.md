# ucum-units

A friendly, complete Rust implementation of [UCUM](https://ucum.org/ucum), the
Unified Code for Units of Measure: the standard used across healthcare, science,
and data interchange to write units precisely and unambiguously.

With `ucum-units` you can parse unit codes, check whether they're valid, work out
their dimensions, ask whether two units are comparable, and convert values
between them, including temperatures and logarithmic units like decibels.

```rust
use ucum::{analyze, convert, is_comparable};

// Convert between units, including temperatures.
let ft = convert(1.0, "[ft_i]", "m").unwrap();
println!("{ft}");                                       // => 0.3048
let body = convert(98.6, "[degF]", "Cel").unwrap();
println!("{body}");                                     // => 37.0 (98.6 °F in Celsius)

// Check commensurability before converting.
println!("{}", is_comparable("kg/m3", "mg/L").unwrap()); // => true  (both densities)
println!("{}", is_comparable("kg", "m").unwrap());       // => false (mass vs length)

// Inspect a unit's scale to the canonical base units.
let km = analyze("km").unwrap();
println!("{}", km.factor);                              // => 1000.0 (1 km = 1000 m)
```

## What is UCUM, and why would I use it?

A unit like “milligrams per deciliter” gets written a dozen different ways in
the wild: `mg/dL`, `mg/dl`, `MG/DL`, `milligram/deciliter`, `mg.dL-1`. That
ambiguity is fine for humans and a disaster for software: two systems exchanging
lab results, sensor readings, or dosages need to agree on exactly what a unit
*is* before they can compare or convert values.

**UCUM** (the Unified Code for Units of Measure) fixes this by giving every unit
a single, unambiguous, machine-parseable code. It is built from a small
grammar (a fixed set of base atoms, metric prefixes, and operators), so any
unit, however exotic, has exactly one canonical spelling. It is the unit
standard used by HL7/FHIR, IEEE 11073, and much of healthcare and laboratory
data interchange.

You do **not** need to memorize UCUM to use this crate. If you already have unit
codes (from a FHIR resource, a device, a spreadsheet), pass them straight in. If
you're writing them yourself, the [five-minute tutorial](#ucum-in-five-minutes)
below is all you need.

## Installation

```sh
cargo add ucum-units
```

The crate is dependency-light (just `thiserror` at runtime) and contains no
`unsafe` code.

## What you can do

| Task | Function |
|------|----------|
| Parse a unit into an AST | `parse(expr)` |
| Check a unit is well-formed and known | `validate(expr)` |
| Get the dimension + conversion factor/offset | `analyze(expr)` |
| Ask whether two units are comparable | `is_comparable(a, b)` |
| Convert a value between units | `convert(value, from, to)` |
| Normalize a unit string for display | `canonical(expr)` |
| Get a readable name (`mm` → `(millimeter)`) | `display_name(expr)` |

Everything is a plain function, deterministic, and thread-safe. The lookup
tables are immutable and shared, so calls are cheap and safe to make from
anywhere.

## UCUM in five minutes

Every UCUM code is built from four ingredients. Once you can spot them, you can
read and write almost any unit.

**1. Atoms** are the base vocabulary, the actual units. `m` (meter), `g`
(gram), `s` (second), `mol` (mole), `K` (kelvin), `L` (liter), `Pa` (pascal),
`min` (minute). Case matters: `m` is the meter.

**2. Prefixes** attach to the front of a metric atom to scale it. `k` = kilo,
`m` = milli, `u` = micro, `n` = nano, `M` = mega, `d` = deci, `c` = centi.

| Code | Reads as |
|------|----------|
| `km` | kilometer |
| `mg` | milligram |
| `uL` | microliter |
| `kPa` | kilopascal |
| `nmol` | nanomole |

> ⚠️ A prefix on its own is not a unit. `M` means the *mega* prefix, not the
> meter; the meter is `m`. This is the single most common newcomer surprise.

**3. Operators** combine atoms into compound units:

- `.` multiplies: `N.m` is a newton-meter.
- `/` divides: `m/s` is meters per second; `mg/dL` is milligrams per deciliter.
- A leading `/` makes a reciprocal: `/min` is “per minute”.
- A trailing **number** is an exponent: `m2` is square meters, `s-1` is per
  second, `m3` is cubic meters. (UCUM writes the exponent as a suffix, not with
  `^`.)

So a force, kg·m/s², is written `kg.m/s2`, and an acceleration is `m/s2`.

**4. Brackets and braces** handle the special cases:

- **`[...]`** wraps customary, named, or “non-metric” units that don't follow
  the prefix rules: `[ft_i]` (international foot), `[gal_us]` (US gallon),
  `[degF]` (degree Fahrenheit), `[in_i]` (inch), `[lb_av]` (pound), `[pH]`.
  When a unit looks like a word or proper name, it usually lives in brackets.
- **`{...}`** is a free-text **annotation**, a human note that carries no
  dimensional meaning. `mg{total}`, `/min{beats}`, and `ng/mL{IgG}` are
  dimensionally identical to `mg`, `/min`, and `ng/mL`; the brace text is along
  for the ride.
- **`(...)`** groups terms, exactly like in arithmetic: `kg/(m.s)` is
  kilograms per (meter·second), which is different from `kg/m.s`.

A couple more building blocks you'll meet:

- **`1`** is the dimensionless unity, a pure ratio. `%` (percent) and `[pH]`
  build on it.
- **`10*6`** is UCUM's scientific notation for powers of ten, common in lab
  counts like `10*6/uL` (millions per microliter).

That's the whole grammar. Putting it together:

| UCUM code | Plain English |
|-----------|---------------|
| `mg/dL` | milligrams per deciliter |
| `mmol/L` | millimoles per liter |
| `km/h` | kilometers per hour |
| `kg.m/s2` | kilogram-meters per second² (a newton) |
| `mm[Hg]` | millimeters of mercury (blood pressure) |
| `10*6/uL` | millions per microliter (a cell count) |
| `ng/mL{IgG}` | nanograms per milliliter, annotated “IgG” |

### Common gotchas

UCUM is precise, which means a few spellings can surprise newcomers:

| You write | It means |
|-----------|----------|
| `m` vs `M` | meter vs the *mega* prefix (`M` on its own isn't a unit) |
| `ft` | femto·tonne (a mass!); the foot is `[ft_i]` |
| `m2`, `s-1` | exponents are suffixes, e.g. `m3/s`, `s-1` |
| `[ft_i]`, `[gal_us]` | customary units live in square brackets |
| `1` | the dimensionless unity |
| `kg{wet}` | `{…}` is a free-text annotation; ignored dimensionally |
| `/s` | a leading slash is a reciprocal, i.e. `s⁻¹` |
| `kPa` | prefixes attach to metric units (`kg`, `kPa`, `mL`) |

## A guided tour (no conversions)

Conversions get [their own section](#working-with-quantities); here's everything
*else* the crate can tell you about a unit code. Suppose someone hands you the
string `mg/dL` and you want to make sense of it.

```rust
use ucum::{validate, analyze, canonical, display_name};

// 1. Is it even a real UCUM unit? validate() checks both the grammar and that
//    every atom is known. It never panics; bad input comes back as an Err.
println!("{}", validate("mg/dL").is_ok());    // => true
println!("{}", validate("flurble").is_ok());  // => false (unknown atom)
println!("{}", validate("mg/").is_ok());       // => false (malformed)

// 2. What does it mean in English? Handy for UIs and logs.
println!("{}", display_name("mg/dL").unwrap()); // => (milligram) / (deciliter)
println!("{}", display_name("mm").unwrap());    // => (millimeter)

// 3. What's its canonical spelling? canonical() re-serializes the parse tree,
//    dropping redundant parentheses but keeping meaningful ones.
println!("{}", canonical("((m))").unwrap());    // => m
println!("{}", canonical("kg/(m.s)").unwrap()); // => kg/(m.s)

// 4. What is it, dimensionally? analyze() gives the dimension vector plus the
//    factor to the canonical base units, without converting any value.
let a = analyze("mg/dL").unwrap();
println!("{}", a.is_dimensionless);             // => false
// mg/dL is a mass concentration: mass / volume = g · m⁻³.
```

`analyze` is the workhorse: it's how you'd group lab results by what they
*measure*, validate that a device is reporting the unit you expect, or render a
unit's meaning, all without performing arithmetic. The
[Dimensions](#dimensions) section below explains the vector it returns.

## Working with quantities

`Quantity` pairs a value with a unit and lets you do dimensional arithmetic:

```rust
use ucum::Quantity;

let speed = Quantity::new(100.0, "km").div(&Quantity::new(2.0, "h"));
println!("{}", speed.is_comparable("m/s").unwrap()); // => true

let in_ms = speed.convert_to("m/s").unwrap();
println!("{}", in_ms.value);                         // => 13.888888888888889
```

## Case-insensitive mode

UCUM has a case-sensitive form (`c/s`, the default for data interchange) and a
case-insensitive form (`c/i`). The free functions use `c/s`; for `c/i`, reach
for the `Ucum` facade:

```rust
use ucum::Ucum;

let ci = Ucum::case_insensitive();
println!("{}", ci.validate("MOL").is_ok());          // => true (mole)
println!("{}", ci.convert(1.0, "M", "CM").unwrap()); // => 100.0
```

## Dimensions

A **dimension** is *what* a unit measures, stripped of the particular unit you
chose. A meter and a foot are different units but the same dimension: length.
Meters-per-second and miles-per-hour are both *length ÷ time*. Capturing this is
what lets the crate tell you that `kg/m3` and `mg/dL` are comparable (both are
mass ÷ volume) while `kg` and `m` are not.

UCUM expresses every dimension as a combination of **seven base quantities**.
`Dimension` records how many powers of each appear, as a fixed exponent vector
`[i8; 7]`:

| index | quantity            | base unit | example unit using it |
|-------|---------------------|-----------|-----------------------|
| 0     | length              | `m`       | `m`, `[ft_i]`, `km`   |
| 1     | time                | `s`       | `s`, `min`, `h`       |
| 2     | mass                | `g`       | `g`, `kg`, `[lb_av]`  |
| 3     | plane angle         | `rad`     | `rad`, `deg`          |
| 4     | temperature         | `K`       | `K`, `Cel`            |
| 5     | electric charge     | `C`       | `C`, `A.s`            |
| 6     | luminous intensity  | `cd`      | `cd`                  |

So the vector reads off directly. A velocity is length¹·time⁻¹, i.e. the
exponent `1` in slot 0 and `-1` in slot 1:

```rust
use ucum::analyze;

// m/s is velocity: length per time.
println!("{:?}", analyze("m/s").unwrap().dimension);
// => Dimension([1, -1, 0, 0, 0, 0, 0])

// m³/s is volume flow rate.
println!("{:?}", analyze("m3/s").unwrap().dimension);
// => Dimension([3, -1, 0, 0, 0, 0, 0])

// A force (newton = kg·m/s²) is mass¹·length¹·time⁻². Both spellings reduce
// to the same vector, which is exactly why they're interchangeable.
println!("{:?}", analyze("kg.m/s2").unwrap().dimension); // => Dimension([1, -2, 1, 0, 0, 0, 0])
println!("{:?}", analyze("N").unwrap().dimension);       // => Dimension([1, -2, 1, 0, 0, 0, 0])
```

That last pair shows the key property: **many units share one dimension.**
`[ft_i]` and `m` both reduce to `[1,0,0,0,0,0,0]`; `kg/m3` and `mg/dL` both
reduce to `[-3,0,1,0,0,0,0]`.

### Printing and comparing

`Dimension` has a `Display` that renders the vector back in UCUM base-unit
syntax, handy for logs and error messages. The dimensionless dimension prints
as `1`:

```rust
use ucum::analyze;

println!("{}", analyze("m/s").unwrap().dimension);     // => m.s-1
println!("{}", analyze("kg.m/s2").unwrap().dimension); // => m.s-2.g
println!("{}", analyze("%").unwrap().dimension);       // => 1
```

[`is_comparable`](#what-you-can-do) is, under the hood, just an equality check
on these vectors (with one extra rule for arbitrary units; see
[Feature support](#feature-support)). Two units are convertible **iff** their
dimension vectors are identical.

### Dimension arithmetic

`Dimension` is a value type you can manipulate directly. The operations mirror
what happens when you multiply or divide units: exponents add, invert, and
scale.

```rust
use ucum::Dimension;

let length = Dimension([1, 0, 0, 0, 0, 0, 0]);
let time   = Dimension([0, 1, 0, 0, 0, 0, 0]);

let velocity = length.mul(time.inv());        // length · time⁻¹
println!("{velocity:?}");                      // => Dimension([1, -1, 0, 0, 0, 0, 0])

let area = length.powi(2);                     // length²
println!("{area:?}");                          // => Dimension([2, 0, 0, 0, 0, 0, 0])

println!("{}", Dimension::DIMENSIONLESS.is_dimensionless()); // => true
```

All three operations **saturate** at the `i8` bounds (±127) instead of
overflowing, so `Dimension` can never panic, even on pathological input like
`m120.m120`, whose length exponent simply caps at `127`.

### A couple of UCUM surprises

Two cases catch people out, because UCUM's notion of “dimension” is narrower
than physics' notion of “base quantity”:

- **The mole is dimensionless.** UCUM treats `mol` as a pure count
  (`[0,0,0,0,0,0,0]`), not a base quantity of its own. So `analyze("mol")` is
  flagged `is_dimensionless`, and a molar concentration `mol/L` has the same
  dimension as a plain inverse volume.
- **Plane angle is *not* dimensionless here.** `rad` occupies its own slot
  (index 3), so `rad` is **not** comparable with the unity `1`, and the
  steradian `sr` analyzes as `rad2`. This makes angles first-class but means
  `is_comparable("rad", "1")` returns `false`.

## Feature support

| Feature | Status |
|---------|--------|
| Case-sensitive (`c/s`) grammar | ✅ |
| Case-insensitive (`c/i`) mode | ✅ |
| Full atom + prefix coverage (essence 2.2) | ✅ |
| Multiplicative conversion | ✅ |
| Affine conversion (`Cel`, `[degF]`, `[degRe]`) | ✅ |
| Logarithmic conversion (`B`, `dB`, `Np`, `[pH]`, …) | ✅ |
| Display-name generation | ✅ |
| Quantity arithmetic | ✅ |
| Arbitrary units (`[iU]`, `[arb'U]`, …) | Parse & analyze; incommensurable by design, so not convertible |
| Special units inside compound terms (`Cel/s`) | Parse & analyze; `convert` reports them as unsupported |

## Reliability

Parsing and analysis are *total*: for any input at all (valid, malformed, or
adversarial) they return a result or a precise error, and never hang, panic, or
overflow the stack. This is enforced by step- and depth-bounded parsing and
checked continuously with property tests (`proptest`) and a `cargo-fuzz` target.

Errors are descriptive and carry a byte offset for parse problems, so you can
point users straight at the issue:

```rust
use ucum::validate;

// Errors are typed; match on the variant, or just print them.
println!("{:?}", validate("m/"));
// => Err(Parse { pos: 2, msg: "unexpected end of input, expected a unit" })
println!("{:?}", validate("flurble"));
// => Err(UnknownAtom { code: "flurble" })
```

## Conformance

`ucum-units` runs against the official UCUM functional test suite and passes all
573 cases: validation, conversion, display-name generation, and quantity
multiplication/division. The test suite is vendored and run as part of `cargo
test`.

## Benchmarks

A [Criterion](https://crates.io/crates/criterion) benchmark suite covers the hot
paths: parsing, validation, analysis, conversion, and display-name generation:

```sh
cargo bench
```

As a rough guide on a modern desktop, parsing a simple unit takes tens of
nanoseconds and a full conversion a few hundred; the unit tables are built once,
lazily, and shared immutably thereafter.

## Minimum supported Rust version

Rust **1.89** (edition 2024).

## License and attribution

The crate's own source code is under the **MIT license** (see the `LICENSE`
file).

This crate bundles two data files, each under its own license:

- **`vendor/ucum-essence.xml`**: the machine-readable UCUM definitions,
  © Regenstrief Institute, Inc. and the UCUM Organization, redistributed
  verbatim under the UCUM Copyright Notice and License v1.1 (see
  `vendor/UCUM-LICENSE.md`). It is unmodified, and is parsed at build time to
  generate the unit and prefix tables.
- **`vendor/UcumFunctionalTests.xml`**: the conformance test suite,
  © Grahame Grieve and contributors, under the Eclipse Public License 1.0.

UCUM is a standard of the Regenstrief Institute and the UCUM Organization
(<https://ucum.org>). This project is independent and is not affiliated with or
endorsed by them. With thanks to the UCUM maintainers for the specification and
the open data that make this crate possible.
