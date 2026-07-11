//! The ordered header of records and its keyword access.

use crate::error::FitsError;
use crate::key::Key;
use crate::record::{is_commentary_keyword, validate_keyword, validate_keyword_raw, Record, Value};
use crate::value::{FromCard, IntoValue};
use crate::write::{self, StructuralHints};

/// An ordered FITS header unit: records in appearance order, with strict keyword access and CRUD.
///
/// Equality is semantic (records compare by content, not by retained bytes).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    records: Vec<Record>,
}

impl Header {
    /// An empty header.
    pub fn new() -> Self {
        Header::default()
    }

    /// Construct from records (used by the parser).
    pub(crate) fn from_records(records: Vec<Record>) -> Self {
        Header { records }
    }

    /// The records in order (read-only escape hatch).
    pub fn cards(&self) -> &[Record] {
        &self.records
    }

    /// Iterate the records in order.
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }

    /// How many records carry this keyword.
    pub fn count(&self, name: &str) -> usize {
        self.records
            .iter()
            .filter(|r| r.keyword() == Some(name))
            .count()
    }

    /// Resolve a key to a record index. A bare name is strict.
    fn resolve(&self, key: &Key) -> Result<Option<usize>, FitsError> {
        let name = key.name();
        let indices: Vec<usize> = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.keyword() == Some(name))
            .map(|(i, _)| i)
            .collect();
        match key.occurrence() {
            Some(n) => Ok(indices.get(n).copied()),
            None => match indices.len() {
                0 => Ok(None),
                1 => Ok(Some(indices[0])),
                count => Err(FitsError::AmbiguousKeyword {
                    keyword: name.to_string(),
                    count,
                }),
            },
        }
    }

    /// Read a keyword as `T`. `Err` only on an ambiguous bare name; `Ok(None)` when absent or the
    /// value does not convert; never panics.
    pub fn get<T: FromCard>(&self, key: impl Into<Key>) -> Result<Option<T>, FitsError> {
        Ok(self
            .resolve(&key.into())?
            .and_then(|i| T::from_card(&self.records[i])))
    }

    /// Borrow a keyword's string value (`Str` content, non-empty); `None` for empty or a literal.
    pub fn get_str(&self, key: impl Into<Key>) -> Result<Option<&str>, FitsError> {
        Ok(self
            .resolve(&key.into())?
            .and_then(|i| self.records[i].str_content()))
    }

    /// Every value for a keyword, in order.
    pub fn get_all<T: FromCard>(&self, name: &str) -> Vec<T> {
        self.records
            .iter()
            .filter(|r| r.keyword() == Some(name))
            .filter_map(T::from_card)
            .collect()
    }

    fn make_record(name: &str, value: Value) -> Record {
        if is_commentary_keyword(name) {
            let text = match value {
                Value::Str(s) | Value::Literal(s) => s,
            };
            Record::commentary(name, text)
        } else {
            Record::value(name, value, None)
        }
    }

    fn set_inner(&mut self, key: Key, value: Value, raw: bool) -> Result<(), FitsError> {
        let name = key.name().to_string();
        if raw {
            validate_keyword_raw(&name)?;
        } else {
            validate_keyword(&name)?;
        }
        match self.resolve(&key)? {
            Some(i) => {
                self.records[i].replace_value(value);
                Ok(())
            }
            None => match key.occurrence() {
                Some(n) => Err(FitsError::OccurrenceOutOfRange {
                    keyword: name.clone(),
                    occurrence: n,
                    count: self.count(&name),
                }),
                None => {
                    self.records.push(Self::make_record(&name, value));
                    Ok(())
                }
            },
        }
    }

    /// Update the addressed record in place, or append when the (unique) name is absent.
    /// The keyword must be FITS-standard (`≤8`, `A-Z 0-9 - _`); use [`set_raw`](Self::set_raw)
    /// for vendor keys.
    pub fn set(&mut self, key: impl Into<Key>, value: impl IntoValue) -> Result<(), FitsError> {
        self.set_inner(key.into(), value.into_value(), false)
    }

    /// Like [`set`](Self::set) but accepts any ≤8-char printable-ASCII keyword (vendor escape hatch).
    pub fn set_raw(&mut self, keyword: &str, value: impl IntoValue) -> Result<(), FitsError> {
        self.set_inner(Key::Name(keyword.to_string()), value.into_value(), true)
    }

    /// Always add a record (a value card, or a commentary card for `COMMENT`/`HISTORY`/blank).
    pub fn append(&mut self, name: &str, value: impl IntoValue) -> Result<(), FitsError> {
        validate_keyword(name)?;
        self.records
            .push(Self::make_record(name, value.into_value()));
        Ok(())
    }

    /// Set or replace the addressed value card's inline comment. No-op if the keyword is absent
    /// or not a value card.
    pub fn set_comment(
        &mut self,
        key: impl Into<Key>,
        comment: impl Into<String>,
    ) -> Result<(), FitsError> {
        if let Some(i) = self.resolve(&key.into())? {
            self.records[i].set_comment(Some(comment.into()));
        }
        Ok(())
    }

    /// Remove the addressed record. Returns whether anything was removed.
    pub fn remove(&mut self, key: impl Into<Key>) -> Result<bool, FitsError> {
        match self.resolve(&key.into())? {
            Some(i) => {
                self.records.remove(i);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Apply several mutations atomically: validate every entry first, then apply all or none.
    pub fn set_many<K, V>(
        &mut self,
        entries: impl IntoIterator<Item = (K, V)>,
    ) -> Result<(), FitsError>
    where
        K: Into<Key>,
        V: IntoValue,
    {
        let items: Vec<(Key, Value)> = entries
            .into_iter()
            .map(|(k, v)| (k.into(), v.into_value()))
            .collect();
        // Validate keywords and resolvability against the current state before mutating.
        for (k, _) in &items {
            validate_keyword(k.name())?;
            if let Some(n) = k.occurrence() {
                if self.resolve(k)?.is_none() {
                    return Err(FitsError::OccurrenceOutOfRange {
                        keyword: k.name().to_string(),
                        occurrence: n,
                        count: self.count(k.name()),
                    });
                }
            } else {
                // Surfaces AmbiguousKeyword before any change.
                self.resolve(k)?;
            }
        }
        for (k, v) in items {
            self.set_inner(k, v, false)?;
        }
        Ok(())
    }

    /// Remove several keys atomically (validation only guards ambiguity). Returns the count removed.
    pub fn remove_many<K: Into<Key>>(
        &mut self,
        keys: impl IntoIterator<Item = K>,
    ) -> Result<usize, FitsError> {
        let keys: Vec<Key> = keys.into_iter().map(Into::into).collect();
        for k in &keys {
            self.resolve(k)?;
        }
        let mut removed = 0;
        for k in keys {
            if self.remove(k)? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Serialize the header block only (cards, `END`, padded to a 2880 multiple) for splicing onto
    /// an existing file's data.
    pub fn to_header_bytes(&self) -> Vec<u8> {
        write::to_header_bytes(self)
    }

    /// Serialize a standalone FITS object (header + a minimal zero data block). Mandatory
    /// structural cards are synthesized only when absent; `structural` is a fallback.
    pub fn to_bytes(&self, structural: &StructuralHints) -> Vec<u8> {
        write::to_bytes(self, structural)
    }
}
