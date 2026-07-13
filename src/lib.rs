//! # fits-header
//!
//! A pure-Rust, MSVC-safe library for reading and writing the header of a
//! [FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.
//!
//! It reads a header unit into an ordered [`Header`] of records, retaining every card so untouched
//! cards serialize byte-for-byte and only created or modified cards are re-rendered. Access is
//! strict and keyword-oriented: [`Header::get`] and friends take a [`Key`] that is either a bare
//! name (unique-or-[`FitsError::AmbiguousKeyword`]) or `(name, occurrence)`. Values read through a
//! generic [`FromCard`] and write through [`IntoValue`]; batch edits are atomic; long strings use
//! the `CONTINUE` convention.
//!
//! ```
//! use fits_header::Header;
//!
//! let mut h = Header::new();
//! h.set("OBJECT", "M31").unwrap();
//! h.set("EXPTIME", 120.0).unwrap();
//! assert_eq!(h.get::<f64>("EXPTIME").unwrap(), Some(120.0));
//!
//! // The header block only — creating a file means appending your own pixel data after
//! // this; editing an existing file means Header::update_file, which preserves it.
//! let bytes = h.to_header_bytes();
//! assert_eq!(bytes.len() % fits_header::BLOCK_LEN, 0);
//! ```
//!
//! ## Design
//!
//! - Pure Rust, no C or system libraries; builds with the MSVC toolchain.
//! - An untouched card (and untouched long-string run) re-emits identical bytes.
//! - Ambiguous keyword access errors instead of guessing.
//!
//! ## Features
//!
//! - `serde` *(off)* — derive `Serialize`/`Deserialize` on the public types.
#![forbid(unsafe_code)]

mod dates;
mod error;
mod header;
mod key;
mod parse;
mod record;
mod value;
mod write;

/// Bytes per header card.
pub const CARD_LEN: usize = 80;
/// Bytes per FITS block (36 cards).
pub const BLOCK_LEN: usize = 2880;

pub use crate::dates::{format_datetime, parse_datetime};
pub use crate::error::FitsError;
pub use crate::header::Header;
pub use crate::key::Key;
#[allow(deprecated)]
pub use crate::parse::parse;
pub use crate::record::{Record, RecordKind, Value};
pub use crate::value::{parse_f64, parse_i64, Fixed, FromCard, IntoValue, Literal, Sci};

/// Compiles the README's code blocks as doctests so the documented API cannot drift.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
