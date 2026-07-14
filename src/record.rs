// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Header records (cards) and their value payloads.

use crate::error::FitsError;
use crate::CARD_LEN;

/// A value card's payload.
///
/// [`IntoValue`](crate::IntoValue) produces one of these from a Rust value; a card's
/// [`RecordKind::Value`] holds one.
///
/// # Examples
///
/// ```
/// # use fits_header::{Header, RecordKind, Value};
/// let mut h = Header::new();
/// h.set("OBJECT", "M31").unwrap(); // a quoted string
/// h.set("EXPTIME", 120.0).unwrap(); // an unquoted literal
///
/// let values: Vec<&Value> = h
///     .cards()
///     .iter()
///     .filter_map(|r| match &r.kind {
///         RecordKind::Value { value, .. } => Some(value),
///         _ => None,
///     })
///     .collect();
/// assert_eq!(
///     values,
///     vec![&Value::Str("M31".to_string()), &Value::Literal("120.0".to_string())]
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
    /// A single-quoted string; content is unescaped. `Str("")` is present-but-empty.
    Str(String),
    /// An unquoted literal token (number, `T`/`F`, …), kept verbatim.
    Literal(String),
}

/// The semantic content of a [`Record`].
///
/// # Examples
///
/// ```
/// # use fits_header::{Header, RecordKind};
/// let mut h = Header::new();
/// h.set("OBJECT", "M31").unwrap();
/// h.append("HISTORY", "dark subtracted").unwrap();
///
/// assert!(matches!(h.cards()[0].kind, RecordKind::Value { .. }));
/// assert!(matches!(h.cards()[1].kind, RecordKind::Commentary { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RecordKind {
    /// An addressable value card: `KEYWORD = value / comment`.
    Value {
        /// The trimmed 8-character keyword.
        keyword: String,
        /// The value payload.
        value: Value,
        /// The inline comment, without the leading ` / `.
        comment: Option<String>,
    },
    /// A repeatable free-text card: `COMMENT`/`HISTORY`/blank keyword.
    Commentary {
        /// `COMMENT`, `HISTORY`, or the empty string (blank keyword).
        keyword: String,
        /// The free-text payload (columns 9–80).
        text: String,
    },
    /// A preserved card that is not addressable as a keyword (`HIERARCH`, unrecognized, blank).
    /// `text` holds the card content for display/serialization.
    Opaque {
        /// The card content (trailing spaces trimmed).
        text: String,
    },
}

/// One header card. Carries its parsed [`RecordKind`] plus, when parsed and unmodified, the
/// original bytes of its physical card(s) so it can be serialized verbatim.
///
/// # Examples
///
/// ```
/// # use fits_header::{Record, Value};
/// let r = Record::value("OBJECT", Value::Str("M31".to_string()), Some("target".to_string()));
/// assert_eq!(r.keyword(), Some("OBJECT"));
/// assert_eq!(r.str_content(), Some("M31"));
/// assert_eq!(r.comment(), Some("target"));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Record {
    /// The semantic content.
    pub kind: RecordKind,
    /// Original physical card bytes: `Some` when parsed and unmodified (a long-string run holds
    /// more than one card); `None` once created or edited (formatted on write). Not part of
    /// equality — two records are equal when their `kind`s are.
    #[cfg_attr(feature = "serde", serde(skip))]
    raw: Option<Vec<[u8; CARD_LEN]>>,
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Record {
    /// A new value card (not byte-backed; formatted on write).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{Record, Value};
    /// let r = Record::value("EXPTIME", Value::Literal("120.0".to_string()), None);
    /// assert_eq!(r.keyword(), Some("EXPTIME"));
    /// assert_eq!(r.value_text(), Some("120.0"));
    /// ```
    pub fn value(keyword: impl Into<String>, value: Value, comment: Option<String>) -> Self {
        Record {
            kind: RecordKind::Value {
                keyword: keyword.into(),
                value,
                comment,
            },
            raw: None,
        }
    }

    /// A new commentary card (not byte-backed; formatted on write).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Record;
    /// let r = Record::commentary("HISTORY", "dark subtracted");
    /// assert_eq!(r.keyword(), Some("HISTORY"));
    /// assert_eq!(r.value_text(), Some("dark subtracted"));
    /// ```
    pub fn commentary(keyword: impl Into<String>, text: impl Into<String>) -> Self {
        Record {
            kind: RecordKind::Commentary {
                keyword: keyword.into(),
                text: text.into(),
            },
            raw: None,
        }
    }

    /// A record backed by original card bytes (parsed, unmodified).
    pub(crate) fn from_raw(kind: RecordKind, raw: Vec<[u8; CARD_LEN]>) -> Self {
        Record {
            kind,
            raw: Some(raw),
        }
    }

    /// The addressable keyword, or `None` for opaque cards.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// assert_eq!(h.cards()[0].keyword(), Some("OBJECT"));
    /// ```
    pub fn keyword(&self) -> Option<&str> {
        match &self.kind {
            RecordKind::Value { keyword, .. } | RecordKind::Commentary { keyword, .. } => {
                Some(keyword)
            }
            RecordKind::Opaque { .. } => None,
        }
    }

    /// The value as text for typed reads: `Str` content (non-empty), a `Literal` token, or
    /// commentary text. `None` for empty strings and opaque cards.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("EXPTIME", 120.0).unwrap();
    /// assert_eq!(h.cards()[0].value_text(), Some("120.0"));
    /// ```
    pub fn value_text(&self) -> Option<&str> {
        match &self.kind {
            RecordKind::Value { value, .. } => match value {
                Value::Str(s) => (!s.is_empty()).then_some(s.as_str()),
                Value::Literal(l) => Some(l),
            },
            RecordKind::Commentary { text, .. } => Some(text),
            RecordKind::Opaque { .. } => None,
        }
    }

    /// The `Str` content of a value card (non-empty), for [`Header::get_str`](crate::Header::get_str).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// h.set("EXPTIME", 120.0).unwrap(); // a Literal, not Str content
    /// assert_eq!(h.cards()[0].str_content(), Some("M31"));
    /// assert_eq!(h.cards()[1].str_content(), None);
    /// ```
    pub fn str_content(&self) -> Option<&str> {
        match &self.kind {
            RecordKind::Value {
                value: Value::Str(s),
                ..
            } => (!s.is_empty()).then_some(s.as_str()),
            _ => None,
        }
    }

    /// The inline comment of a value card.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("EXPTIME", 120.0).unwrap();
    /// h.set_comment("EXPTIME", "seconds").unwrap();
    /// assert_eq!(h.cards()[0].comment(), Some("seconds"));
    /// ```
    pub fn comment(&self) -> Option<&str> {
        match &self.kind {
            RecordKind::Value { comment, .. } => comment.as_deref(),
            _ => None,
        }
    }

    /// Original card bytes when byte-backed (unmodified).
    pub(crate) fn raw_cards(&self) -> Option<&[[u8; CARD_LEN]]> {
        self.raw.as_deref()
    }

    /// Replace a value card's value (or a commentary card's text) and mark the record dirty
    /// so it is reformatted on write.
    pub(crate) fn replace_value(&mut self, new: Value) {
        match &mut self.kind {
            RecordKind::Value { value, .. } => *value = new,
            RecordKind::Commentary { text, .. } => {
                *text = match new {
                    Value::Str(s) | Value::Literal(s) => s,
                };
            }
            RecordKind::Opaque { .. } => {}
        }
        self.raw = None;
    }

    /// Set or clear a value card's comment, marking it dirty.
    pub(crate) fn set_comment(&mut self, c: Option<String>) {
        if let RecordKind::Value { comment, .. } = &mut self.kind {
            *comment = c;
            self.raw = None;
        }
    }
}

/// True for keywords whose payload is free text (`COMMENT`, `HISTORY`, or blank).
pub fn is_commentary_keyword(name: &str) -> bool {
    name.is_empty() || name == "COMMENT" || name == "HISTORY"
}

/// Validate a standard FITS keyword: ≤8 characters, bytes in `A-Z 0-9 - _`.
pub fn validate_keyword(name: &str) -> Result<(), FitsError> {
    if name.len() > 8 {
        return Err(FitsError::KeywordTooLong {
            keyword: name.to_string(),
        });
    }
    for &b in name.as_bytes() {
        let ok = b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-' || b == b'_';
        if !ok {
            return Err(FitsError::InvalidKeyword {
                keyword: name.to_string(),
            });
        }
    }
    Ok(())
}

/// Validate a vendor keyword escape hatch: ≤8 characters, printable ASCII (any charset).
pub fn validate_keyword_raw(name: &str) -> Result<(), FitsError> {
    if name.len() > 8 {
        return Err(FitsError::KeywordTooLong {
            keyword: name.to_string(),
        });
    }
    for &b in name.as_bytes() {
        if !(0x20..=0x7e).contains(&b) {
            return Err(FitsError::InvalidKeyword {
                keyword: name.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_validation_charset_and_length() {
        for ok in ["", "A", "DATE-OBS", "NAXIS_1", "K2"] {
            assert!(validate_keyword(ok).is_ok(), "{ok:?} should validate");
        }
        assert!(matches!(
            validate_keyword("NINECHARS"),
            Err(FitsError::KeywordTooLong { .. })
        ));
        for bad in ["obj", "KEY WORD", "É", "K.1"] {
            assert!(
                matches!(validate_keyword(bad), Err(FitsError::InvalidKeyword { .. })),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn raw_validation_allows_printable_ascii_only() {
        for ok in ["obj", "K.1", "a b", "~"] {
            assert!(validate_keyword_raw(ok).is_ok(), "{ok:?} should validate");
        }
        assert!(matches!(
            validate_keyword_raw("NINECHARS"),
            Err(FitsError::KeywordTooLong { .. })
        ));
        for bad in ["tab\there", "É"] {
            assert!(
                matches!(
                    validate_keyword_raw(bad),
                    Err(FitsError::InvalidKeyword { .. })
                ),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn commentary_keywords() {
        assert!(is_commentary_keyword(""));
        assert!(is_commentary_keyword("COMMENT"));
        assert!(is_commentary_keyword("HISTORY"));
        assert!(!is_commentary_keyword("OBJECT"));
    }

    #[test]
    fn accessors_by_kind() {
        let v = Record::value("K", Value::Str("s".into()), Some("c".into()));
        assert_eq!(v.keyword(), Some("K"));
        assert_eq!(v.value_text(), Some("s"));
        assert_eq!(v.str_content(), Some("s"));
        assert_eq!(v.comment(), Some("c"));

        let lit = Record::value("K", Value::Literal("42".into()), None);
        assert_eq!(lit.value_text(), Some("42"));
        assert_eq!(lit.str_content(), None, "literal is not Str content");

        let c = Record::commentary("HISTORY", "note");
        assert_eq!(c.keyword(), Some("HISTORY"));
        assert_eq!(c.value_text(), Some("note"));
        assert_eq!(c.str_content(), None);
        assert_eq!(c.comment(), None);
    }

    #[test]
    fn equality_ignores_retained_bytes() {
        let kind = RecordKind::Value {
            keyword: "K".into(),
            value: Value::Str("s".into()),
            comment: None,
        };
        let parsed = Record::from_raw(kind.clone(), vec![[b' '; CARD_LEN]]);
        let created = Record::value("K", Value::Str("s".into()), None);
        assert_eq!(parsed, created);
    }

    #[test]
    fn mutation_drops_retained_bytes() {
        let kind = RecordKind::Value {
            keyword: "K".into(),
            value: Value::Str("s".into()),
            comment: None,
        };
        let mut r = Record::from_raw(kind, vec![[b' '; CARD_LEN]]);
        assert!(r.raw_cards().is_some());
        r.replace_value(Value::Str("t".into()));
        assert!(r.raw_cards().is_none(), "edited record must reformat");

        let mut r2 = Record::from_raw(
            RecordKind::Value {
                keyword: "K".into(),
                value: Value::Str("s".into()),
                comment: None,
            },
            vec![[b' '; CARD_LEN]],
        );
        r2.set_comment(Some("c".into()));
        assert!(r2.raw_cards().is_none());
        assert_eq!(r2.comment(), Some("c"));
    }

    #[test]
    fn set_comment_ignores_non_value_records() {
        let mut c = Record::from_raw(
            RecordKind::Commentary {
                keyword: "COMMENT".into(),
                text: "x".into(),
            },
            vec![[b' '; CARD_LEN]],
        );
        c.set_comment(Some("ignored".into()));
        assert_eq!(c.comment(), None);
        assert!(c.raw_cards().is_some(), "no-op keeps original bytes");
    }
}
