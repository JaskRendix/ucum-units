# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Logarithmic / non-linear special-unit conversion.** `convert` now handles
  `B`, `dB`, `Np`, `[pH]`, sound-pressure bels, prism-dioptre/percent-slope, and
  homeopathic potencies via per-function forward/inverse pairs. Metric prefixes
  on logarithmic units (e.g. `dB`) correctly fold into the function argument.
- **Correct arbitrary-unit semantics.** `[iU]`, `[arb'U]`, and other arbitrary
  units are now incommensurable with everything (including each other);
  `is_comparable` returns `false` and `convert` returns `UnsupportedSpecial`.
- **Case-insensitive (`c/i`) mode** via the new `Ucum` facade
  (`Ucum::case_insensitive()`), matching against the upper-case `CODE` column.
- **Display-name generation** (`display_name`), e.g. `mm` → `(millimeter)`.
- **Quantity arithmetic** (`Quantity`): value + unit with `mul`, `div`, and
  `convert_to`.
- Conformance suite now asserts **all 573 cases with zero skips** (validation,
  conversion, display-name, multiplication, and division).
- A [Criterion] benchmark suite (`cargo bench`) covering parsing, validation,
  analysis, conversion, and display-name generation.

[Criterion]: https://crates.io/crates/criterion

### Changed

- Conversion model generalized from linear-only `(factor, offset)` to support
  logarithmic functions; the `Analysis` public surface is unchanged.

## [0.1.0] - 2026-06-02

### Added

- Initial release.
- Total, bounded recursive-descent parser for the UCUM case-sensitive (`c/s`)
  grammar: multiplication (`.`), division (`/`), leading-slash reciprocal,
  grouping, exponent suffixes (`m2`, `s-1`), the dimensionless unity `1`,
  numeric factors, the `10*`/`10^` exponent forms, `%`, and annotations
  (`{…}`). Guaranteed to terminate on every input (step- and depth-bounded).
- Prefix and atom tables generated at build time from the verbatim, vendored
  `ucum-essence.xml` (version 2.2); no XML/serialization dependency at runtime.
- `Dimension`, a saturating `[i8; 7]` exponent vector with `Display`.
- Public API: `parse`, `validate`, `analyze`, `is_comparable`, `convert`,
  `canonical`, plus the `Analysis`, `UnitExpr`, and `UcumError` types.
- Affine conversion for temperature units (`Cel`, `[degF]`, `[degRe]`).
- Logarithmic and arbitrary special units parse and analyze; `convert` returns
  `UnsupportedSpecial` rather than a wrong number.
- Official UCUM functional-test conformance harness: 529/529 validation and
  30/30 conversion cases pass; display-name and quantity-algebra sections are
  out of scope and skipped with logged reasons.
- Property tests (`proptest`) and a `cargo-fuzz` target proving totality.
- `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`.

[Unreleased]: https://github.com/hulo-ai/ucum-units/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/hulo-ai/ucum-units/releases/tag/v0.1.0
