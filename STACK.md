# Stack

## Language & toolchain

- **Rust**, edition **2021**, `rust-version = 1.74`.
- Stable toolchain with `rustfmt` + `clippy` (`rust-toolchain.toml`).
- Task runner: **just** (`justfile`).

## Dependencies

Pure-Rust, MSVC-safe only — no C or system libraries.

- **`time`** — FITS date keywords (`DATE-OBS`, `DATE-LOC`, `DATE-END`) and MJD ↔ calendar conversion.
- **`thiserror`** — error type for parse/serialize failures.
- **`serde`** *(optional, `serde` feature, off by default)* — `Serialize`/`Deserialize` on the public types.
- **`proptest`** *(dev)* — property tests for the parse ↔ `to_bytes` round-trip.
- **`serde_json`** *(dev)* — JSON round-trip tests for the `serde` feature.

## Licensing

Apache-2.0 (`LICENSE`, `Cargo.toml` `license = "Apache-2.0"`).

## Quality gates

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc` — via `just verify`
  and in GitHub Actions CI (Linux, Windows, macOS).
- `pre-commit`: hygiene hooks + `cargo fmt`/`cargo clippy` + `gitleaks` secret scan on push.

## Agentic tooling

- **APM** (`apm.yml` / `apm.lock.yaml`): `language-rust`, `lsp-rust` (rust-analyzer), `speckit` +
  `steering-speckit`, `hooks-attribution-guard`, `steering-git-workflow`, `release-please`.
- **SpecKit** scaffolded under `.specify/` with `/speckit-*` commands.
