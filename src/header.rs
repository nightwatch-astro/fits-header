//! The ordered header of records and its keyword access.

use crate::error::{FitsError, Result};
use crate::key::Key;
use crate::record::{is_commentary_keyword, validate_keyword, validate_keyword_raw, Record, Value};
use crate::value::{FromCard, IntoValue};
use crate::write;
use crate::{BLOCK_LEN, CARD_LEN};
use std::fs;
use std::io::Write as _;
use std::path::Path;

/// An ordered FITS header unit: [`Record`]s in appearance order, with strict keyword
/// access (via [`Key`]) and CRUD.
///
/// A `Header` is an in-memory value. `set`, `append`, `remove`, `set_many`, and
/// `set_comment` change it in memory only — nothing is written to disk. Persist it with
/// [`update_file`](Self::update_file) to edit an existing file's header in place (the
/// common case), or [`write_to_file`](Self::write_to_file) to create a new file from a
/// header you built plus pixel data you already have.
/// [`to_header_bytes`](Self::to_header_bytes) is the lower-level building block behind
/// both, for callers who assemble the file bytes themselves.
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

    /// Parse one FITS header unit from raw bytes.
    ///
    /// Reads 80-byte cards in order, stops at `END`, and retains every card (including
    /// commentary, `HIERARCH`, and unrecognized cards) so untouched cards serialize verbatim.
    /// `CONTINUE` runs are reassembled into a single logical value. Bytes after `END` (the data
    /// unit, later HDUs) are ignored — this crate is header-only and never inspects them.
    ///
    /// # Examples
    ///
    /// ```
    /// use fits_header::Header;
    ///
    /// let mut bytes = Vec::new();
    /// for card in ["OBJECT  = 'M31     '", "EXPTIME =                120.0", "END"] {
    ///     let mut c = card.as_bytes().to_vec();
    ///     c.resize(80, b' ');
    ///     bytes.extend(c);
    /// }
    ///
    /// let header = Header::parse(&bytes).unwrap();
    /// assert_eq!(header.get_str("OBJECT").unwrap(), Some("M31"));
    /// assert_eq!(header.get::<f64>("EXPTIME").unwrap(), Some(120.0));
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<Header> {
        crate::parse::parse_header(bytes)
    }

    /// Read a FITS header from a file on disk.
    ///
    /// Reads the whole file and [`parse`](Self::parse)s it; parsing already stops at `END`, so
    /// the data unit and any later HDUs are read but never interpreted.
    ///
    /// # Examples
    ///
    /// ```
    /// use fits_header::Header;
    ///
    /// let mut bytes = Vec::new();
    /// for card in ["OBJECT  = 'M31     '", "END"] {
    ///     let mut c = card.as_bytes().to_vec();
    ///     c.resize(80, b' ');
    ///     bytes.extend(c);
    /// }
    /// while bytes.len() % fits_header::BLOCK_LEN != 0 {
    ///     bytes.push(b' ');
    /// }
    /// bytes.extend_from_slice(&[0u8; 4]); // stand-in pixel data
    ///
    /// let path = std::env::temp_dir().join("fits-header-doctest-read_from_file.fits");
    /// std::fs::write(&path, &bytes).unwrap();
    ///
    /// let header = Header::read_from_file(&path).unwrap();
    /// assert_eq!(header.get_str("OBJECT").unwrap(), Some("M31"));
    ///
    /// std::fs::remove_file(&path).ok();
    /// ```
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Header> {
        let bytes = fs::read(path)?;
        Header::parse(&bytes)
    }

    /// Edit a FITS file's header in place, preserving its data unit (and any later HDUs)
    /// byte-for-byte.
    ///
    /// Reads the whole file, locates the header region by scanning for the `END` card, parses
    /// only that region, runs `edit` on it, then writes the re-serialized header back followed
    /// by every byte that came after the original header — untouched, regardless of what `edit`
    /// did.
    ///
    /// The write is atomic and edits the real file in place: it writes a temp file in the
    /// target's directory and renames it over the target. It follows symlinks (a symlinked
    /// `path` stays a symlink; its target is edited) and, on Unix, preserves the target's file
    /// mode. A crash or interruption cannot leave a truncated file.
    ///
    /// Errors with [`FitsError::MissingEnd`] if the file has no `END` card, or
    /// [`FitsError::TruncatedHeader`] if it has one but ends before the header's 2880-byte
    /// block is complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use fits_header::Header;
    ///
    /// let mut bytes = Vec::new();
    /// for card in ["OBJECT  = 'M31     '", "END"] {
    ///     let mut c = card.as_bytes().to_vec();
    ///     c.resize(80, b' ');
    ///     bytes.extend(c);
    /// }
    /// while bytes.len() % fits_header::BLOCK_LEN != 0 {
    ///     bytes.push(b' ');
    /// }
    /// let data = [1u8, 2, 3, 4]; // stand-in pixel data
    /// bytes.extend_from_slice(&data);
    ///
    /// let path = std::env::temp_dir().join("fits-header-doctest-update_file.fits");
    /// std::fs::write(&path, &bytes).unwrap();
    ///
    /// Header::update_file(&path, |h| {
    ///     h.set("OBJECT", "NGC 7000")?;
    ///     Ok(())
    /// })
    /// .unwrap();
    ///
    /// let after = std::fs::read(&path).unwrap();
    /// let header = Header::parse(&after).unwrap();
    /// assert_eq!(header.get_str("OBJECT").unwrap(), Some("NGC 7000"));
    /// assert_eq!(&after[after.len() - data.len()..], &data, "data unit preserved");
    ///
    /// std::fs::remove_file(&path).ok();
    /// ```
    pub fn update_file<P: AsRef<Path>>(
        path: P,
        edit: impl FnOnce(&mut Header) -> Result<()>,
    ) -> Result<()> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        let header_len = header_region_len(&bytes)?;
        if header_len > bytes.len() {
            // END found, but the file ends before the header block is padded out.
            return Err(FitsError::TruncatedHeader);
        }
        let tail = &bytes[header_len..];

        let mut header = Header::parse(&bytes[..header_len])?;
        edit(&mut header)?;

        let mut out = header.to_header_bytes();
        out.extend_from_slice(tail);
        write_atomic(path, &out)?;
        Ok(())
    }

    /// Create a new FITS file: the serialized header block followed by `data`.
    ///
    /// This is the convenience for the rarer case where you already have pixel data and
    /// are writing it for the first time. It creates `path` and errors if it already
    /// exists — via [`OpenOptions::create_new`](std::fs::OpenOptions::create_new), which is
    /// race-free — so this method can never overwrite or corrupt an existing file's
    /// contents. To edit a file that already exists, use [`update_file`](Self::update_file)
    /// instead.
    ///
    /// `data` is the caller's own pixel bytes, written immediately after the header block;
    /// pass `&[]` for a header-only file. This crate is header-only and never fabricates
    /// pixel data itself.
    ///
    /// Errors with [`FitsError::Io`] if the write fails, including when `path` already
    /// exists ([`io::ErrorKind::AlreadyExists`](std::io::ErrorKind::AlreadyExists)).
    ///
    /// # Examples
    ///
    /// ```
    /// use fits_header::Header;
    ///
    /// let mut header = Header::new();
    /// header.set("OBJECT", "M31").unwrap();
    ///
    /// let path = std::env::temp_dir().join("fits-header-doctest-write_to_file.fits");
    /// # std::fs::remove_file(&path).ok();
    /// header.write_to_file(&path, &[0u8; 4]).unwrap();
    ///
    /// let bytes = std::fs::read(&path).unwrap();
    /// assert_eq!(&bytes[bytes.len() - 4..], &[0u8; 4], "pixel data survived");
    /// let back = Header::parse(&bytes).unwrap();
    /// assert_eq!(back.get_str("OBJECT").unwrap(), Some("M31"));
    ///
    /// // Writing to the same path again errors instead of overwriting it.
    /// assert!(header.write_to_file(&path, &[1u8; 4]).is_err());
    ///
    /// std::fs::remove_file(&path).ok();
    /// ```
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&self.to_header_bytes())?;
        file.write_all(data)?;
        Ok(())
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
    fn resolve(&self, key: &Key) -> Result<Option<usize>> {
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
    pub fn get<T: FromCard>(&self, key: impl Into<Key>) -> Result<Option<T>> {
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
    pub fn get_str(&self, key: impl Into<Key>) -> Result<Option<&str>> {
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

    fn set_inner(&mut self, key: Key, value: Value, raw: bool) -> Result<()> {
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

    /// Updates the addressed record (or appends when the unique name is absent) in the
    /// in-memory header; nothing is written to disk — see [`Header`] to persist. The
    /// keyword must be FITS-standard (`≤8`, `A-Z 0-9 - _`); use [`set_raw`](Self::set_raw)
    /// for vendor keys.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{FitsError, Header};
    /// let mut h = Header::new();
    /// h.set("OBJECT", "M31").unwrap(); // appends
    /// h.set("OBJECT", "NGC 7000").unwrap(); // updates the in-memory record
    /// assert_eq!(h.count("OBJECT"), 1);
    ///
    /// let err = h.set("object", 1); // lowercase is not FITS-standard
    /// assert!(matches!(err, Err(FitsError::InvalidKeyword { .. })));
    /// ```
    pub fn set(&mut self, key: impl Into<Key>, value: impl IntoValue) -> Result<()> {
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
    pub fn set_raw(&mut self, keyword: &str, value: impl IntoValue) -> Result<()> {
        self.set_inner(Key::Name(keyword.to_string()), value.into_value(), true)
    }

    /// Always add a record to the in-memory header (a value card, or a commentary card
    /// for `COMMENT`/`HISTORY`/blank).
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
    pub fn append(&mut self, name: &str, value: impl IntoValue) -> Result<()> {
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
    pub fn set_comment(&mut self, key: impl Into<Key>, comment: impl Into<String>) -> Result<()> {
        if let Some(i) = self.resolve(&key.into())? {
            self.records[i].set_comment(Some(comment.into()));
        }
        Ok(())
    }

    /// Remove the addressed record from the in-memory header. Returns whether anything
    /// was removed.
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
    pub fn remove(&mut self, key: impl Into<Key>) -> Result<bool> {
        match self.resolve(&key.into())? {
            Some(i) => {
                self.records.remove(i);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Apply several mutations to the in-memory header atomically: validate every entry
    /// first, then apply all or none.
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
    pub fn set_many<K, V>(&mut self, entries: impl IntoIterator<Item = (K, V)>) -> Result<()>
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
    ) -> Result<usize> {
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

    /// Serialize the header block only (cards, `END`, padded to a 2880 multiple).
    ///
    /// This is the lower-level building block: the bytes alone, for callers who assemble
    /// the rest of the file themselves. To write a header plus pixel data straight to a
    /// new file, use [`write_to_file`](Self::write_to_file) instead — it wraps this and
    /// errors rather than overwriting an existing path.
    /// [`update_file`](Self::update_file) edits an existing file's header while preserving
    /// its data unit.
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
}

/// The byte length of the header region at the start of `bytes`: cards up to and including
/// `END`, rounded up to a [`BLOCK_LEN`] multiple. Scans the same way [`Header::parse`] does, so
/// this always agrees with what parsing would consume.
fn header_region_len(bytes: &[u8]) -> Result<usize> {
    for (i, card) in bytes.chunks_exact(CARD_LEN).enumerate() {
        let keyword = String::from_utf8_lossy(&card[..8]).trim().to_string();
        if keyword == "END" {
            let raw_len = (i + 1) * CARD_LEN;
            return Ok(raw_len.div_ceil(BLOCK_LEN) * BLOCK_LEN);
        }
    }
    Err(FitsError::MissingEnd)
}

/// Write `bytes` to `path` atomically, editing the real file in place.
///
/// Follows symlinks: if `path` is a symlink, its canonical target is resolved first and both
/// the temp file and the rename land in the target's directory, so the link is preserved and
/// the file it points at is the one edited. The temp file is renamed over the target (an atomic
/// replace), so a crash mid-write leaves the original file intact, never a truncated one. On
/// Unix the target's file mode is copied onto the temp file before the rename, so a 0600 file
/// stays 0600 instead of dropping to the umask default. Any failure after the temp file is
/// created removes it.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    // Resolve symlinks so we edit the real file in place; fall back to `path` if it does not
    // yet exist (update_file always reads first, so it does).
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let dir = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = target.file_name().map(|n| n.to_string_lossy().into_owned());
    let tmp_name = match file_name {
        Some(name) => format!(".{name}.tmp-{}", std::process::id()),
        None => format!(".fits-header.tmp-{}", std::process::id()),
    };
    let tmp_path = dir.join(tmp_name);

    let result = (|| {
        fs::write(&tmp_path, bytes)?;
        copy_mode(&target, &tmp_path)?;
        fs::rename(&tmp_path, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

/// Copy `src`'s file permissions onto `dst`. No-op off Unix, and silently ignores a missing
/// `src` (a freshly created file has no prior mode to preserve).
#[cfg(unix)]
fn copy_mode(src: &Path, dst: &Path) -> Result<()> {
    match fs::metadata(src) {
        Ok(meta) => fs::set_permissions(dst, meta.permissions())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

#[cfg(not(unix))]
fn copy_mode(_src: &Path, _dst: &Path) -> Result<()> {
    Ok(())
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
