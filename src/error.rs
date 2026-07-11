//! Error type for fallible header operations.

/// Errors from validated header mutations and ambiguous lookups.
///
/// Parsing is lenient and does not produce these; serialization is infallible.
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
}
