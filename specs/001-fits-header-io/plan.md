# Implementation Plan: Generic FITS Header Read/Write & CRUD

**Branch**: `feat/fits-header-io` | **Date**: 2026-07-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-fits-header-io/spec.md`

## Summary

`fits-header` is a single Rust library crate that faithfully reads, edits, and writes one FITS header
unit. Parsing keeps every card in order and retains its original bytes, so serialization reproduces
untouched cards byte-for-byte and re-renders only what the caller changed. Access is strict and
keyword-oriented through a unified key (bare name, or `(name, occurrence)`); a generic `get::<T>()`
reads values, type-directed `set`/`append` write them, and batch edits are atomic. Long strings use the
`CONTINUE` convention on both read and write. Numeric and date helpers sit on top. The
crate is pure-Rust and MSVC-safe.

## Technical Context

**Language/Version**: Rust (edition 2021), `rust-version = 1.74`.

**Primary Dependencies**: `time` 0.3 (`parsing`, `formatting`) for ISO-8601 dates and Julian-day math;
`thiserror` 2 for the error type; `serde` 1 behind an optional `serde` feature; `proptest` 1 (dev).

**Storage**: None. Input/output are `&[u8]` / `Vec<u8>`; the header holds no image data.

**Testing**: `cargo test` — unit tests, doc-tests, and `proptest` round-trip properties (byte-exact for
untouched cards + semantic equality). CI runs default features and `--features serde`.

**Target Platform**: Cross-platform, pure-Rust — Linux, Windows (MSVC), macOS.

**Project Type**: Single library crate.

**Performance Goals**: Linear single-pass parse and serialize; a header of a few thousand cards parses
and serializes in well under a millisecond.

**Constraints**: Pure-Rust, MSVC-safe, no C/system libraries. `#![forbid(unsafe_code)]`. Reads never
panic; fallible mutations return `Result`.

**Scale/Scope**: One primary header unit; small public surface (`Header`, `parse`, two serializers, a
value-conversion trait pair, and free-function helpers).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution is the unratified template — no principles to gate against, so no violations.
The design holds to standard library-crate practices: single crate, generic types only, `unsafe`-free,
property-tested core contract, small surface. **Result**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/001-fits-header-io/
├── plan.md              # This file
├── research.md          # Phase 0 — design decisions
├── data-model.md        # Phase 1 — entities and rules
├── quickstart.md        # Phase 1 — runnable validation
├── contracts/api.md     # Phase 1 — public API contract
├── checklists/requirements.md
└── tasks.md             # Phase 2 — /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── lib.rs      # Crate root: docs, module wiring, public re-exports, constants (CARD_LEN, BLOCK_LEN)
├── error.rs    # FitsError (thiserror): AmbiguousKeyword, KeywordTooLong, InvalidKeyword
├── record.rs   # Record (Value | Commentary | Opaque), Value (Str | Literal), raw bytes + modified flag
├── key.rs      # Key (Name | Occurrence) + From<&str>/From<(&str,usize)>
├── value.rs    # FromCard + IntoValue traits and impls; Literal/Fixed/Sci wrappers; number formatting
├── header.rs   # Header: ordered records, get/set/remove/append/get_all/count, atomic batch
├── parse.rs    # parse(&[u8]) -> Result<Header>: byte-exact retention, CONTINUE reassembly, commentary
├── write.rs    # to_header_bytes / to_bytes, StructuralHints, card formatting, CONTINUE+LONGSTRN
└── dates.rs    # ISO-8601 parse/format (via time)

tests/
├── read.rs         # US1 — faithful read, CONTINUE reassembly
├── crud.rs         # US2 — strict Key CRUD, occurrence selection, atomic batch
├── write.rs        # US3 — byte-exact write-back, structural synth, CONTINUE emission
├── helpers.rs      # US4 — numeric / date
└── roundtrip.rs    # proptest: byte-exact untouched + semantic equality, 80-byte cards, 2880 padding
```

**Structure Decision**: Single library crate. Read, write, value conversion, records, and helpers are
separate modules mapping to the user stories; integration behavior lives under `tests/`, small unit
checks in `#[cfg(test)]` modules next to the code.

## Complexity Tracking

No constitution violations; no complexity to justify.
