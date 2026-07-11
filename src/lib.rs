//! # fits-header
//!
//! A pure-Rust, MSVC-safe library for reading and writing the header of a
//! [FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.
//!
//! It exposes a generic, ordered `Header` of `(keyword, value, comment)` cards — free of
//! application domain types — with full CRUD over single or multiple keywords, parsing from
//! raw FITS bytes, and serialization back into a valid FITS object.
//!
//! ## Design
//!
//! - **Pure Rust, MSVC-safe** — minimal dependencies, no C libraries.
//! - **Round-trippable** — `parse(header.to_bytes(..))` reproduces the header.
//! - **Escape hatch** — arbitrary keywords can be written for vendor quirks.
#![forbid(unsafe_code)]
