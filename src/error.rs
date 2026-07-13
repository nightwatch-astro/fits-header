//! Error type for fallible header operations.

/// A `Result` whose error is always [`FitsError`].
pub type Result<T> = std::result::Result<T, FitsError>;

/// Errors from validated header mutations, ambiguous lookups, and oversized standalone
/// serialization.
///
/// Parsing is lenient and does not produce these; header-only serialization
/// ([`Header::to_header_bytes`](crate::Header::to_header_bytes)) is infallible.
///
/// # Examples
///
/// ```
/// # use fits_header::{FitsError, Header};
/// let mut h = Header::new();
/// h.append("GAIN", 1).unwrap();
/// h.append("GAIN", 2).unwrap();
/// assert!(matches!(
///     h.get::<i64>("GAIN"),
///     Err(FitsError::AmbiguousKeyword { .. })
/// ));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FitsError {
    /// A bare-name `get`/`set`/`remove` addressed a keyword that occurs more than once.
    /// Select one with an `(name, occurrence)` key.
    #[error("keyword '{keyword}' occurs {count} times; select an occurrence")]
    AmbiguousKeyword {
        /// The duplicated keyword.
        keyword: String,
        /// How many times it occurs.
        count: usize,
    },

    /// A keyword longer than the 8-character FITS field.
    #[error("keyword '{keyword}' exceeds 8 characters")]
    KeywordTooLong {
        /// The offending keyword.
        keyword: String,
    },

    /// A keyword containing bytes outside the FITS keyword set (`A-Z 0-9 - _`).
    #[error("keyword '{keyword}' contains characters outside A-Z 0-9 - _")]
    InvalidKeyword {
        /// The offending keyword.
        keyword: String,
    },

    /// An `(name, occurrence)` key targeted an occurrence that does not exist.
    #[error("keyword '{keyword}' has no occurrence {occurrence} (found {count})")]
    OccurrenceOutOfRange {
        /// The keyword addressed.
        keyword: String,
        /// The 0-based occurrence requested.
        occurrence: usize,
        /// How many occurrences exist.
        count: usize,
    },

    /// [`Header::update_file`](crate::Header::update_file) found no `END` card in the
    /// existing file's header region, so the data unit's boundary cannot be located.
    #[error("no END card found in header")]
    MissingEnd,

    /// [`Header::update_file`](crate::Header::update_file) found an `END` card, but the file
    /// ends before that header's 2880-byte block is complete — a truncated FITS file.
    #[error("header ends before its 2880-byte block is complete (truncated file)")]
    TruncatedHeader,

    /// A file read or write failed ([`Header::read_from_file`](crate::Header::read_from_file),
    /// [`Header::update_file`](crate::Header::update_file)).
    #[error("I/O error: {0}")]
    Io(String),
}

impl From<std::io::Error> for FitsError {
    fn from(e: std::io::Error) -> Self {
        FitsError::Io(e.to_string())
    }
}
