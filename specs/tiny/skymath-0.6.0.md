# TinySpec: Consume Skymath 0.6.0

**Branch**: deps/skymath-0.6.0
**Date**: 2026-07-22
**Status**: done
**Complexity**: small

## What

fits-header consumes the released skymath 0.6.0 API while preserving FITS
`DATE-OBS` parsing behavior.

## Context

| File | Role |
|------|------|
| `Cargo.toml` | Declares the direct skymath requirement |
| `Cargo.lock` | Pins the resolved skymath package |
| `src/dates.rs` | Delegates `DATE-OBS` parsing to skymath |
| `tests/values.rs` | Exercises typed `DATE-OBS` header reads |

## Requirements

1. The active direct dependency accepts skymath 0.6.0 and no older minor line.
2. The lockfile resolves skymath 0.6.0 without unrelated package churn.
3. Existing valid and invalid FITS date forms retain their behavior.
4. Default and all-feature builds, tests, lint, docs, and package checks pass.

## Plan

1. Update the direct dependency with Cargo and regenerate the lockfile.
2. Compile and test the existing skymath-backed date path.
3. Make only compatibility changes required by skymath 0.6.0.
4. Run repository and release gates, then scan for stale active constraints.

## Tasks

- [x] Update `Cargo.toml` and `Cargo.lock` with Cargo.
- [x] Verify `src/dates.rs` against skymath 0.6.0.
- [x] Run format, lint, default/all-feature test, and documentation gates.
- [x] Run package and release-please gates and scan active dependency constraints.

## Done When

- [x] All tasks are checked off.
- [x] The lockfile delta is limited to skymath.
- [x] Tests and lint pass.
- [x] Documentation and package checks pass.
