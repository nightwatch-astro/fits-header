//! Record selectors.

/// Selects a record by keyword.
///
/// A bare name is **strict**: `get`/`set`/`remove` error with
/// [`FitsError::AmbiguousKeyword`](crate::FitsError::AmbiguousKeyword) if the keyword is
/// duplicated. The `(name, occurrence)` form targets exactly one record (0-based).
///
/// # Examples
///
/// ```
/// # use fits_header::Header;
/// let mut h = Header::new();
/// h.append("GAIN", 100).unwrap();
/// h.append("GAIN", 200).unwrap();
///
/// assert!(h.get::<i64>("GAIN").is_err()); // ambiguous bare name
/// assert_eq!(h.get::<i64>(("GAIN", 1)).unwrap(), Some(200)); // explicit occurrence
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// The sole occurrence of a keyword (strict).
    Name(String),
    /// The n-th (0-based) occurrence of a keyword.
    Occurrence(String, usize),
}

impl Key {
    /// The keyword this key refers to.
    pub fn name(&self) -> &str {
        match self {
            Key::Name(n) | Key::Occurrence(n, _) => n,
        }
    }

    /// The selected occurrence index, if any.
    pub fn occurrence(&self) -> Option<usize> {
        match self {
            Key::Name(_) => None,
            Key::Occurrence(_, n) => Some(*n),
        }
    }
}

impl From<&str> for Key {
    fn from(name: &str) -> Self {
        Key::Name(name.to_string())
    }
}

impl From<String> for Key {
    fn from(name: String) -> Self {
        Key::Name(name)
    }
}

impl From<&String> for Key {
    fn from(name: &String) -> Self {
        Key::Name(name.clone())
    }
}

impl From<(&str, usize)> for Key {
    fn from((name, n): (&str, usize)) -> Self {
        Key::Occurrence(name.to_string(), n)
    }
}

impl From<(String, usize)> for Key {
    fn from((name, n): (String, usize)) -> Self {
        Key::Occurrence(name, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_and_accessors() {
        let k: Key = "GAIN".into();
        assert_eq!(k, Key::Name("GAIN".to_string()));
        assert_eq!(k.name(), "GAIN");
        assert_eq!(k.occurrence(), None);

        let k: Key = ("GAIN".to_string()).into();
        assert_eq!(k.name(), "GAIN");
        let k: Key = (&"GAIN".to_string()).into();
        assert_eq!(k.name(), "GAIN");

        let k: Key = ("GAIN", 1).into();
        assert_eq!(k, Key::Occurrence("GAIN".to_string(), 1));
        assert_eq!(k.name(), "GAIN");
        assert_eq!(k.occurrence(), Some(1));
        let k: Key = ("GAIN".to_string(), 2).into();
        assert_eq!(k.occurrence(), Some(2));
    }
}
