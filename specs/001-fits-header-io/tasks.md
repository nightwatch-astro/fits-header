# Tasks: Generic FITS Header Read/Write & CRUD

**Input**: Design documents from `specs/001-fits-header-io/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api.md

**Tests**: Included — the round-trip guarantee is a core, testable contract (SC-002/003/007).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency)
- Paths are repository-root relative (single crate).

---

## Phase 1: Foundational (blocking prerequisites)

- [ ] T001 `src/error.rs` — `FitsError` enum (`AmbiguousKeyword`, `KeywordTooLong`, `InvalidKeyword`) via `thiserror`.
- [ ] T002 `src/lib.rs` — module wiring, public re-exports, `CARD_LEN`/`BLOCK_LEN`, `#![forbid(unsafe_code)]`.
- [ ] T003 [P] `src/key.rs` — `Key` enum + `From<&str>`/`From<String>`/`From<(&str, usize)>`.
- [ ] T004 `src/record.rs` — `Value` (`Str`/`Literal`), `Record` (`Value`/`Commentary`/`Opaque`/`Continuation`) with retained raw bytes + modified flag; keyword validation helper.
- [ ] T005 `src/value.rs` — `FromCard` + `IntoValue` traits; impls for `String`/`f64`/`i64`/`u32`/`bool`/`PrimitiveDateTime`; `Literal`/`Fixed`/`Sci` wrappers; numeric formatting (shortest-round-trip + `.0`/`E` normalization).

---

## Phase 2: US1 — faithful read (P1)

- [ ] T010 [US1] `src/parse.rs` — `parse(&[u8])`: walk 80-byte cards, classify (value/commentary/opaque/continuation), retain raw, stop at `END`.
- [ ] T011 [US1] `src/parse.rs` — string/literal value extraction (`''`→`'`, trim, empty→`Str("")`, cut ` /` comment); CONTINUE run reassembly for `get`.
- [ ] T012 [P] [US1] `tests/read.rs` — scenarios: OBJECT/EXPTIME/comment, commentary retained, CONTINUE reassembled, stop-at-END.

**Checkpoint**: a real header parses; values/commentary retained; typed reads work.

---

## Phase 3: US2 — strict CRUD (P1)

- [ ] T020 [US2] `src/header.rs` — `Header`, `count`, `cards`/`iter`, `get`/`get_str`/`get_all` with strict `Key` resolution (`Err(AmbiguousKeyword)` on bare-name duplicates; occurrence targeting).
- [ ] T021 [US2] `src/header.rs` — `set` (update-in-place clears raw, else append), `append`, `set_comment`, `remove` (incl. CONTINUE run); commentary routing for `COMMENT`/`HISTORY`/blank.
- [ ] T022 [US2] `src/header.rs` — `set_many`/`remove_many` atomic (validate-all-then-apply); keyword validation.
- [ ] T023 [P] [US2] `tests/crud.rs` — unique update/append, occurrence select, ambiguity guard, atomic-batch rejection, vendor keyword, HISTORY append.

**Checkpoint**: full CRUD by name and occurrence, atomic and strict.

---

## Phase 4: US3 — byte-exact write-back (P1)

- [ ] T030 [US3] `src/write.rs` — card formatting (value: cols 1–8/`= `/quote+escape+pad / right-justify col 30 / ` / comment`; commentary: free text); emit raw for untouched records.
- [ ] T031 [US3] `src/write.rs` — `to_header_bytes` (cards + `END` + 2880 padding); `to_bytes(&StructuralHints)` (structural synth-if-absent, minimal data block); `StructuralHints` + `Default`.
- [ ] T032 [US3] `src/write.rs` — CONTINUE emission for over-length strings + `LONGSTRN` announcement.
- [ ] T033 [P] [US3] `tests/write.rs` — byte-exact untouched, structural synth/no-dup, 80-byte/2880, CONTINUE round-trip.

**Checkpoint**: edit-and-write-back preserves untouched cards byte-for-byte.

---

## Phase 5: US4 — helpers (P2)

- [ ] T040 [P] [US4] `src/coords.rs` — sexagesimal RA/Dec parse (separator/frac tolerant, sign@0°) + format.
- [ ] T041 [P] [US4] `src/dates.rs` — ISO-8601 parse/format; MJD↔calendar (`time`); `parse_f64`/`parse_i64`.
- [ ] T042 [P] [US4] `tests/helpers.rs` — RA/Dec boundaries, format round-trip, `20.0`→20, date + MJD.

---

## Phase 6: Cross-cutting & polish

- [ ] T050 `tests/roundtrip.rs` — proptest: byte-exact untouched + semantic `parse(to_header_bytes(h))==h`, 80-byte cards, 2880 padding.
- [ ] T051 [P] `serde` feature: `cfg`-gated derives on public types; `tests` behind `--features serde`.
- [ ] T052 [P] Doc-comments on all public items (missing_docs is warn); crate-level examples.
- [ ] T053 `just verify` green (default + `--features serde`); fix clippy/fmt.

## Dependencies

- Phase 1 blocks all others. Within a story, tasks are sequential unless `[P]`.
- US1 → US2 → US3 build on each other (read → edit → write); US4 is independent after Phase 1.
- T050 (roundtrip) requires US1+US3.
