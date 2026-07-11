# Public API Contract

The crate's contract is its public Rust surface. Signatures are the stable interface; bodies are
implementation. All items are re-exported from the crate root (`fits_header::…`).

## Free functions

```rust
/// Parse one FITS header unit from raw bytes (80-byte cards in 2880-byte blocks, stop at END).
/// Retains every card; reassembles CONTINUE runs into one logical value.
pub fn parse(bytes: &[u8]) -> Result<Header, FitsError>;
```

## `Header`

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header { /* ordered records */ }

impl Header {
    pub fn new() -> Self;

    // read-only access to the ordered records (escape hatch)
    pub fn cards(&self) -> &[Record];
    pub fn iter(&self) -> impl Iterator<Item = &Record>;
    pub fn count(&self, name: &str) -> usize;

    // --- typed reads ---
    // key: "NAME" (strict; Err if duplicated) or ("NAME", n) (n-th occurrence)
    pub fn get<T: FromCard>(&self, key: impl Into<Key>) -> Result<Option<T>, FitsError>;
    pub fn get_str(&self, key: impl Into<Key>) -> Result<Option<&str>, FitsError>;
    pub fn get_all<T: FromCard>(&self, name: &str) -> Vec<T>;

    // --- CRUD ---
    /// Update the addressed record in place, or append when the unique name is absent.
    pub fn set(&mut self, key: impl Into<Key>, value: impl IntoValue) -> Result<(), FitsError>;
    /// Like `set` but accepts any ≤8-char printable-ASCII keyword (vendor escape hatch).
    pub fn set_raw(&mut self, keyword: &str, value: impl IntoValue) -> Result<(), FitsError>;
    /// Always add a record (value card, or commentary card for COMMENT/HISTORY/blank).
    pub fn append(&mut self, name: &str, value: impl IntoValue) -> Result<(), FitsError>;
    /// Attach or replace the addressed record's inline comment.
    pub fn set_comment(&mut self, key: impl Into<Key>, comment: impl Into<String>) -> Result<(), FitsError>;
    /// Remove the addressed record (and its CONTINUE run). Returns whether anything was removed.
    pub fn remove(&mut self, key: impl Into<Key>) -> Result<bool, FitsError>;

    // --- atomic batch (validate all, then apply all or none) ---
    pub fn set_many<K, V>(&mut self, entries: impl IntoIterator<Item = (K, V)>) -> Result<(), FitsError>
    where K: Into<Key>, V: IntoValue;
    pub fn remove_many<K: Into<Key>>(&mut self, keys: impl IntoIterator<Item = K>) -> Result<usize, FitsError>;

    // --- serialization ---
    /// Header block only (cards, END, padded to 2880) — splice onto existing file data.
    pub fn to_header_bytes(&self) -> Vec<u8>;
    /// Standalone FITS object (header + minimal data block). Synthesizes SIMPLE/BITPIX/NAXIS*
    /// only when absent; StructuralHints is a fallback. Errs with DataTooLarge when the
    /// declared data segment exceeds MAX_ZERO_FILL (1 GiB) instead of zero-filling it.
    pub fn to_bytes(&self, structural: &StructuralHints) -> Result<Vec<u8>, FitsError>;
}
```

Guarantees:
- For a parsed header, `to_header_bytes` emits every untouched card byte-for-byte; `parse` of the output
  is semantically equal to the header.
- Every emitted card is `CARD_LEN` (80) bytes; output length is a multiple of `BLOCK_LEN` (2880).

## `Key`

```rust
pub enum Key { Name(String), Occurrence(String, usize) }
impl From<&str> for Key;            // "GAIN"        -> Name
impl From<String> for Key;
impl From<(&str, usize)> for Key;   // ("GAIN", 1)   -> Occurrence
```

## `Record` / `Value`

```rust
#[derive(Clone, Debug)]                 // PartialEq is semantic (compares `kind` only)
pub struct Record {
    pub kind: RecordKind,
    // retained physical card bytes + modified flag, private; a long-string run keeps all its cards
}

pub enum RecordKind {
    Value { keyword: String, value: Value, comment: Option<String> },
    Commentary { keyword: String, text: String },   // COMMENT / HISTORY / blank
    Opaque { text: String },                          // HIERARCH / unrecognized
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value { Str(String), Literal(String) }
```

A `CONTINUE` run is one `Value` record that retains the bytes of all its physical cards; there is no
separate continuation variant. `Record` equality ignores the retained bytes.

## Conversion traits & wrappers

```rust
pub trait FromCard: Sized { fn from_card(record: &Record) -> Option<Self>; }
// impls: String, f64, i64, u32, bool, time::PrimitiveDateTime

pub trait IntoValue { fn into_value(self) -> Value; }
// impls: &str, String, f64, i64, u32, bool, Literal, Fixed, Sci

pub struct Literal<S: Into<String>>(pub S);   // verbatim literal token
pub struct Fixed(pub f64, pub u8);            // fixed decimal places
pub struct Sci(pub f64, pub u8);              // scientific, N significant digits
```

## `StructuralHints`

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct StructuralHints { pub bitpix: i64, pub naxis1: u32, pub naxis2: u32 }
impl Default for StructuralHints { /* 1×1 8-bit image */ }
```

## Helper functions

```rust
// always available
pub fn parse_f64(s: &str) -> Option<f64>;
pub fn parse_i64(s: &str) -> Option<i64>;                // "20.0" -> 20
pub fn parse_datetime(s: &str) -> Option<time::PrimitiveDateTime>;
pub fn format_datetime(dt: &time::PrimitiveDateTime) -> String;

// #[cfg(feature = "coords")]
pub fn sexagesimal_ra_to_deg(s: &str) -> Option<f64>;    // "10 00 00" -> 150.0
pub fn sexagesimal_dec_to_deg(s: &str) -> Option<f64>;   // "-00 30 00" -> -0.5
pub fn deg_to_sexagesimal_ra(deg: f64) -> String;
pub fn deg_to_sexagesimal_dec(deg: f64) -> String;
pub fn mjd_to_datetime(mjd: f64) -> time::PrimitiveDateTime;
pub fn datetime_to_mjd(dt: &time::PrimitiveDateTime) -> f64;
```

## Error

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FitsError {
    #[error("keyword '{keyword}' occurs {count} times; select an occurrence")]
    AmbiguousKeyword { keyword: String, count: usize },
    #[error("keyword '{keyword}' exceeds 8 characters")]
    KeywordTooLong { keyword: String },
    #[error("keyword '{keyword}' contains characters outside A-Z 0-9 - _")]
    InvalidKeyword { keyword: String },
    #[error("keyword '{keyword}' has no occurrence {occurrence} (found {count})")]
    OccurrenceOutOfRange { keyword: String, occurrence: usize, count: usize },
    #[error("declared data size of {declared} bytes exceeds the to_bytes zero-fill cap ({max})")]
    DataTooLarge { declared: u64, max: u64 },
}
```

## Constants

```rust
pub const CARD_LEN: usize = 80;       // bytes per header card
pub const BLOCK_LEN: usize = 2880;    // bytes per FITS block (36 cards)
pub const MAX_ZERO_FILL: u64 = 1 << 30; // largest data segment to_bytes will zero-fill
```

## Feature flags

- `serde` *(off by default)* — derives `Serialize`/`Deserialize` on `Header`, `Record`, `Value`,
  `StructuralHints`.
- `coords` *(off by default)* — sexagesimal RA/Dec and MJD↔calendar helpers.
