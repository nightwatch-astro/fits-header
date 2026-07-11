# Quickstart & Validation

How to build the crate and prove the feature works. Signatures are in
[contracts/api.md](./contracts/api.md); types and rules are in [data-model.md](./data-model.md).

## Prerequisites

- Stable Rust toolchain (pinned by `rust-toolchain.toml`).
- Optional: [`just`](https://github.com/casey/just).

## Build & gate

```sh
just verify                    # cargo fmt --check + clippy -D warnings + cargo test
cargo test --features serde    # exercise the optional feature
cargo doc --no-deps
```

Expected: clean build with default features and with `serde`; all tests pass; docs build.

## Validation scenarios

Each maps to a user story in [spec.md](./spec.md); they are realized as tests under `tests/`.

### US1 — faithful read

- Parse a header with `OBJECT = 'M31     '`, `EXPTIME = 120.0 / seconds`, some `HISTORY` cards, and a
  `CONTINUE`-continued value.
- Expect: `get::<String>("OBJECT") == Ok(Some("M31"))`; `get::<f64>("EXPTIME") == Ok(Some(120.0))`;
  `HISTORY` cards retained (`count("HISTORY") > 0`); the continued value reassembles via `get::<String>`.

### US2 — strict CRUD

- `set("OBJECT", "NGC 7000")?` updates in place; `set("FILTER", "Ha")?` appends.
- With a duplicated `GAIN`: `get::<f64>("GAIN")` returns `Err(AmbiguousKeyword)`; `get::<f64>(("GAIN", 1))?`
  returns the second occurrence; `set(("GAIN", 1), 130)?` edits only it.
- `append("HISTORY", "calibrated 2026-07-11")?`; `get_all::<String>("HISTORY")` includes it.
- `set_many([("FILTER","Ha"),("GAIN","120")])?` applies both; a batch with a 9-char keyword returns
  `Err(KeywordTooLong)` and leaves the header unchanged.

### US3 — byte-exact write-back

- Parse, `set("EXPTIME", 300.0)?`, then `to_header_bytes()`.
- Expect: every card except the `EXPTIME` card is byte-identical to the input; re-parsing yields a
  semantically equal header; every card is 80 bytes; length is a multiple of 2880.
- `to_bytes(&StructuralHints::default())` on a from-scratch header synthesizes `SIMPLE/BITPIX/NAXIS*` and
  appends a minimal data block; on a parsed header those cards are not duplicated.
- A string longer than one card serializes across `CONTINUE` cards and reassembles on re-parse.

### US4 — helpers

- `sexagesimal_ra_to_deg("10 00 00") == Some(150.0)`; `sexagesimal_dec_to_deg("-00 30 00") == Some(-0.5)`.
- `deg_to_sexagesimal_ra(150.0)` re-parses to `150.0`.
- `parse_i64("20.0") == Some(20)`.
- `parse_datetime("2026-07-11T22:15:03")` succeeds; `format_datetime` yields the same string.
- `mjd_to_datetime(mjd)` agrees with the matching `DATE-OBS`.

### Round-trip property (core contract)

- `tests/roundtrip.rs` (proptest): for arbitrary generated headers, untouched cards serialize
  byte-for-byte, `parse(to_header_bytes(h))` is semantically equal to `h`, every card is 80 bytes, and
  output length is a multiple of 2880.

## Definition of done

- `just verify` is green with default features and with `--features serde`.
- Each user story's tests pass, including the ambiguity guards, atomic-batch rejection, `CONTINUE`
  round-trip, and byte-exact write-back.
