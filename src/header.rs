//! The ordered header of records and its keyword access.

use crate::error::FitsError;
use crate::key::Key;
use crate::record::{is_commentary_keyword, validate_keyword, validate_keyword_raw, Record, Value};
use crate::value::{FromCard, IntoValue};
use crate::write::{self, StructuralHints};

/// An ordered FITS header unit: [`Record`]s in appearance order, with strict keyword
/// access (via [`Key`]) and CRUD.
///
/// Equality is semantic (records compare by content, not by retained bytes).
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    records: Vec<Record>,
}

impl Header {
    /// An empty header.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let h = Header::new();
    /// assert_eq!(h.count("OBJECT"), 0);
    /// ```
    pub fn new() -> Self {
        Header::default()
    }

    /// Construct from records (used by the parser).
    pub(crate) fn from_records(records: Vec<Record>) -> Self {
        Header { records }
    }

    /// The records in order (read-only escape hatch).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// assert_eq!(h.cards()[0].keyword(), Some("OBJECT"));
    /// ```
    pub fn cards(&self) -> &[Record] {
        &self.records
    }

    /// Iterate the records in order.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// h.set("EXPTIME", 120.0).unwrap();
    /// let names: Vec<&str> = h.iter().filter_map(|r| r.keyword()).collect();
    /// assert_eq!(names, vec!["OBJECT", "EXPTIME"]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }

    /// How many records carry this keyword.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.append("HISTORY", "dark subtracted").unwrap();
    /// h.append("HISTORY", "flat fielded").unwrap();
    /// assert_eq!(h.count("HISTORY"), 2);
    /// assert_eq!(h.count("OBJECT"), 0);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("EXPTIME", 120.0).unwrap();
    /// assert_eq!(h.get::<f64>("EXPTIME").unwrap(), Some(120.0));
    /// assert_eq!(h.get::<i64>("MISSING").unwrap(), None);
    /// ```
    pub fn get<T: FromCard>(&self, key: impl Into<Key>) -> Result<Option<T>, FitsError> {
        Ok(self
            .resolve(&key.into())?
            .and_then(|i| T::from_card(&self.records[i])))
    }

    /// Borrow a keyword's string value (`Str` content, non-empty); `None` for empty or a literal.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// h.set("EXPTIME", 120.0).unwrap(); // a Literal, not Str content
    /// assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31"));
    /// assert_eq!(h.get_str("EXPTIME").unwrap(), None);
    /// ```
    pub fn get_str(&self, key: impl Into<Key>) -> Result<Option<&str>, FitsError> {
        Ok(self
            .resolve(&key.into())?
            .and_then(|i| self.records[i].str_content()))
    }

    /// Every value for a keyword, in order. Unlike [`get`](Self::get), never errors on a
    /// duplicated keyword — that is the point of calling it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.append("HISTORY", "dark subtracted").unwrap();
    /// h.append("HISTORY", "flat fielded").unwrap();
    /// assert_eq!(
    ///     h.get_all::<String>("HISTORY"),
    ///     vec!["dark subtracted".to_string(), "flat fielded".to_string()]
    /// );
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{FitsError, Header};
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap(); // appends
    /// h.set("OBJECT", "NGC 7000").unwrap(); // updates in place
    /// assert_eq!(h.count("OBJECT"), 1);
    ///
    /// let err = h.set("object", 1); // lowercase is not FITS-standard
    /// assert!(matches!(err, Err(FitsError::InvalidKeyword { .. })));
    /// ```
    pub fn set(&mut self, key: impl Into<Key>, value: impl IntoValue) -> Result<(), FitsError> {
        self.set_inner(key.into(), value.into_value(), false)
    }

    /// Like [`set`](Self::set) but accepts any ≤8-char printable-ASCII keyword (vendor escape hatch).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set_raw("pi.name", "Jane Doe").unwrap(); // lowercase, not FITS-standard
    /// assert_eq!(h.get_str("pi.name").unwrap(), Some("Jane Doe"));
    /// ```
    pub fn set_raw(&mut self, keyword: &str, value: impl IntoValue) -> Result<(), FitsError> {
        self.set_inner(Key::Name(keyword.to_string()), value.into_value(), true)
    }

    /// Always add a record (a value card, or a commentary card for `COMMENT`/`HISTORY`/blank).
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.append("HISTORY", "dark subtracted").unwrap();
    /// h.append("HISTORY", "flat fielded").unwrap();
    /// assert_eq!(h.get_all::<String>("HISTORY").len(), 2);
    /// ```
    pub fn append(&mut self, name: &str, value: impl IntoValue) -> Result<(), FitsError> {
        validate_keyword(name)?;
        self.records
            .push(Self::make_record(name, value.into_value()));
        Ok(())
    }

    /// Set or replace the addressed value card's inline comment. No-op if the keyword is absent
    /// or not a value card.
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("AIRMASS", 1.2).unwrap();
    /// assert!(h.remove("AIRMASS").unwrap());
    /// assert!(!h.remove("AIRMASS").unwrap());
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set_many([("FILTER", "Ha"), ("TELESCOP", "EdgeHD 8")]).unwrap();
    ///
    /// // A rejected batch leaves the header untouched.
    /// assert!(h.set_many([("GAIN", "1"), ("TOOLONGKEY", "2")]).is_err());
    /// assert_eq!(h.count("GAIN"), 0);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// h.set("EXPTIME", 120.0).unwrap();
    /// assert_eq!(h.remove_many(["OBJECT", "MISSING"]).unwrap(), 1);
    /// assert_eq!(h.count("OBJECT"), 0);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Header;
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// let bytes = h.to_header_bytes();
    /// assert_eq!(bytes.len() % fits_header::BLOCK_LEN, 0);
    /// ```
    pub fn to_header_bytes(&self) -> Vec<u8> {
        write::to_header_bytes(self)
    }

    /// Serialize a standalone FITS object (header + a minimal zero data block). Mandatory
    /// structural cards are synthesized only when absent; `structural` is a fallback.
    ///
    /// Errors with [`FitsError::DataTooLarge`] when the declared data segment exceeds
    /// [`MAX_ZERO_FILL`](crate::MAX_ZERO_FILL) — for real-file edits, serialize with
    /// [`to_header_bytes`](Self::to_header_bytes) and splice the original data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{Header, StructuralHints};
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap();
    /// let file = h.to_bytes(&StructuralHints::default()).unwrap();
    /// assert!(file.starts_with(b"SIMPLE"));
    /// ```
    pub fn to_bytes(&self, structural: &StructuralHints) -> Result<Vec<u8>, FitsError> {
        write::to_bytes(self, structural)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_routes_commentary_keywords_to_commentary_records() {
        let mut h = Header::new();
        h.set("COMMENT", "a note").unwrap();
        h.set("HISTORY", "step 1").unwrap();
        assert!(matches!(
            h.cards()[0].kind,
            crate::record::RecordKind::Commentary { .. }
        ));
        assert_eq!(h.get_all::<String>("HISTORY"), vec!["step 1".to_string()]);
    }

    #[test]
    fn get_all_skips_unconvertible_values() {
        let mut h = Header::new();
        h.append("GAIN", 100).unwrap();
        h.append("GAIN", "not a number").unwrap();
        h.append("GAIN", 200).unwrap();
        assert_eq!(h.get_all::<i64>("GAIN"), vec![100, 200]);
        assert_eq!(h.count("GAIN"), 3);
    }

    #[test]
    fn set_comment_on_absent_key_is_noop() {
        let mut h = Header::new();
        h.set_comment("NOPE", "x").unwrap();
        assert!(h.cards().is_empty());
    }

    #[test]
    fn remove_returns_false_when_absent() {
        let mut h = Header::new();
        assert!(!h.remove("NOPE").unwrap());
    }

    #[test]
    fn remove_many_aborts_on_ambiguity_before_removing() {
        let mut h = Header::new();
        h.set("A", 1).unwrap();
        h.append("DUP", 1).unwrap();
        h.append("DUP", 2).unwrap();
        let before = h.clone();
        assert!(matches!(
            h.remove_many(["A", "DUP"]),
            Err(FitsError::AmbiguousKeyword { .. })
        ));
        assert_eq!(h, before, "nothing may be removed on a rejected batch");

        assert_eq!(h.remove_many(["A", "MISSING"]).unwrap(), 1);
    }

    #[test]
    fn set_many_accepts_occurrence_keys() {
        let mut h = Header::new();
        h.append("GAIN", 1).unwrap();
        h.append("GAIN", 2).unwrap();
        h.set_many([(("GAIN", 0), 10), (("GAIN", 1), 20)]).unwrap();
        assert_eq!(h.get_all::<i64>("GAIN"), vec![10, 20]);
    }

    #[test]
    fn iter_matches_cards() {
        let mut h = Header::new();
        h.set("A", 1).unwrap();
        h.set("B", 2).unwrap();
        assert_eq!(h.iter().count(), 2);
        let names: Vec<_> = h.iter().filter_map(|r| r.keyword()).collect();
        assert_eq!(names, vec!["A", "B"]);
    }

    #[test]
    fn get_on_missing_key_is_ok_none() {
        let h = Header::new();
        assert_eq!(h.get::<i64>("NOPE").unwrap(), None);
        assert_eq!(h.get_str("NOPE").unwrap(), None);
        assert_eq!(h.get::<i64>(("NOPE", 3)).unwrap(), None);
    }
}
