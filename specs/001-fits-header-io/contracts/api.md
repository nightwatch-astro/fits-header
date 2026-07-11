# Public API Contract

The crate's contract is its public Rust surface. Signatures are the stable interface; bodies are
implementation. All items are re-exported from the crate root (`fits_header::…`).

## Free functions

```rust
/// Parse one FITS header unit from raw bytes.
/// Reads 80-byte cards in 2880-byte blocks, stops at END, ignores HIERARCH/COMMENT/HISTORY.
pub fn parse(bytes: &[u8]) -> Result<Header, FitsError>;
```

## `Header`

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header { /* ordered cards */ }

impl Header {
    /// Empty header.
    pub fn new() -> Self;

    /// Cards in order.
    pub fn cards(&self) -> &[Card];
    pub fn iter(&self) -> impl Iterator<Item = &Card>;
    pub fn contains(&self, keyword: &str) -> bool;

    // --- typed reads (first occurrence; None on absent or type mismatch) ---
    pub fn get<T: FromCard>(&self, keyword: &str) -> Option<T>;
    pub fn get_str(&self, keyword: &str) -> Option<&str>;   // Str content; None for "" or Literal
    pub fn get_f64(&self, keyword: &str) -> Option<f64>;
    pub fn get_i64(&self, keyword: &str) -> Option<i64>;
    pub fn get_u32(&self, keyword: &str) -> Option<u32>;
    pub fn get_bool(&self, keyword: &str) -> Option<bool>;

    // --- single CRUD ---
    /// Create or update a string-valued card (first occurrence updated, else appended).
    pub fn set(&mut self, keyword: &str, value: impl Into<String>);
    /// Create or update a literal (numeric/logical) card.
    pub fn set_literal(&mut self, keyword: &str, token: impl Into<String>);
    pub fn set_f64(&mut self, keyword: &str, value: f64);
    pub fn set_i64(&mut self, keyword: &str, value: i64);
    pub fn set_bool(&mut self, keyword: &str, value: bool);
    /// Attach or replace a card's inline comment.
    pub fn set_comment(&mut self, keyword: &str, comment: impl Into<String>);
    /// Remove all occurrences of a keyword. Returns how many were removed.
    pub fn remove(&mut self, keyword: &str) -> usize;

    // --- atomic batch CRUD (validate all, then apply all or none) ---
    pub fn set_many<K, V>(&mut self, entries: impl IntoIterator<Item = (K, V)>)
        -> Result<(), FitsError>
    where K: AsRef<str>, V: Into<String>;
    pub fn remove_many<K: AsRef<str>>(&mut self, keywords: impl IntoIterator<Item = K>) -> usize;

    // --- serialization ---
    /// Serialize to a valid FITS object: structural cards, header cards, END,
    /// 2880-padding, minimal data block.
    pub fn to_bytes(&self, structural: &StructuralHints) -> Vec<u8>;
}
```

Guarantees:
- `parse(h.to_bytes(&hints))` yields a header equal to `h` for representative headers (round-trip).
- Every card emitted by `to_bytes` is exactly `CARD_LEN` (80) bytes; the output length is a multiple
  of `BLOCK_LEN` (2880).

## `Card` / `Value`

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Card { pub keyword: String, pub value: Value, pub comment: Option<String> }

#[derive(Clone, Debug, PartialEq)]
pub enum Value { Str(String), Literal(String) }
```

## `StructuralHints`

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct StructuralHints { pub bitpix: i64, pub naxis1: u32, pub naxis2: u32 }

impl Default for StructuralHints { /* 1×1 8-bit image */ }
```

## `FromCard`

```rust
pub trait FromCard: Sized { fn from_card(card: &Card) -> Option<Self>; }
// impls: String, f64, i64, u32, bool, time::PrimitiveDateTime
```

## Helper functions

```rust
/// Sexagesimal parse (space- or colon-separated, optional fractional seconds).
pub fn sexagesimal_ra_to_deg(s: &str) -> Option<f64>;   // "10 00 00" -> 150.0
pub fn sexagesimal_dec_to_deg(s: &str) -> Option<f64>;  // "-00 30 00" -> -0.5 (sign preserved)

/// Sexagesimal format (re-parses to the input degrees within fixed precision).
pub fn deg_to_sexagesimal_ra(deg: f64) -> String;
pub fn deg_to_sexagesimal_dec(deg: f64) -> String;

/// Lenient numeric parse ("20.0" -> 20).
pub fn parse_f64(s: &str) -> Option<f64>;
pub fn parse_i64(s: &str) -> Option<i64>;

/// Date/time (FITS ISO-8601, timezone-naive) and Modified Julian Date.
pub fn parse_datetime(s: &str) -> Option<time::PrimitiveDateTime>;
pub fn format_datetime(dt: &time::PrimitiveDateTime) -> String;
pub fn mjd_to_datetime(mjd: f64) -> time::PrimitiveDateTime;
pub fn datetime_to_mjd(dt: &time::PrimitiveDateTime) -> f64;
```

## Error

```rust
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FitsError {
    #[error("keyword '{keyword}' exceeds 8 characters")]
    KeywordTooLong { keyword: String },
    #[error("keyword '{keyword}' contains characters outside A-Z 0-9 - _")]
    InvalidKeyword { keyword: String },
}
```

## Feature flags

- `serde` *(off by default)* — derives `Serialize`/`Deserialize` on `Header`, `Card`, `Value`,
  `StructuralHints`.
