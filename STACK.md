# Stack Decisions

Recorded during project setup. This crate is intentionally minimal.

## Language & toolchain

- **Rust**, edition **2021**, `rust-version = 1.74`.
- Toolchain pinned to **stable** with `rustfmt` + `clippy` (`rust-toolchain.toml`).
- Task runner: **just** (`justfile`).

## Dependencies

Policy (revised 2026-07-11): the crate is **not** zero-dependency, but stays **pure Rust,
MSVC-safe, and publishable** — no C or system libraries. Versions are aligned with
`nightwatch-astro/alm`, the repo this crate merges back into.

- **`time` 0.3** *(runtime)* — parse/format FITS date keywords (`DATE-OBS`, `DATE-LOC`,
  `DATE-END`) and convert between Modified Julian Date (`MJD-OBS`/`MJD-AVG`) and calendar dates.
- **`thiserror` 2** *(runtime)* — typed error enum for parse/serialize failures.
- **`serde` 1** *(optional, off-by-default `serde` feature)* — `Serialize`/`Deserialize` on
  `Header`/`Card`/`StructuralHints` for JSON and other formats. Not a default so consumers who
  don't need it pay nothing.
- **`proptest` 1** *(dev-only)* — property-based testing of the parse↔`to_bytes` round-trip.

Deliberately **not** added: `chrono` (superseded by `time`, which alm already uses),
`hifitime` (full astronomical time scales — overkill for header I/O), `byteorder`/`lexical`
(unneeded for text cards), and any crate pulling C/system libraries.

## Licensing

- **Apache-2.0** (`LICENSE`, `Cargo.toml` `license = "Apache-2.0"`).
- Note: the extraction brief suggested dual `MIT OR Apache-2.0`; Apache-2.0-only was
  chosen at setup. Revisit before the first crates.io publish if dual-licensing is
  desired (add `LICENSE-MIT` and set `license = "MIT OR Apache-2.0"`).

## Quality gates

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc` — locally
  via `just verify` and in GitHub Actions CI (Linux, Windows, macOS).
- `pre-commit`: hygiene hooks + `cargo fmt`/`cargo clippy` + `gitleaks` secret scan on push.

## Agentic tooling

- Managed with **APM** (`apm.yml` / `apm.lock.yaml`). Installed packages: `language-rust`,
  `lsp-rust` (rust-analyzer), `speckit` + `steering-speckit`, `hooks-attribution-guard`,
  `steering-git-workflow`, `release-please`.
- **SpecKit** (lightweight) scaffolded under `.specify/` with `/speckit-*` commands.
