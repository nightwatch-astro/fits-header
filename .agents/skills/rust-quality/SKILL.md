---
name: rust-quality
description: Use to run Rust format, lint, and test checks with the project toolchain.
---

# Rust Quality

## Preferred Flow

1. Run `scripts/check.sh`.
2. If issues are formatting-only, run `scripts/fix.sh`.
3. Re-run `scripts/check.sh` to confirm fixes.
4. When the agent needs API-design or library-specific guidance, LOAD references/idioms.md.

## Tooling Preference

Order the scripts implement:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/check.sh` | Run all checks (fmt, clippy, test) |
| `scripts/fix.sh` | Apply formatting only (cargo fmt) |

`fix.sh` is narrower than `check.sh`: it only formats. Clippy findings need
manual review and fixes; test failures need manual fixes. Re-run `check.sh`
after `fix.sh`.

