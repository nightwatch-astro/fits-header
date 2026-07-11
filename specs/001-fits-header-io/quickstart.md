# Quickstart & Validation

How to build the crate and prove the feature works end to end. Signatures are in
[contracts/api.md](./contracts/api.md); types and rules are in [data-model.md](./data-model.md).

## Prerequisites

- Stable Rust toolchain (pinned by `rust-toolchain.toml`).
- Optional: [`just`](https://github.com/casey/just).

## Build & gate

```sh
just verify            # cargo fmt --check + clippy -D warnings + cargo test
cargo test --features serde   # exercise the optional feature
cargo doc --no-deps
```

Expected: clean build with default features and with `serde`; all tests pass; docs build.

## Validation scenarios

Each maps to a user story and its acceptance scenarios in [spec.md](./spec.md). They are realized as
tests under `tests/`; the outcomes below are what those tests assert.

### US1 — read every card

- Parse the bytes of a header containing `OBJECT = 'M31     '` and `EXPTIME = 120.0 / seconds`.
- Expect: `get_str("OBJECT") == Some("M31")`; `get_f64("EXPTIME") == Some(120.0)`; the comment
  `seconds` is not part of the value; `COMMENT`/`HISTORY`/`HIERARCH` cards are absent; parsing stops
  at `END`.

### US2 — CRUD, single and atomic batch

- On a parsed header: `set("OBJECT", "NGC 7000")` updates in place; `set_f64("EXPTIME", 300.0)`
  updates; `remove("HISTORY")` deletes; a fresh `set("FILTER", "Ha")` appends.
- `set_many([("FILTER","Ha"),("GAIN","120")])` applies both; a batch containing a 9-character keyword
  returns `Err(FitsError::KeywordTooLong { .. })` and leaves the header unchanged (all-or-nothing).

### US3 — serialize to a valid FITS object

- `header.to_bytes(&StructuralHints::default())`.
- Expect: output begins with `SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2`, then the header cards,
  then `END`; every card is 80 bytes; total length is a multiple of 2880; re-parsing reproduces the
  original cards.

### US4 — coordinate / numeric / date helpers

- `sexagesimal_ra_to_deg("10 00 00") == Some(150.0)`; `sexagesimal_dec_to_deg("-00 30 00") == Some(-0.5)`.
- `deg_to_sexagesimal_ra(150.0)` re-parses to `150.0`.
- `parse_i64("20.0") == Some(20)`.
- `parse_datetime("2026-07-11T22:15:03")` succeeds; `format_datetime` of it yields the same string.
- `mjd_to_datetime(mjd)` agrees with the matching `DATE-OBS`.

### Round-trip property (core contract)

- `tests/roundtrip.rs` (proptest): for arbitrary generated headers, `parse(to_bytes(h))` equals `h`;
  every emitted card is 80 bytes; output length is a multiple of 2880.

## Definition of done

- `just verify` is green with default features and with `--features serde`.
- Each user story's tests pass, including the atomic-batch rejection case and the round-trip property.
