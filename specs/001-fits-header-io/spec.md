# Feature Specification: Generic FITS Header Read/Write & CRUD

**Feature Branch**: `feat/fits-header-io`

**Created**: 2026-07-11

**Status**: Draft

**Input**: User description: "Generic, pure-Rust (MSVC-safe, minimal deps) publishable FITS header read/write library for the `fits-header` crate — extract all header cards from a FITS file, CRUD single or multiple header keywords, serialize the header back into a valid FITS object, with coordinate/date/number helpers and a round-trip guarantee."

## User Scenarios & Testing *(mandatory)*

The consumers of this feature are **developers** integrating FITS metadata handling
into their own tools (the immediate consumer being a thin metadata adapter in
`nightwatch-astro/alm`). The "interface" is a library API; each user story below is a
standalone slice that can be built, tested, and demonstrated on its own.

### User Story 1 - Extract every header card from a FITS file (Priority: P1)

A developer has the raw bytes of a FITS file and needs to read all of its header
metadata — every keyword, its value, and any inline comment — in the order they appear.

**Why this priority**: Reading is the foundational capability and an immediately useful
MVP on its own: it lets a caller inspect a FITS file's metadata (the existing alm use
case). Nothing else in the feature is demonstrable without a header to work with.

**Independent Test**: Feed the byte content of a representative FITS file to the parser
and assert that the returned header contains the expected keywords, values, and comments,
in file order, stopping at the `END` card.

**Acceptance Scenarios**:

1. **Given** the bytes of a FITS file whose primary header contains
   `OBJECT  = 'M31     '`, **When** the caller parses the bytes, **Then** the header
   exposes a card with keyword `OBJECT` and string value `M31`.
2. **Given** a header card `EXPTIME = 120.0 / seconds`, **When** parsed, **Then** the
   numeric value `120.0` is readable and the inline comment `seconds` is not part of the value.
3. **Given** a header containing `COMMENT`, `HISTORY`, and `HIERARCH …` cards, **When**
   parsed, **Then** those cards are ignored and do not appear as retrievable keywords.
4. **Given** bytes with trailing content after the `END` card, **When** parsed, **Then**
   parsing stops at `END` and later bytes do not add cards.

---

### User Story 2 - Create, update, and delete keywords — single and multiple (Priority: P2)

A developer holds a parsed (or newly built) header and needs to add new keywords (including
arbitrary vendor-specific ones), change existing values/comments, and remove keywords —
operating either on one keyword at a time or on several in a single call.

**Why this priority**: This is the "CRUD within a file" capability. It builds directly on
the header produced by US1 and, together with US3, enables editing a FITS file's metadata.

**Independent Test**: Starting from a header (parsed or empty), perform create/update/delete
operations on single and multiple keywords and assert the resulting ordered card set matches
expectations.

**Acceptance Scenarios**:

1. **Given** a header without `FILTER`, **When** the caller sets `FILTER` to `Ha`, **Then**
   a new card `FILTER='Ha'` is appended and readable.
2. **Given** a header with `GAIN = 100`, **When** the caller sets `GAIN` to `120`, **Then**
   the existing card is updated in place (order preserved) and reads back as `120`.
3. **Given** a header, **When** the caller applies a batch of several keyword updates in one
   call, **Then** all named keywords reflect their new values and untouched cards are unchanged.
4. **Given** a header containing `TEMP` and `NOTES`, **When** the caller removes both in one
   call, **Then** neither keyword is retrievable and the remaining cards keep their order.
5. **Given** a caller needs a non-standard keyword (vendor quirk), **When** they set that
   arbitrary keyword, **Then** it is stored and later serialized without special-casing.

---

### User Story 3 - Serialize a header back into a valid FITS object (Priority: P2)

A developer needs to turn a header (parsed and possibly edited, or freshly built) back into
bytes that form a valid FITS object, so the header can be written to a file or fed to another
FITS-aware tool.

**Why this priority**: Completes the read → edit → write round-trip. On its own it delivers
value (produce a minimal valid FITS object from a set of keywords); combined with US1/US2 it
enables full in-place metadata editing.

**Independent Test**: Build a header, serialize it with structural hints, and assert the byte
output is well-formed (80-byte cards, structural cards present, `END` present, length a
multiple of 2880) and that re-parsing it reproduces the original cards.

**Acceptance Scenarios**:

1. **Given** a header with several keywords, **When** serialized, **Then** the output begins
   with the structural cards `SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2`, followed by the
   header's cards, followed by `END`.
2. **Given** any header, **When** serialized, **Then** every emitted card is exactly 80 bytes
   and the total serialized length is a whole multiple of 2880 bytes.
3. **Given** default structural hints, **When** serialized, **Then** the structural cards
   describe a 1×1 8-bit image and a minimal data block follows the header.
4. **Given** a header, **When** it is serialized and the result re-parsed, **Then** the
   re-parsed header contains the same keyword/value/comment cards as the original
   (round-trip equality for representative headers).

---

### User Story 4 - Interpret and format coordinate, numeric, and date values (Priority: P3)

A developer needs to interpret common FITS value encodings and convert them both ways:
sexagesimal right ascension / declination (read *and* write), numeric strings that use a
decimal form for integer quantities, ISO-8601 date/time keywords, and Modified Julian Dates.

**Why this priority**: A convenience layer on top of the header. Useful but not required to
read, edit, or write raw headers; it can ship after the core.

**Independent Test**: Call the helper functions (and the datetime-typed read) with
representative inputs and assert the converted results, including sign handling at the zero
boundary and round-trip between sexagesimal strings and degrees.

**Acceptance Scenarios**:

1. **Given** an RA string `10 00 00` (or `10:00:00`, or with fractional seconds), **When**
   converted, **Then** the result is `150.0` degrees (hours × 15).
2. **Given** a Dec string `-00 30 00`, **When** converted, **Then** the result is `-0.5`
   degrees — the negative sign is preserved even though the degrees field is `0`.
3. **Given** the degree value `150.0`, **When** formatted as sexagesimal RA, **Then** the
   result is a string that parses back to `150.0` (value-level round-trip).
4. **Given** a numeric string `20.0` where an integer is expected, **When** parsed as an
   integer, **Then** the result is `20`.
5. **Given** a card `DATE-OBS= '2026-07-11T22:15:03'`, **When** read as a datetime, **Then**
   the caller receives a parsed date/time value; formatting it back yields the same string.
6. **Given** `MJD-OBS = 60867.0` and its matching `DATE-OBS`, **When** the MJD is converted
   to a calendar date, **Then** it agrees with `DATE-OBS` (and the inverse conversion holds).

---

### Edge Cases

- **Empty string value** (`KEYWORD = '        '`): treated as present-but-empty → the card has
  no meaningful string value rather than a string of spaces.
- **Doubled single quotes inside a string** (`'O''Brien'`): unescaped to `O'Brien` on read and
  re-escaped on write so the value round-trips.
- **Missing structural information on write**: structural hints default to a 1×1 8-bit image so
  serialization always yields a valid FITS object.
- **Keyword longer than 8 characters / non-standard keyword**: standard cards use the trimmed
  8-character keyword field; longer/HIERARCH-style keys are ignored on read, but arbitrary ≤8
  keys can be written via the escape hatch.
- **Duplicate keywords in a header**: reads and single-keyword updates act on the first
  occurrence (first-seen wins); order is otherwise preserved.
- **No `END` card present**: parsing consumes the available cards and stops at end of input.
- **Typed read of a value that does not match the requested type** (e.g. non-numeric text via a
  numeric read, or an unparseable `DATE-OBS`): the typed read reports absence rather than a wrong
  value; it never panics.
- **Sexagesimal with mixed separators / fractional seconds** (`10:00:00.5` vs `10 00 00.5`):
  both are accepted.

## Requirements *(mandatory)*

### Functional Requirements

**Reading (extract all header cards)**

- **FR-001**: The library MUST parse a caller-provided byte buffer into an ordered header of
  cards, each card carrying a keyword, a value, and an optional comment.
- **FR-002**: The library MUST read the header as 80-byte cards within 2880-byte blocks and MUST
  stop parsing at the `END` card.
- **FR-003**: The library MUST ignore `HIERARCH`, `COMMENT`, and `HISTORY` cards (they do not
  become retrievable keywords).
- **FR-004**: The library MUST interpret single-quoted string values: unescape doubled `''` to a
  single `'`, trim trailing spaces, and treat an all-blank string as having no value.
- **FR-005**: The library MUST extract numeric values by stripping a leading `= ` and surrounding
  spaces and cutting the value at an inline ` /` comment before trimming.
- **FR-006**: Keyword lookups MUST match on the exact case of the trimmed 8-character keyword field.

**Header data structure & CRUD**

- **FR-007**: The library MUST expose a header data structure that is an ordered sequence of cards
  `{ keyword, value, comment? }`, allowing callers to read the cards out, mutate them, and put
  them back for serialization.
- **FR-008**: The library MUST provide a single generic typed read, `get::<T>(keyword)`, that
  interprets a card's value as the requested type via an extensible conversion trait, supporting at
  least text, 64-bit float, 64-bit signed integer, 32-bit unsigned integer, boolean (FITS `T`/`F`),
  and a date/time type; it MUST return absence when the keyword is missing or the value cannot be
  interpreted as the requested type. Named convenience wrappers (`get_str`, `get_f64`, `get_i64`,
  `get_u32`, `get_bool`) MUST delegate to it.
- **FR-009**: The library MUST allow creating/inserting a card for an arbitrary keyword (an escape
  hatch for vendor-specific keys), setting its value and optional comment.
- **FR-010**: The library MUST allow updating the value (and comment) of an existing keyword, both
  for a single keyword and for several keywords supplied together in one operation. Updating a
  keyword that is absent MUST create it.
- **FR-011**: The library MUST allow deleting a keyword, both for a single keyword and for several
  keywords supplied together in one operation.
- **FR-012**: Card order MUST be preserved: in-place updates keep a card's position, newly created
  cards are appended, and order is retained through a serialize→parse round-trip.

**Writing (serialize to a valid FITS object)**

- **FR-013**: The library MUST serialize the header to 80-character cards with the keyword
  left-justified in columns 1–8, `= ` in columns 9–10, string values single-quoted and padded to
  at least 8 characters inside the quotes, numeric values right-justified to column 30, and an
  optional ` / comment` suffix.
- **FR-014**: The library MUST prepend the structural cards `SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`,
  `NAXIS2` derived from caller-supplied structural hints, defaulting to a 1×1 8-bit image when
  hints are not specified.
- **FR-015**: The library MUST append an `END` card, pad the header to a whole multiple of 2880
  bytes, and follow it with a minimal data block so the result is a valid FITS object.

**Coordinate, numeric & date helpers**

- **FR-016**: The library MUST convert a sexagesimal right-ascension string to degrees (hours × 15),
  accepting both space- and colon-separated forms and fractional seconds.
- **FR-017**: The library MUST convert a sexagesimal declination string to degrees, accepting both
  space- and colon-separated forms and fractional seconds, and preserving the sign even when the
  degrees field is `0`.
- **FR-018**: The library MUST format a degree value back into a sexagesimal right-ascension and a
  sexagesimal declination string, such that the formatted string parses back to the original degrees
  (value-level round-trip within a documented precision).
- **FR-019**: The library MUST provide lenient numeric parsing that accepts a decimal-form string
  (e.g. `20.0`) where an integer is expected and yields the integer value (`20`).
- **FR-020**: The library MUST parse FITS date keywords (`DATE-OBS`, `DATE-LOC`, `DATE-END`, in
  ISO-8601 `YYYY-MM-DDThh:mm:ss[.fff]` form) into a date/time value and format such a value back to
  the same string.
- **FR-021**: The library MUST convert between a Modified Julian Date (`MJD-OBS`, `MJD-AVG`) and a
  calendar date/time.

**Non-functional constraints**

- **FR-022**: The library MUST use only pure-Rust, MSVC-safe dependencies (no C or system
  libraries) and MUST remain publishable to crates.io. The runtime dependency set is limited to a
  date/time library and an error-handling library; an optional off-by-default `serde` capability may
  add derive support.
- **FR-023**: The library MUST guarantee round-trip fidelity: re-parsing a serialized header
  reproduces the same keyword/value/comment cards for representative headers, every emitted card is
  exactly 80 bytes, the serialized header is padded to a 2880-byte multiple, and string, numeric,
  sexagesimal, and date values survive the round-trip.
- **FR-024**: The library MUST NOT contain application-specific domain types; it exposes only a
  generic header. Any field↔keyword mapping to domain models is the responsibility of a downstream
  adapter.
- **FR-025**: The library MUST offer `Serialize`/`Deserialize` for its public data types behind an
  optional, disabled-by-default `serde` feature, so consumers who do not need it incur no cost.

### Key Entities *(include if feature involves data)*

- **Header**: An ordered collection of cards representing one FITS header unit. Supports reading
  cards out, a generic typed read (`get::<T>`), CRUD over keywords, and serialization back to bytes.
- **Card**: A single header entry — a keyword (≤8-character standard field), a value (as text),
  and an optional inline comment.
- **Structural Hints**: Caller-supplied description of the FITS object's structure used only when
  writing (image dimensions and bit depth), defaulting to a 1×1 8-bit image.
- **Value conversion trait**: The extension point behind `get::<T>()` that turns a card's text into
  a requested Rust type (text, numbers, boolean, date/time). `Header`, `Card`, and `Structural
  Hints` gain `Serialize`/`Deserialize` when the `serde` feature is enabled.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Given a representative FITS file, 100% of the header cards preceding `END` (excluding
  ignored `HIERARCH`/`COMMENT`/`HISTORY`) are extracted with their correct keyword, value, and comment.
- **SC-002**: For representative headers, serializing a header and re-parsing the result yields an
  identical ordered set of keyword/value/comment cards (round-trip equality), verified by
  property-based tests over generated headers.
- **SC-003**: For any serialized header, every card is exactly 80 bytes and the total serialized
  length is a whole multiple of 2880 bytes.
- **SC-004**: Creating, updating, and deleting keywords — exercised for both a single keyword and a
  batch of several — produces the expected header state in 100% of the defined CRUD scenarios.
- **SC-005**: The coordinate/numeric helpers produce correct results on the boundary cases:
  RA `10 00 00` → `150.0`, Dec `-00 30 00` → `-0.5` (sign preserved at 0°), integer parse of
  `20.0` → `20`, and formatting `150.0` back to a sexagesimal RA that re-parses to `150.0`.
- **SC-006**: FITS date keywords round-trip (`DATE-OBS` string → date/time → identical string), and
  an `MJD-OBS` value converts to a calendar date that agrees with its matching `DATE-OBS`.
- **SC-007**: The library builds and passes its test suite using only pure-Rust dependencies (no C
  dependencies) on Linux, Windows, and macOS, with default features and with the `serde` feature enabled.

## Assumptions

- **Scope is a single (primary) header unit.** "All headers" means all cards of one header unit up
  to `END`. Multi-extension FITS files (additional HDUs beyond the primary) are out of scope for
  this feature; the parser reads one header and the serializer writes one header plus a minimal data
  block. This matches the single-`END` read/write described in the brief and the source parser.
- **Input is an in-memory byte buffer** supplied by the caller; file I/O (opening/reading paths) is
  the caller's responsibility, not this library's.
- **FITS conformance**: input follows the FITS convention of 80-byte cards in 2880-byte blocks;
  malformed cards are skipped/ignored rather than causing the whole parse to fail.
- **Duplicate keywords**: reads and single-keyword updates operate on the first occurrence;
  bulk operations address distinct keywords.
- **Values are held as text** and interpreted on demand by the generic typed read and helpers.
- **Dependency policy**: the crate is not zero-dependency but stays pure-Rust and MSVC-safe.
  Date/time is provided by the `time` crate (aligned with alm), errors by `thiserror`, optional
  serialization by `serde`, and property testing by `proptest` (dev-only).
- **Date/time semantics**: FITS date keywords are treated as timezone-naive civil date/times (UTC
  implied); leap seconds and non-UTC time scales (TAI/TT) are not modeled.
- **License/packaging**: the crate is already scaffolded (Apache-2.0, publishable). The project
  constitution is currently the unratified template, so no additional governance constraints apply
  beyond those stated here.

## Out of Scope

- Parsing or interpreting FITS **data** (image arrays, tables) beyond emitting the minimal
  placeholder data block required for a valid object on write.
- **Multi-extension HDU** traversal (headers of extension units beyond the primary).
- Any **application-specific field↔keyword mapping** (e.g. mapping `EXPTIME` → a domain
  `exposure` field); that belongs to a downstream adapter such as alm's `crates/metadata/fits`.
- Full **astronomical time scales** (TAI/TT/leap-second-precise epochs); only civil ISO-8601 and
  MJD↔calendar conversion are in scope.
- File-system access, compression, checksum verification, and network I/O.
