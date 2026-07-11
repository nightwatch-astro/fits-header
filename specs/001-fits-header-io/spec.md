# Feature Specification: Generic FITS Header Read/Write & CRUD

**Feature Branch**: `feat/fits-header-io`

**Created**: 2026-07-11

**Status**: Draft

**Input**: A pure-Rust, MSVC-safe FITS header library that faithfully reads, edits, and writes one
FITS header unit — preserving every card byte-for-byte except the ones the caller changes — with
strict keyword access, atomic batch edits, date helpers, and a round-trip guarantee.

## User Scenarios & Testing *(mandatory)*

The consumers of this feature are **developers** integrating FITS metadata handling into their own
tools. The interface is a library API; each user story below is a standalone slice that can be built,
tested, and demonstrated on its own.

### User Story 1 - Read a header faithfully (Priority: P1)

A developer parses the bytes of a FITS file and reads its header metadata — every keyword, value, and
comment, in order — while nothing is silently discarded.

**Why this priority**: Reading is the foundation and a useful MVP: inspect a file's metadata. Faithful
retention (every card kept, in order) is what later lets an edit be written back without corrupting the
file.

**Independent Test**: Parse a representative header and assert the expected keywords/values/comments
are readable in order; commentary and vendor cards are retained; `CONTINUE`-continued values read as
one logical value.

**Acceptance Scenarios**:

1. **Given** a header with `OBJECT = 'M31     '`, **When** parsed, **Then** `get::<String>("OBJECT")`
   is `Some("M31")`.
2. **Given** `EXPTIME = 120.0 / seconds`, **When** parsed, **Then** `get::<f64>("EXPTIME")` is
   `Some(120.0)` and the comment is not part of the value.
3. **Given** a header containing `COMMENT`, `HISTORY`, and `HIERARCH` cards, **When** parsed, **Then**
   those cards are **retained** (not dropped) but are not addressable as ordinary 8-character keywords.
4. **Given** a long string value split across `CONTINUE` cards, **When** parsed, **Then**
   `get::<String>` returns the reassembled value.
5. **Given** bytes after the `END` card, **When** parsed, **Then** parsing of the header stops at `END`.

---

### User Story 2 - Edit keywords with strict, unambiguous CRUD (Priority: P1)

A developer creates, updates, and deletes keywords — by name for the common unique case, and by an
explicit occurrence when a keyword repeats — and the library refuses ambiguous operations rather than
guessing.

**Why this priority**: Editing is the core purpose. Strictness prevents silently mutating the wrong
card; atomic batches keep the header consistent on failure.

**Independent Test**: On a parsed or empty header, exercise create/update/delete by name and by
occurrence, batch operations, and the ambiguity guards; assert the resulting ordered records.

**Acceptance Scenarios**:

1. **Given** a header with a single `GAIN`, **When** `set("GAIN", 120)`, **Then** that card is updated
   in place; **When** `set("FILTER", "Ha")` and `FILTER` is absent, **Then** a card is appended.
2. **Given** a keyword that appears more than once, **When** `get`/`set`/`remove` is called with the
   bare name, **Then** it returns `Err(AmbiguousKeyword)`; **When** called with `("GAIN", 1)`, **Then**
   it targets exactly the second occurrence.
3. **Given** repeatable `HISTORY` cards, **When** `append("HISTORY", "calibrated")`, **Then** a new
   history line is added; `get_all::<String>("HISTORY")` returns all lines in order.
4. **Given** a batch update where one entry has an invalid keyword, **When** applied, **Then** the whole
   batch is rejected and the header is unchanged (atomic).
5. **Given** an arbitrary vendor keyword, **When** set, **Then** it is stored and later serialized.

---

### User Story 3 - Write back without corrupting the file (Priority: P1)

A developer serializes an edited header. Cards left untouched come back byte-for-byte identical; only
created or modified cards are re-rendered. The header can be produced either as a standalone FITS
object or as a header block to splice back onto an existing file's data.

**Why this priority**: Faithful write-back is what makes in-place editing safe. Byte-exact preservation
keeps diffs minimal and leaves vendor formatting and the file's image data intact.

**Independent Test**: Parse, edit one keyword, serialize, and assert every untouched card is byte-equal
to its input and re-parsing reproduces the header; assert 80-byte cards and 2880-byte padding.

**Acceptance Scenarios**:

1. **Given** a parsed header, **When** one keyword is edited and `to_header_bytes()` is produced,
   **Then** every card except the edited one is byte-identical to the input, and re-parsing yields an
   equal header.
2. **Given** any header, **When** serialized, **Then** every emitted card is exactly 80 bytes and the
   output length is a multiple of 2880.
3. **Given** a from-scratch header without `SIMPLE`, **When** `to_bytes(&StructuralHints::default())`,
   **Then** the mandatory `SIMPLE/BITPIX/NAXIS/NAXIS1/NAXIS2` cards are synthesized and a minimal data
   block follows; **Given** a parsed header that already has them, **Then** they are not duplicated.
4. **Given** a string value longer than one card, **When** serialized, **Then** it is written across
   `CONTINUE` cards with a `LONGSTRN` announcement, and re-parsing reassembles it.

---

### User Story 4 - Interpret and format numbers and dates (Priority: P2)

A developer converts decimal-form integers and ISO-8601 dates.

**Why this priority**: A convenience layer over the header; useful but not required to read, edit, or
write raw cards.

**Independent Test**: Call the helpers and the datetime-typed read with representative inputs and assert
results, including date round-trips.

**Acceptance Scenarios**:

1. **Given** `20.0` where an integer is expected, **When** parsed, **Then** `20`.
2. **Given** `DATE-OBS = '2026-07-11T22:15:03'`, **When** read as a datetime, **Then** a parsed value;
   formatting it back yields the same string.

---

### Edge Cases

- **Empty string value** (`KEYWORD = '        '`): present-but-empty; `get::<String>` yields `None`.
- **Doubled single quotes** (`'O''Brien'`): unescaped on read, re-escaped on write.
- **Duplicate keyword**: bare-name `get`/`set`/`remove` return `Err(AmbiguousKeyword)`; use `("NAME", n)`.
- **Commentary** (`COMMENT`/`HISTORY`/blank): repeatable free-text keywords; addressed via `get_all`,
  `append`, and `("NAME", n)`.
- **`HIERARCH` / non-standard / blank cards**: retained verbatim, not addressable as 8-char keywords.
- **`CONTINUE` run**: a value plus its trailing `CONTINUE` cards is one logical value; editing or
  removing the value replaces or removes the whole run.
- **No `END` card**: parsing stops at end of input.
- **Type mismatch on a typed read**: returns `None`, never panics.

## Requirements *(mandatory)*

### Functional Requirements

**Reading**

- **FR-001**: Parse a byte buffer into an ordered header of records, each retaining its original 80
  bytes so untouched records serialize byte-for-byte.
- **FR-002**: Read 80-byte cards in 2880-byte blocks; stop at `END`.
- **FR-003**: Retain `HIERARCH`, `COMMENT`, `HISTORY`, blank, and unrecognized cards for fidelity;
  `HIERARCH`/blank/unrecognized are not addressable as 8-character keywords.
- **FR-004**: Interpret single-quoted strings: unescape `''`→`'`, trim trailing spaces, all-blank → no
  value.
- **FR-005**: Extract numeric/logical values by stripping the `= ` indicator and cutting an inline ` /`
  comment before trimming.
- **FR-006**: Reassemble a value card and its trailing `CONTINUE` cards (long-string convention) into a
  single logical value on read.
- **FR-007**: Keyword lookup matches the exact-case trimmed 8-character keyword.

**Header data structure & access**

- **FR-008**: Expose a `Header` of ordered records; each record retains original bytes and a modified
  flag. Equality (`PartialEq`) compares semantically (keyword/value/comment), not bytes.
- **FR-009**: Provide one generic typed read, `get::<T>(key)`, over an extensible conversion trait,
  supporting text, `f64`, `i64`, `u32`, `bool` (`T`/`F`), and a date/time type; return `None` on absent
  or type mismatch.
- **FR-010**: Address records by a key that is either a bare name (strict) or `(name, occurrence)`.
  Bare-name `get`/`set`/`remove` MUST return `Err(AmbiguousKeyword)` when the name is duplicated; the
  `(name, occurrence)` form targets exactly one record.
- **FR-011**: `set(key, value)` updates the addressed record in place, or appends when the (unique) name
  is absent. `append(name, value)` always adds a record. `remove(key)` deletes the addressed record.
- **FR-012**: `get_all::<T>(name)` returns every value for a name in order; `count(name)` returns the
  number of occurrences. Commentary keywords (`COMMENT`/`HISTORY`/blank) use these same methods.
- **FR-013**: Writes are type-directed via a value-conversion trait: text → quoted string; numbers/bool
  → literal; wrappers `Literal(text)` (verbatim), `Fixed(value, decimals)`, and `Sci(value, digits)`
  control literal formatting. Default `f64` formatting is shortest round-trip, normalized to read as a
  float (decimal point or `E` exponent present).
- **FR-014**: Batch mutations validate every entry first and apply all-or-nothing (atomic); on any
  rejection the header is unchanged. Keyword validation: ≤8 characters, bytes in `A–Z 0–9 - _`.
- **FR-015**: Serialization emits untouched records byte-for-byte and re-renders only created/modified
  records; card order is preserved.

**Writing**

- **FR-016**: `to_header_bytes()` produces the header block only (cards, `END`, padded to a 2880
  multiple) for splicing onto existing file data.
- **FR-017**: `to_bytes(&StructuralHints)` produces a standalone FITS object: header block plus a
  minimal zero data block sized from the effective `BITPIX`/`NAXIS`. Mandatory structural cards
  (`SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2`) are synthesized only when absent; `StructuralHints`
  is a fallback, ignored when those cards are already present. Default hints describe a 1×1 8-bit image.
- **FR-018**: Render a value card as an 80-character record: keyword left-justified in columns 1–8,
  `= ` in 9–10, strings single-quoted with `'`→`''` and padded to ≥8 chars inside the quotes, literals
  right-justified to column 30, optional ` / comment`. Commentary keywords (`COMMENT`/`HISTORY`/blank)
  render their payload as free text (columns 9–80, no `=`).
- **FR-019**: Write a string value longer than one card across `CONTINUE` cards per the long-string
  convention and ensure a `LONGSTRN` announcement card is present; the run re-parses to the same value.

**Numeric & date helpers**

- **FR-021**: Provide lenient numeric parsing accepting decimal-form integers (`"20.0"` → `20`).
  (FR-020, sexagesimal coordinate conversion, was removed from scope: domain math belongs in a
  downstream astronomy crate.)
- **FR-022**: Parse FITS date keywords (`DATE-OBS`/`DATE-LOC`/`DATE-END`, ISO-8601 civil form) to a
  date/time value and format back.

**Non-functional**

- **FR-023**: Use only pure-Rust, MSVC-safe dependencies (no C or system libraries); remain publishable.
  `#![forbid(unsafe_code)]`. Reads never panic on malformed input.
- **FR-024**: Guarantee round-trip fidelity: for a parsed header, untouched cards serialize byte-for-byte
  and re-parsing yields a semantically equal header; every emitted card is 80 bytes; the header is padded
  to a 2880 multiple; strings, numerics, and dates round-trip.
- **FR-025**: Contain no application-specific domain types; the header is generic. An optional,
  disabled-by-default `serde` feature derives `Serialize`/`Deserialize` on the public data types.

### Key Entities

- **Header**: Ordered records for one header unit; generic typed reads, strict keyword/occurrence access,
  CRUD, and serialization. Retains original bytes for byte-exact preservation.
- **Record (Card)**: One 80-byte header line — a value card (keyword, value, comment), a commentary card
  (keyword, free text), or a preserved opaque card. Carries original bytes and a modified flag.
- **Value**: A value card's payload — a quoted string or an unquoted literal token.
- **Key**: A record selector — a bare name (strict/unique) or `(name, occurrence)`.
- **StructuralHints**: Image geometry/bit depth used only when synthesizing missing structural cards on
  write; defaults to a 1×1 8-bit image.

## Success Criteria *(mandatory)*

- **SC-001**: For a representative header, all cards preceding `END` are retained in order, and standard
  value keywords read back with correct value and comment.
- **SC-002**: For a parsed header with one keyword edited, serializing leaves every untouched card
  byte-identical to its input, and re-parsing yields a semantically equal header (verified by property
  tests over generated headers).
- **SC-003**: Every serialized card is exactly 80 bytes and the total length is a multiple of 2880.
- **SC-004**: CRUD by name and by occurrence — including atomic batch rejection and the ambiguity guards
  — produce the expected header state in 100% of the defined scenarios.
- **SC-005**: Numeric helpers: integer parse `20.0` → `20`.
- **SC-006**: Date keywords round-trip (`DATE-OBS` string → datetime → identical string).
- **SC-007**: A long string round-trips through `CONTINUE` cards (reassembled on read, re-split on write).
- **SC-008**: Builds and tests pass with only pure-Rust dependencies (no C) on Linux, Windows, and macOS,
  with default features and with `serde` enabled.

## Assumptions

- **Scope is a single (primary) header unit** up to `END`; multi-extension HDUs are out of scope.
- **Input is an in-memory byte buffer**; file I/O is the caller's. The header holds no image data — the
  standalone object uses a minimal data block, and in-place editing reattaches the caller's original data.
- **Malformed cards** are retained verbatim where possible and never cause a panic.
- **Dependencies** are pure-Rust and MSVC-safe: `time` (dates), `thiserror` (errors), `serde`
  (optional serialization), `proptest` (dev-only tests).
- **Date/time** are timezone-naive civil values; leap seconds and non-UTC time scales are not modeled.

## Out of Scope

- Parsing or interpreting FITS **data** (image arrays, tables) beyond the minimal write block.
- **Multi-extension HDU** traversal.
- Any **application-specific field↔keyword mapping**; that belongs to a downstream adapter.
- Full astronomical time scales (TAI/TT/leap seconds); only civil ISO-8601.
- **Coordinate/epoch conversions** (sexagesimal RA/Dec, MJD↔calendar); they belong in a downstream
  astronomy crate.
- File-system access, compression, checksum verification, and network I/O.
