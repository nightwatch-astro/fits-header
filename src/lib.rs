// The crate root page is the README verbatim (also compiled as doctests below), so
// docs.rs's landing page is the full pitch, not just this comment.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod dates;
mod error;
mod header;
mod key;
mod parse;
mod record;
mod value;
mod write;

// Source of truth: docs/guide.md — also linked from the README and run via
// `cargo run --example quickstart`; its code blocks are compiled as doctests here.
#[doc = include_str!("../docs/guide.md")]
pub mod guide {}

/// Bytes per header card.
pub const CARD_LEN: usize = 80;
/// Bytes per FITS block (36 cards).
pub const BLOCK_LEN: usize = 2880;

pub use crate::dates::{format_datetime, parse_datetime};
pub use crate::error::{FitsError, Result};
pub use crate::header::Header;
pub use crate::key::Key;
#[allow(deprecated)]
pub use crate::parse::parse;
pub use crate::record::{Record, RecordKind, Value};
pub use crate::value::{parse_f64, parse_i64, Fixed, FromCard, IntoValue, Literal, Sci};
