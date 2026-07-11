# Implementation Plan: Generic FITS Header Read/Write & CRUD

**Branch**: `feat/fits-header-io` | **Date**: 2026-07-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-fits-header-io/spec.md`

## Summary

`fits-header` is a single Rust library crate that reads, edits, and writes one FITS header
unit. Raw FITS bytes are parsed into an ordered `Header` of `(keyword, value, comment)` cards;
callers read values through one generic `get::<T>()` accessor, mutate keywords with single and
atomic-batch CRUD, and serialize the header back into a valid FITS object with a round-trip
guarantee. Coordinate, numeric, and date/MJD helpers sit on top. The crate is pure-Rust and
MSVC-safe.

## Technical Context

**Language/Version**: Rust (edition 2021), `rust-version = 1.74`.

**Primary Dependencies**: `time` 0.3 (`parsing`, `formatting`) for ISO-8601 dates and Julian-day
math; `thiserror` 2 for the error type; `serde` 1 behind an optional `serde` feature; `proptest` 1
as a dev-dependency.

**Storage**: None. Input and output are in-memory `&[u8]` / `Vec<u8>`; file I/O is the caller's.

**Testing**: `cargo test` — unit tests per module, doc-tests on public items, and `proptest`
property tests for the parse ↔ `to_bytes` round-trip. CI runs the suite with default features and
with `--features serde`.

**Target Platform**: Cross-platform, pure-Rust — Linux, Windows (MSVC), macOS.

**Project Type**: Single library crate.

**Performance Goals**: Linear single-pass parse and serialize; a header of a few thousand cards
(tens of KB) parses and serializes in well under a millisecond. No allocation-heavy hot paths.

**Constraints**: Pure-Rust, MSVC-safe, no C or system libraries. `#![forbid(unsafe_code)]`.
Malformed input never panics — it is skipped on read; fallible mutations return `Result`.

**Scale/Scope**: One primary header unit, up to a few thousand cards; public surface is small
(one `Header` type, `parse`, `to_bytes`, a value-conversion trait, and free-function helpers).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution (`.specify/memory/constitution.md`) is the unratified template — it
defines no principles to gate against, so there are no constitution violations. The design holds
to standard library-crate practices that a future constitution would likely encode:

- **Library-first, self-contained** — one crate, generic types only, no application domain types.
- **Test-first on the core contract** — the round-trip guarantee is covered by property tests;
  each spec acceptance scenario maps to an example test.
- **Simplicity** — small public surface, single-responsibility modules, no `unsafe`.

**Result**: PASS (no gates defined; no violations).

## Project Structure

### Documentation (this feature)

```text
specs/001-fits-header-io/
├── plan.md              # This file
├── research.md          # Phase 0 — design decisions
├── data-model.md        # Phase 1 — entities and their rules
├── quickstart.md        # Phase 1 — runnable validation scenarios
├── contracts/
│   └── api.md           # Phase 1 — public API contract
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify)
└── tasks.md             # Phase 2 — /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
src/
├── lib.rs      # Crate root: docs, module wiring, public re-exports, constants (CARD_LEN, BLOCK_LEN)
├── error.rs    # FitsError (thiserror)
├── card.rs     # Card, Value (Str | Literal), and the FromCard conversion trait + impls
├── header.rs   # Header: ordered cards, generic get::<T>, single + atomic-batch CRUD
├── parse.rs    # parse(&[u8]) -> Result<Header, FitsError>
├── write.rs    # StructuralHints, Header::to_bytes(&StructuralHints) -> Vec<u8>
├── coords.rs   # sexagesimal RA/Dec parse + format
└── dates.rs    # ISO-8601 date parse/format and MJD <-> calendar conversion

tests/
├── parse.rs        # Reading scenarios (US1)
├── crud.rs         # CRUD single + atomic batch (US2)
├── write.rs        # Serialization + structural cards (US3)
├── helpers.rs      # Coordinate / numeric / date / MJD helpers (US4)
└── roundtrip.rs    # proptest: parse(to_bytes(h)) fidelity, 80-byte cards, 2880 padding
```

**Structure Decision**: Single library crate (`src/lib.rs` + focused modules). Reading, writing,
value conversion, and helpers are separated into their own modules so each maps to a user story
and can be built and tested independently. Integration-level behavior lives under `tests/`; small
unit checks live in `#[cfg(test)]` modules next to the code they exercise.

## Complexity Tracking

No constitution violations; no complexity to justify.
