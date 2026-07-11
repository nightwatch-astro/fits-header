# Feature Specification: Generic FITS Header Read/Write & CRUD

**Feature Branch**: `feat/fits-header-io`

**Created**: 2026-07-11

**Status**: Draft

**Input**: User description: "Generic, dependency-free (std-only), publishable FITS header read/write library for the `fits-header` crate — extract all header cards from a FITS file, CRUD single or multiple header keywords, and serialize the header back into a valid FITS object, with coordinate/number helpers and a round-trip guarantee."

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

### User Story 4 - Convert coordinate and numeric header values (Priority: P3)

A developer needs to interpret common FITS value encodings: sexagesimal right ascension and
declination strings, and numeric strings that use a decimal form for integer quantities.

**Why this priority**: A convenience layer on top of the header. Useful but not required to
read, edit, or write headers; it can ship after the core.

**Independent Test**: Call the helper functions with representative inputs and assert the
converted numeric results, including sign handling at the zero boundary.

**Acceptance Scenarios**:

1. **Given** an RA string `10 00 00`, **When** converted, **Then** the result is `150.0`
   degrees (hours × 15).
2. **Given** a Dec string `-00 30 00`, **When** converted, **Then** the result is `-0.5`
   degrees — the negative sign is preserved even though the degrees field is `0`.
3. **Given** a numeric string `20.0` where an integer is expected, **When** parsed as an
   integer, **Then** the result is `20`.

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
- **Value present but not numeric when a numeric getter is used**: the typed getter reports
  absence rather than a wrong number.

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
- **FR-008**: The library MUST provide typed reads for a keyword as a string, a 64-bit float, a
  64-bit signed integer, and a 32-bit unsigned integer, returning absence when the keyword is
  missing or the value cannot be interpreted as the requested type.
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

**Coordinate & numeric helpers**

- **FR-016**: The library MUST provide a helper that converts a sexagesimal right-ascension string
  (`H M S`) to degrees by multiplying the hour value by 15.
- **FR-017**: The library MUST provide a helper that converts a sexagesimal declination string
  (`±D M S`) to degrees, preserving the sign even when the degrees field is `0`.
- **FR-018**: The library MUST provide lenient numeric parsing that accepts a decimal-form string
  (e.g. `20.0`) where an integer is expected and yields the integer value (`20`).

**Non-functional constraints**

- **FR-019**: The library MUST have zero third-party dependencies and use only the standard library.
- **FR-020**: The library MUST guarantee round-trip fidelity: re-parsing a serialized header
  reproduces the same keyword/value/comment cards for representative headers, every emitted card is
  exactly 80 bytes, the serialized header is padded to a 2880-byte multiple, and string, numeric,
  and sexagesimal values survive the round-trip.
- **FR-021**: The library MUST NOT contain application-specific domain types; it exposes only a
  generic header. Any field↔keyword mapping to domain models is the responsibility of a downstream
  adapter.

### Key Entities *(include if feature involves data)*

- **Header**: An ordered collection of cards representing one FITS header unit. Supports reading
  cards out, typed lookups, CRUD over keywords, and serialization back to bytes.
- **Card**: A single header entry — a keyword (≤8-character standard field), a value (as text),
  and an optional inline comment.
- **Structural Hints**: Caller-supplied description of the FITS object's structure used only when
  writing (image dimensions and bit depth), defaulting to a 1×1 8-bit image.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Given a representative FITS file, 100% of the header cards preceding `END` (excluding
  ignored `HIERARCH`/`COMMENT`/`HISTORY`) are extracted with their correct keyword, value, and comment.
- **SC-002**: For representative headers, serializing a header and re-parsing the result yields an
  identical ordered set of keyword/value/comment cards (round-trip equality).
- **SC-003**: For any serialized header, every card is exactly 80 bytes and the total serialized
  length is a whole multiple of 2880 bytes.
- **SC-004**: Creating, updating, and deleting keywords — exercised for both a single keyword and a
  batch of several — produces the expected header state in 100% of the defined CRUD scenarios.
- **SC-005**: The coordinate/numeric helpers produce correct results on the boundary cases:
  RA `10 00 00` → `150.0`, Dec `-00 30 00` → `-0.5` (sign preserved at 0°), and integer parse of
  `20.0` → `20`.
- **SC-006**: The library builds and passes its test suite with zero third-party dependencies on
  Linux, Windows, and macOS.

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
- **Values are held as text** and interpreted on demand by the typed getters and helpers.
- **License/packaging**: the crate is already scaffolded (Apache-2.0, publishable). The project
  constitution is currently the unratified template, so no additional governance constraints apply
  beyond those stated here.

## Out of Scope

- Parsing or interpreting FITS **data** (image arrays, tables) beyond emitting the minimal
  placeholder data block required for a valid object on write.
- **Multi-extension HDU** traversal (headers of extension units beyond the primary).
- Any **application-specific field↔keyword mapping** (e.g. mapping `EXPTIME` → a domain
  `exposure` field); that belongs to a downstream adapter such as alm's `crates/metadata/fits`.
- File-system access, compression, checksum verification, and network I/O.
