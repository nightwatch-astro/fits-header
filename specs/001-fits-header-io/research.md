# Research & Design Decisions

Design decisions for `fits-header`, fixing the *how* before implementation.

## 1. Faithful editor, not a normalizer

**Decision**: `parse` retains every card in order — value, commentary (`COMMENT`/`HISTORY`/blank),
`HIERARCH`, and unrecognized cards. Nothing is dropped. Normalization is a caller concern.

**Rationale**: An editor must be able to change one keyword and write the file back without discarding
provenance or duplicating structure. Dropping commentary/structure makes faithful write-back impossible.

## 2. Byte-exact preservation via raw retention

**Decision**: Each record keeps its original 80 bytes plus a `modified` flag. `to_*_bytes` emits the raw
bytes for untouched records and the canonical formatter only for created/modified records. `PartialEq`
is semantic (keyword/value/comment); byte-equality is a separate test check.

**Rationale**: An edit changes exactly the records it touches — minimal diffs, vendor formatting and
exact numeric text preserved — and we never have to reproduce arbitrary vendor formatting from a model.

## 3. Record taxonomy

**Decision**: A `Record` is one of `Value { keyword, value, comment }`, `Commentary { keyword, text }`
(`COMMENT`/`HISTORY`/blank), or `Opaque` (`HIERARCH`/unrecognized). All carry raw bytes and a modified
flag. `Value` payload is `Value::Str(String)` (quoted, unescaped) or `Value::Literal(String)` (verbatim
token). Records are an internal detail; the public surface is accessor-first, with read-only `cards()`
as an escape hatch.

**Rationale**: Raw retention means opaque cards need no structural modeling. The `Str`/`Literal` split is
the one bit the writer needs to know (quote vs. right-justify).

## 4. Strict, unified keyword access

**Decision**: `get`/`set`/`remove` take `impl Into<Key>`, where `Key` is `Name(&str)` (strict) or
`Occurrence(&str, usize)`. Bare-name operations on a **duplicated** keyword return
`Err(AmbiguousKeyword)`; the `(name, n)` form selects one. `get_all::<T>(name)` and `count(name)` handle
the multi case; `append(name, value)` always adds. There is no `_at`/`_nth`/`positions` API.

**Rationale**: Refusing ambiguity prevents silently mutating the wrong card; the optional occurrence is
the selector. Commentary is just repeatable keywords, handled by the same methods.

## 5. Read/write value symmetry

**Decision**: Reads use `get::<T>()` over `FromCard` (impls: `String`, `f64`, `i64`, `u32`, `bool`,
`time::PrimitiveDateTime`). Writes use `impl IntoValue`: `&str`/`String` → `Str`; `f64`/`i64`/`u32`/
`bool` → `Literal`; wrappers `Literal(text)` (verbatim), `Fixed(v, decimals)`, `Sci(v, sig_digits)`.

**Rationale**: One accessor and one mutator, extended by trait impls and small wrapper types rather than
a method zoo.

## 6. Numeric formatting

**Decision**: Integers render as bare digits. Default `f64` uses shortest round-trip formatting,
normalized so it reads as a float: if the shortest form has no `.`/exponent, append `.0`; uppercase any
exponent to `E`. `Fixed(v, d)` → fixed decimals; `Sci(v, s)` → scientific with `s` significant digits.

**Rationale**: Shortest round-trip guarantees `get::<f64>` returns the same value; forcing a decimal
point keeps FITS int/float typing correct; `Fixed`/`Sci` give explicit control when wanted.

## 7. CONTINUE long strings

**Decision**: On read, a value card and its trailing `CONTINUE` cards are reassembled into one logical
string (strip the trailing `&`, concatenate); the run is retained for byte-exact passthrough when
untouched. On write, a new/edited string longer than one card is split across `CONTINUE` cards and a
`LONGSTRN` announcement card is ensured. Continuation cards are part of their value card, not independent
keywords — `count`/`get`/the occurrence key treat the run as one value; editing/removing replaces or
removes the whole run.

**Rationale**: Faithful editing of real files requires understanding `CONTINUE`; supporting it on write
too avoids a surprising length limit on string values.

## 8. Serialization outputs

**Decision**: `to_header_bytes()` emits the header block only (cards, `END`, 2880 padding) for splicing
onto existing file data. `to_bytes(&StructuralHints)` emits a standalone object (header + minimal zero
data block sized from `BITPIX`/`NAXIS`). Mandatory structural cards are synthesized only when absent;
`StructuralHints` is a fallback ignored when the cards are present. `StructuralHints::default()` is a
1×1 8-bit image.

**Rationale**: In-place editing must not fabricate or discard image data — `to_header_bytes` +
original data does that. `to_bytes` covers building a new object from scratch.

## 9. Error type

**Decision**: One `FitsError` (via `thiserror`): `AmbiguousKeyword { keyword, count }`,
`KeywordTooLong { keyword }`, `InvalidKeyword { keyword }`. `parse` returns `Result<Header, FitsError>`
for signature stability (lenient today). `to_*_bytes` are infallible.

**Rationale**: A single small error type; keeping `parse` fallible avoids a future breaking change.

## 10. Dates, MJD, sexagesimal

**Decision**: `impl FromCard for time::PrimitiveDateTime` parses `YYYY-MM-DDThh:mm:ss[.fff]` (civil).
`mjd_to_datetime`/`datetime_to_mjd` convert via Julian day (`MJD = JD − 2_400_000.5`). Sexagesimal
parsers accept space/colon separators and fractional seconds and preserve the declination sign at 0°
(sign taken from the leading token); formatters round-trip within fixed precision.

**Rationale**: `time` supplies Julian-day and format parsing without C deps; sign-from-token is the only
way to represent `-0.5°` as `-00 30 00`.

## 11. Optional serialization

**Decision**: An off-by-default `serde` feature derives `Serialize`/`Deserialize` on the public data
types (`time/serde` enabled with it).

**Rationale**: Consumers who serialize a header opt in; others pay nothing.

## 12. Testing strategy

**Decision**: `proptest` generates arbitrary headers and asserts (a) untouched cards serialize
byte-for-byte, (b) `parse(to_header_bytes(h))` is semantically equal to `h`, (c) every emitted card is
80 bytes, (d) total length is a multiple of 2880. Spec scenarios become example tests; helper boundary
cases are asserted directly.

**Rationale**: The round-trip contract is a property; property testing covers the input space examples
cannot.
