// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Record selectors.

/// Selects a record by keyword.
///
/// A bare name is strict: `get`/`set`/`remove` report an ambiguity when duplicated. Use
/// `(name, occurrence)` to select one standard-keyword occurrence. HIERARCH keys are explicit
/// via [`Key::hierarch`] or [`Key::hierarch_occurrence`], so they cannot collide with standard
/// keywords that happen to have the same visible name.
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
    /// The sole occurrence of a hierarchical keyword (strict).
    Hierarch(String),
    /// The n-th occurrence of a hierarchical keyword.
    HierarchOccurrence(String, usize),
}
impl Key {
    /// The keyword name addressed by this key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Key;
    /// assert_eq!(Key::hierarch("ESO DET DIT").name(), "ESO DET DIT");
    /// ```
    pub fn name(&self) -> &str {
        match self {
            Key::Name(n)
            | Key::Occurrence(n, _)
            | Key::Hierarch(n)
            | Key::HierarchOccurrence(n, _) => n,
        }
    }
    /// The selected occurrence index, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::Key;
    /// assert_eq!(Key::hierarch_occurrence("ESO DET DIT", 1).occurrence(), Some(1));
    /// ```
    pub fn occurrence(&self) -> Option<usize> {
        match self {
            Key::Name(_) | Key::Hierarch(_) => None,
            Key::Occurrence(_, n) | Key::HierarchOccurrence(_, n) => Some(*n),
        }
    }

    pub(crate) fn is_hierarch(&self) -> bool {
        matches!(self, Key::Hierarch(_) | Key::HierarchOccurrence(_, _))
    }
    /// Select the sole occurrence of a HIERARCH keyword by its long name.
    ///
    /// Supply only the text after `HIERARCH`, including spaces in multi-word names.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{Header, Key};
    /// let mut h = Header::new();
    /// h.set(Key::hierarch("ESO DET DIT"), 10.0).unwrap();
    /// assert_eq!(h.get::<f64>(Key::hierarch("ESO DET DIT")).unwrap(), Some(10.0));
    /// ```
    pub fn hierarch(name: impl Into<String>) -> Self {
        Key::Hierarch(name.into())
    }

    /// Select the zero-based occurrence of a HIERARCH keyword by its long name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fits_header::{Header, Key};
    /// let mut h = Header::new();
    /// h.set(Key::hierarch("ESO DET DIT"), 10.0).unwrap();
    /// assert_eq!(h.get::<f64>(Key::hierarch_occurrence("ESO DET DIT", 0)).unwrap(), Some(10.0));
    /// ```
    pub fn hierarch_occurrence(name: impl Into<String>, occurrence: usize) -> Self {
        Key::HierarchOccurrence(name.into(), occurrence)
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

impl From<(&String, usize)> for Key {
    fn from((name, n): (&String, usize)) -> Self {
        Key::Occurrence(name.clone(), n)
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
