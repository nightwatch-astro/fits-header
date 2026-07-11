# Research & Design Decisions

Design decisions for `fits-header`. The spec carries no open clarifications; this record fixes
the *how* before implementation.

## 1. Card value representation

**Decision**: A card holds `value: Value`, where `Value` is `Str(String)` (a quoted string card,
content unescaped) or `Literal(String)` (an unquoted token — numbers, `T`/`F`, kept verbatim).

**Rationale**: Round-trip fidelity (FR-023) requires knowing whether a value was quoted, because
the writer formats string cards (single-quoted, padded) differently from literals (right-justified).
A bare `value: String` cannot distinguish the string `"123"` from the number `123`. The two-variant
enum records exactly the one bit that matters and keeps the writer total.

**Alternatives considered**: (a) `value: String` with quotes retained in the stored text — encodes
quoting via a magic leading `'`, but forces every read to re-scan/unescape and is easy to corrupt.
(b) A fully typed value enum (Int/Float/Bool/Str/…) — pushes FITS's untyped-text reality into the
parser, which must then guess types eagerly; rejected in favor of parsing on demand at `get::<T>()`.

## 2. Typed reads — one generic accessor

**Decision**: `Header::get<T: FromCard>(&self, keyword: &str) -> Option<T>` over a
`FromCard { fn from_card(&Card) -> Option<Self> }` trait, with impls for `f64`, `i64`, `u32`, `bool`,
`String`, and `time::PrimitiveDateTime`. Named wrappers (`get_f64`, `get_i64`, `get_u32`, `get_bool`)
delegate to it. `get_str(&self, keyword) -> Option<&str>` is a borrowing convenience that returns the
`Str` content without allocating.

**Rationale**: One method, one place for conversion logic; the caller states intent through `T` and
gets `None` on mismatch. Adding a new readable type is a trait impl, not a new method.

**Alternatives considered**: Five-plus concrete getters (duplicated absence/parse logic); a bare
`get() -> &str` (pushes all lenient numeric/bool/date parsing onto callers). Rejected.

## 3. Duplicate keywords

**Decision**: Reads and single-keyword updates act on the **first** occurrence; order is preserved.
Deletes remove **all** occurrences of the keyword.

**Rationale**: Deterministic and matches the "first-seen wins" read semantics in the spec. Deleting
all occurrences is the least surprising outcome of "remove this keyword".

## 4. Atomic batch mutations

**Decision**: `set_many` / `remove_many` validate every entry first, then apply all or none
(FR-011a). Validation rejects a keyword longer than 8 characters or containing bytes outside the
FITS keyword set (`A–Z`, `0–9`, `-`, `_`). On rejection the `Header` is left untouched and a
`FitsError` is returned.

**Rationale**: A partially-applied batch leaves the header in an unintended state the caller cannot
easily reason about. Validate-then-commit is cheap for in-memory cards.

## 5. Parsing

**Decision**: Byte-level, single pass. Walk 80-byte cards; take the keyword from columns 1–8
(trimmed); treat a card as a value card when columns 9–10 are `= `. Classify the value: a leading
`'` (after the indicator) is a `Str` (consume to the closing quote, unescaping `''`→`'`, trim
trailing spaces, empty→`Str("")`); otherwise cut at an inline ` /` comment and keep the trimmed
token as `Literal`. Stop at `END`. Skip `HIERARCH`, `COMMENT`, `HISTORY`, and blank cards. Bytes
are treated as ASCII; non-ASCII is handled without panicking. A trailing partial card is ignored.

**Rationale**: FITS headers are fixed-width ASCII; byte indexing is simpler and faster than
tokenizing and avoids UTF-8 assumptions. Leniency (skip, don't fail) satisfies the "malformed cards
are skipped" assumption.

## 6. Writing

**Decision**: `Header::to_bytes(&StructuralHints) -> Vec<u8>`. Emit, in order: structural cards
`SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2` from `StructuralHints`; then each card as an 80-char
record (keyword left-justified cols 1–8, `= ` cols 9–10, `Str` single-quoted with `'`→`''` and padded
to ≥8 chars inside the quotes, `Literal` right-justified to column 30, optional ` / comment`); then
`END`; pad the header block with spaces to a multiple of 2880; append a minimal data block padded to
2880. `StructuralHints::default()` describes a 1×1 8-bit image (`BITPIX=8`, `NAXIS=2`,
`NAXIS1=NAXIS2=1`).

**Rationale**: Mirrors the read rules exactly so `parse(to_bytes(h))` round-trips. A value that would
overflow the 80-column card is truncated to fit (documented); over-long values are prevented earlier
by batch validation for the common path.

## 7. Error type

**Decision**: One `FitsError` enum (via `thiserror`) with variants for the fallible operations:
`KeywordTooLong`, `InvalidKeyword`. `parse` returns `Result<Header, FitsError>` for signature
stability; today it yields `Ok` for any input (leniency), reserving `Err` for a future strict mode.
`to_bytes` is infallible.

**Rationale**: A single small error type keeps the surface simple. Keeping `parse` fallible avoids a
breaking signature change if strict validation is added later.

## 8. Dates and MJD

**Decision**: `impl FromCard for time::PrimitiveDateTime` parses FITS `YYYY-MM-DDThh:mm:ss[.fff]`
(timezone-naive civil time). Free functions `mjd_to_datetime(f64) -> PrimitiveDateTime` and
`datetime_to_mjd(&PrimitiveDateTime) -> f64` convert via the Julian day: `MJD = JD − 2_400_000.5`,
using `time::Date::to_julian_day()` / `from_julian_day()` for the date part plus the intra-day
fraction from the time part.

**Rationale**: `time` provides Julian-day conversion and format-description parsing without C deps.
Civil (UTC-implied) semantics match FITS `DATE-OBS`; leap seconds and TAI/TT are out of scope.

## 9. Sexagesimal helpers

**Decision**: `sexagesimal_ra_to_deg` / `sexagesimal_dec_to_deg` accept space- or colon-separated
`H M S` / `±D M S` with optional fractional seconds; Dec preserves the sign even when degrees are `0`
(parse the sign from the leading token, not from the numeric degree value). `deg_to_sexagesimal_ra` /
`deg_to_sexagesimal_dec` format back with fixed fractional-second precision so the string re-parses to
the original degrees within that precision. `parse_f64` / `parse_i64` accept decimal-form integers
(`"20.0"` → `20`).

**Rationale**: Real headers mix separators and fractional seconds. Sign-from-token is the only way to
represent `-0.5°` as `-00 30 00`.

## 10. Optional serialization

**Decision**: An off-by-default `serde` feature derives `Serialize`/`Deserialize` on `Header`, `Card`,
`Value`, and `StructuralHints` (with `time/serde` enabled for datetime fields). No effect on the
default build.

**Rationale**: Consumers who serialize a header to JSON/etc. opt in; others incur no dependency or
compile cost.

## 11. Testing strategy

**Decision**: `proptest` generates arbitrary valid headers (random valid keywords, `Str`/`Literal`
values, optional comments) and asserts: `parse(to_bytes(h))` yields the same ordered cards; every
emitted card is exactly 80 bytes; total length is a multiple of 2880. Spec acceptance scenarios
(US1–US4) become example tests; helper boundary cases (RA `10 00 00`→150, Dec `-00 30 00`→−0.5,
`20.0`→20, date and MJD round-trips) are asserted directly.

**Rationale**: The round-trip contract is a property, not a handful of examples; property testing
covers the input space the examples cannot.
