//! Error type for fallible header operations.

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

    /// [`Header::to_bytes`](crate::Header::to_bytes) declined to zero-fill a declared data
    /// segment larger than [`MAX_ZERO_FILL`](crate::MAX_ZERO_FILL). Serialize the header with
    /// [`Header::to_header_bytes`](crate::Header::to_header_bytes) and supply the data yourself.
    #[error("declared data size of {declared} bytes exceeds the to_bytes zero-fill cap ({max})")]
    DataTooLarge {
        /// The data size the header declares (saturated on overflow).
        declared: u64,
        /// The cap it exceeds ([`MAX_ZERO_FILL`](crate::MAX_ZERO_FILL)).
        max: u64,
    },
}
