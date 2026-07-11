//! # fits-header
//!
//! A dependency-free, `std`-only library for reading and writing the header of a
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
//! - **std-only, no dependencies** — publishable and MSVC-safe.
//! - **Round-trippable** — `parse(header.to_bytes()) == header` for representative headers.
//! - **Escape hatch** — arbitrary keywords can be written for vendor quirks.
#![forbid(unsafe_code)]

// Implementation to follow:
//   - `struct Header` (ordered Vec of cards) with typed getters/setters.
//   - `parse(&[u8]) -> Result<Header>` over 2880-byte blocks / 80-byte cards.
//   - `Header::to_bytes(&StructuralHints) -> Vec<u8>`.
//   - sexagesimal + numeric parsing helpers.
