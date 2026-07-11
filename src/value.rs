//! Typed reads (`FromCard`) and writes (`IntoValue`), plus number parsing/formatting.

use crate::record::{Record, Value};

/// Convert a record's value into a Rust type. The extension point behind
/// [`Header::get`](crate::Header::get).
pub trait FromCard: Sized {
    /// Extract `Self` from a record, or `None` if absent/unparseable.
    fn from_card(record: &Record) -> Option<Self>;
}

/// Convert a Rust value into a [`Value`]. The extension point behind
/// [`Header::set`](crate::Header::set) / [`append`](crate::Header::append).
pub trait IntoValue {
    /// The value payload to store.
    fn into_value(self) -> Value;
}

// --- FromCard -------------------------------------------------------------

impl FromCard for String {
    fn from_card(record: &Record) -> Option<Self> {
        record.value_text().map(str::to_string)
    }
}

impl FromCard for bool {
    fn from_card(record: &Record) -> Option<Self> {
        match record.value_text()?.trim() {
            "T" | "1" => Some(true),
            "F" | "0" => Some(false),
            _ => None,
        }
    }
}

impl FromCard for f64 {
    fn from_card(record: &Record) -> Option<Self> {
        record.value_text().and_then(parse_f64)
    }
}

impl FromCard for f32 {
    fn from_card(record: &Record) -> Option<Self> {
        record.value_text().and_then(parse_f64).map(|v| v as f32)
    }
}

macro_rules! from_card_int {
    ($($t:ty),* $(,)?) => {$(
        impl FromCard for $t {
            fn from_card(record: &Record) -> Option<Self> {
                record.value_text().and_then(lenient_i128).and_then(|v| <$t>::try_from(v).ok())
            }
        }
    )*};
}
from_card_int!(i64, i32, u64, u32, i16, u16, i8, u8);

impl FromCard for time::PrimitiveDateTime {
    fn from_card(record: &Record) -> Option<Self> {
        record.value_text().and_then(crate::dates::parse_datetime)
    }
}

// --- IntoValue ------------------------------------------------------------

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::Str(self.to_string())
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::Str(self)
    }
}

impl IntoValue for &String {
    fn into_value(self) -> Value {
        Value::Str(self.clone())
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Literal(if self { "T" } else { "F" }.to_string())
    }
}

macro_rules! into_value_int {
    ($($t:ty),* $(,)?) => {$(
        impl IntoValue for $t {
            fn into_value(self) -> Value { Value::Literal(self.to_string()) }
        }
    )*};
}
into_value_int!(i64, i32, u64, u32, i16, u16, i8, u8, usize, isize);

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::Literal(format_f64(self))
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::Literal(format_f64(self as f64))
    }
}

/// Write a literal token verbatim (numeric text a vendor supplied, or a value you don't want
/// reformatted).
pub struct Literal<S: Into<String>>(pub S);

impl<S: Into<String>> IntoValue for Literal<S> {
    fn into_value(self) -> Value {
        Value::Literal(self.0.into())
    }
}

/// Write a float with a fixed number of decimal places.
pub struct Fixed(pub f64, pub u8);

impl IntoValue for Fixed {
    fn into_value(self) -> Value {
        Value::Literal(format!("{:.*}", self.1 as usize, self.0))
    }
}

/// Write a float in scientific notation with `N` significant digits (uppercase `E`).
pub struct Sci(pub f64, pub u8);

impl IntoValue for Sci {
    fn into_value(self) -> Value {
        let prec = self.1.saturating_sub(1) as usize;
        Value::Literal(format!("{:.*E}", prec, self.0))
    }
}

// --- number parsing/formatting -------------------------------------------

/// Parse a float, accepting the Fortran `D` exponent (`1.5D3`).
pub fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.contains(['D', 'd']) {
        t.replace(['D', 'd'], "E").parse().ok()
    } else {
        t.parse().ok()
    }
}

/// Parse an integer, accepting decimal-form integers (`"20.0"` → `20`).
pub fn parse_i64(s: &str) -> Option<i64> {
    lenient_i128(s).and_then(|v| i64::try_from(v).ok())
}

fn lenient_i128(s: &str) -> Option<i128> {
    let t = s.trim();
    if let Ok(i) = t.parse::<i128>() {
        return Some(i);
    }
    let f = parse_f64(t)?;
    (f.fract() == 0.0 && f.is_finite()).then_some(f as i128)
}

/// Shortest round-trip `f64` text, normalized to read as a float (decimal point or `E`).
pub(crate) fn format_f64(x: f64) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let mut s = format!("{x}");
    if s.contains('e') {
        s = s.replacen('e', "E", 1);
    }
    if !s.contains('.') && !s.contains('E') {
        s.push_str(".0");
    }
    s
}
