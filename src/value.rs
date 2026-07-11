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
///
/// # Examples
///
/// ```
/// # use fits_header::{Header, Literal};
/// let mut h = Header::new();
/// h.set("BSCALE", Literal("1.000")).unwrap(); // kept as-is, not reformatted
/// assert_eq!(h.get::<String>("BSCALE").unwrap().as_deref(), Some("1.000"));
/// ```
pub struct Literal<S: Into<String>>(pub S);

impl<S: Into<String>> IntoValue for Literal<S> {
    fn into_value(self) -> Value {
        Value::Literal(self.0.into())
    }
}

/// Write a float with a fixed number of decimal places.
///
/// # Examples
///
/// ```
/// # use fits_header::{Fixed, Header};
/// let mut h = Header::new();
/// h.set("EXPTIME", Fixed(120.0, 2)).unwrap();
/// assert_eq!(h.get::<String>("EXPTIME").unwrap().as_deref(), Some("120.00"));
/// ```
pub struct Fixed(pub f64, pub u8);

impl IntoValue for Fixed {
    fn into_value(self) -> Value {
        Value::Literal(format!("{:.*}", self.1 as usize, self.0))
    }
}

/// Write a float in scientific notation with `N` significant digits (uppercase `E`).
///
/// # Examples
///
/// ```
/// # use fits_header::{Header, Sci};
/// let mut h = Header::new();
/// h.set("BZERO", Sci(0.000123, 3)).unwrap();
/// assert_eq!(h.get::<String>("BZERO").unwrap().as_deref(), Some("1.23E-4"));
/// ```
pub struct Sci(pub f64, pub u8);

impl IntoValue for Sci {
    fn into_value(self) -> Value {
        let prec = self.1.saturating_sub(1) as usize;
        Value::Literal(format!("{:.*E}", prec, self.0))
    }
}

// --- number parsing/formatting -------------------------------------------

/// Parse a float, accepting the Fortran `D` exponent (`1.5D3`).
///
/// # Examples
///
/// ```
/// assert_eq!(fits_header::parse_f64("1.5D3"), Some(1500.0));
/// assert_eq!(fits_header::parse_f64("120.0"), Some(120.0));
/// ```
pub fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.contains(['D', 'd']) {
        t.replace(['D', 'd'], "E").parse().ok()
    } else {
        t.parse().ok()
    }
}

/// Parse an integer, accepting decimal-form integers (`"20.0"` → `20`).
///
/// # Examples
///
/// ```
/// assert_eq!(fits_header::parse_i64("20.0"), Some(20));
/// assert_eq!(fits_header::parse_i64("20.5"), None);
/// ```
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
    if s.len() > 20 {
        // Positional display expands extreme magnitudes into hundreds of digits, which no
        // 80-byte card can hold; exponent form is equally exact and always fits.
        s = format!("{x:E}");
    } else if !s.contains('.') {
        s.push_str(".0");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;

    fn literal(token: &str) -> Record {
        Record::value("K", Value::Literal(token.to_string()), None)
    }

    #[test]
    fn format_f64_normalizes() {
        assert_eq!(format_f64(120.0), "120.0");
        assert_eq!(format_f64(0.5), "0.5");
        assert_eq!(format_f64(-0.0), "-0.0");
        assert_eq!(format_f64(f64::NAN), "NaN");
        // Extreme magnitudes switch to exponent form instead of hundreds of digits.
        assert_eq!(format_f64(1e300), "1E300");
        assert_eq!(format_f64(1e-300), "1E-300");
        assert_eq!(format_f64(f64::MIN_POSITIVE), "2.2250738585072014E-308");
        // Every emitted token fits a fixed-format value field or close to it, and
        // round-trips exactly.
        for x in [0.1, 1.0 / 3.0, 6.02214076e23, 1e300, -1e-300, f64::MAX] {
            let s = format_f64(x);
            assert!(s.len() <= 24, "token too long: {s}");
            assert_eq!(s.parse::<f64>().unwrap(), x);
        }
    }

    #[test]
    fn parse_f64_accepts_fortran_exponent() {
        assert_eq!(parse_f64("1.5D3"), Some(1500.0));
        assert_eq!(parse_f64("1.5d-2"), Some(0.015));
        assert_eq!(parse_f64(" 2.0 "), Some(2.0));
        assert_eq!(parse_f64("abc"), None);
    }

    #[test]
    fn parse_i64_is_lenient_but_rejects_fractions() {
        assert_eq!(parse_i64("20.0"), Some(20));
        assert_eq!(parse_i64("20.5"), None);
        assert_eq!(parse_i64("1e3"), Some(1000));
        // Larger than i64 → None, not a wrap.
        assert_eq!(parse_i64("170141183460469231731687303715884105727"), None);
        assert_eq!(parse_i64("inf"), None);
    }

    #[test]
    fn int_narrowing_fails_closed() {
        assert_eq!(u8::from_card(&literal("300")), None);
        assert_eq!(u8::from_card(&literal("255")), Some(255));
        assert_eq!(u32::from_card(&literal("-1")), None);
        assert_eq!(i16::from_card(&literal("-32768")), Some(-32768));
    }

    #[test]
    fn bool_from_card_variants() {
        assert_eq!(bool::from_card(&literal("T")), Some(true));
        assert_eq!(bool::from_card(&literal("1")), Some(true));
        assert_eq!(bool::from_card(&literal("F")), Some(false));
        assert_eq!(bool::from_card(&literal("0")), Some(false));
        assert_eq!(bool::from_card(&literal("yes")), None);
    }

    #[test]
    fn string_from_card_reads_literal_token() {
        assert_eq!(
            String::from_card(&literal("120.0")).as_deref(),
            Some("120.0")
        );
        // An empty Str value reads as absent.
        let empty = Record::value("K", Value::Str(String::new()), None);
        assert_eq!(String::from_card(&empty), None);
    }

    #[test]
    fn into_value_wrappers() {
        assert_eq!(true.into_value(), Value::Literal("T".to_string()));
        assert_eq!(false.into_value(), Value::Literal("F".to_string()));
        assert_eq!(
            Literal("007").into_value(),
            Value::Literal("007".to_string())
        );
        assert_eq!(
            Fixed(1.5, 3).into_value(),
            Value::Literal("1.500".to_string())
        );
        assert_eq!(
            Sci(0.000123, 3).into_value(),
            Value::Literal("1.23E-4".to_string())
        );
        assert_eq!(
            Sci(1234.0, 1).into_value(),
            Value::Literal("1E3".to_string())
        );
        assert_eq!("s".into_value(), Value::Str("s".to_string()));
        assert_eq!((&"s".to_string()).into_value(), Value::Str("s".to_string()));
        assert_eq!(42u8.into_value(), Value::Literal("42".to_string()));
        assert_eq!((-3isize).into_value(), Value::Literal("-3".to_string()));
        assert_eq!(2.5f32.into_value(), Value::Literal("2.5".to_string()));
    }

    #[test]
    fn datetime_from_card() {
        let r = Record::value("K", Value::Str("2026-07-11T22:15:03".to_string()), None);
        let dt = time::PrimitiveDateTime::from_card(&r).unwrap();
        assert_eq!(crate::dates::format_datetime(&dt), "2026-07-11T22:15:03");
        assert_eq!(time::PrimitiveDateTime::from_card(&literal("nope")), None);
    }
}
