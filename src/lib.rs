//! # fits-header
//!
//! A pure-Rust, MSVC-safe library for reading and writing the header of a
//! [FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.
//!
//! The crate is deliberately free of application domain types: it exposes a
//! generic, ordered [`Header`] of `(keyword, value, comment)` cards plus the
//! machinery to parse a header out of raw FITS bytes and serialize it back into
//! a valid FITS object. It supports full CRUD over single or multiple keywords.
//!
//! ## Status
//!
//! This is the scaffold produced during project setup. The parser, the
//! [`Header`] CRUD surface, and `to_bytes` serialization are implemented as
//! follow-up work (see `AGENTS.md` and the SpecKit spec).
//!
//! ## Design goals
//!
//! - **Pure Rust, MSVC-safe** — minimal deps (`time`, `thiserror`), no C libraries, publishable.
//! - **Round-trippable** — `parse(header.to_bytes()) == header` for representative headers.
//! - **Escape hatch** — arbitrary keywords can be written for vendor quirks.
#![forbid(unsafe_code)]

// Implementation to follow:
//   - `struct Header` (ordered Vec of cards) with a generic `get::<T>()` accessor
//     (String/f64/i64/u32/bool/datetime via a `FromCard` trait) + named wrappers,
//     plus setters and single/multi CRUD.
//   - `parse(&[u8]) -> Result<Header>` over 2880-byte blocks / 80-byte cards.
//   - `Header::to_bytes(&StructuralHints) -> Vec<u8>`.
//   - sexagesimal parse + format, numeric parsing, and MJD <-> date helpers (via `time`).
//   - optional `serde` feature: Serialize/Deserialize on Header/Card/StructuralHints.
