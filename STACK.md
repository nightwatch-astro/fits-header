# Stack Decisions

Recorded during project setup. This crate is intentionally minimal.

## Language & toolchain

- **Rust**, edition **2021**, `rust-version = 1.74`.
- Toolchain pinned to **stable** with `rustfmt` + `clippy` (`rust-toolchain.toml`).
- Task runner: **just** (`justfile`).

## Dependencies

- **None.** `fits-header` is `std`-only by design and must stay that way:
  - Publishable and trivially auditable (no supply-chain surface).
  - **MSVC-safe** — no C/system libraries, builds on Windows without extra toolchains.
  - Verified on Linux, Windows, and macOS in CI.
- Consequence: error handling, parsing, and formatting are hand-rolled rather than
  pulling `thiserror`/`anyhow`/`nom`. This is a deliberate constraint, not an omission.

## Licensing

- **Apache-2.0** (`LICENSE`, `Cargo.toml` `license = "Apache-2.0"`).
- Note: the extraction brief suggested dual `MIT OR Apache-2.0`; Apache-2.0-only was
  chosen at setup. Revisit before the first crates.io publish if dual-licensing is
  desired (add `LICENSE-MIT` and set `license = "MIT OR Apache-2.0"`).

## Quality gates

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc` — locally
  via `just verify` and in GitHub Actions CI.
- `pre-commit`: hygiene hooks + `cargo fmt`/`cargo clippy` + `gitleaks` secret scan on push.

## Agentic tooling

- Managed with **APM** (`apm.yml` / `apm.lock.yaml`). Installed packages: `language-rust`,
  `lsp-rust` (rust-analyzer), `speckit` + `steering-speckit`, `hooks-attribution-guard`,
  `steering-git-workflow`, `release-please`.
- **SpecKit** (lightweight) scaffolded under `.specify/` with `/speckit-*` commands.
