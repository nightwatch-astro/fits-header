//! Header records (cards) and their value payloads.

use crate::error::FitsError;
use crate::CARD_LEN;

/// A value card's payload.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
    /// A single-quoted string; content is unescaped. `Str("")` is present-but-empty.
    Str(String),
    /// An unquoted literal token (number, `T`/`F`, …), kept verbatim.
    Literal(String),
}

/// The semantic content of a record.
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

/// One header card. Carries its parsed content plus, when parsed and unmodified, the original
/// bytes of its physical card(s) so it can be serialized verbatim.
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

    /// The `Str` content of a value card (non-empty), for `get_str`.
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
