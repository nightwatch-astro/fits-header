//! Core value helpers: lenient numbers, dates, and formatting wrappers.

use fits_header::{format_datetime, parse_datetime, parse_f64, parse_i64, Fixed, Header, Sci};

#[test]
fn lenient_numeric() {
    assert_eq!(parse_i64("20.0"), Some(20));
    assert_eq!(parse_i64("42"), Some(42));
    assert_eq!(parse_i64("-7"), Some(-7));
    assert_eq!(parse_f64("1.5D3"), Some(1500.0));
    assert_eq!(parse_i64("nope"), None);
}

#[test]
fn datetime_roundtrip() {
    for s in [
        "2026-07-11T22:15:03",
        "2026-07-11T22:15:03.5",
        "1999-12-31T23:59:59",
    ] {
        let dt = parse_datetime(s).unwrap();
        assert_eq!(format_datetime(&dt), s);
    }
}

#[test]
fn datetime_via_get() {
    let mut h = Header::new();
    h.set("DATE-OBS", "2026-07-11T22:15:03").unwrap();
    let dt: Option<time::PrimitiveDateTime> = h.get("DATE-OBS").unwrap();
    let dt = dt.expect("parses");
    assert_eq!(format_datetime(&dt), "2026-07-11T22:15:03");
}

#[test]
fn fixed_and_sci_and_default_float() {
    let mut h = Header::new();
    h.set("A", Fixed(120.0, 2)).unwrap();
    assert_eq!(h.get::<String>("A").unwrap().as_deref(), Some("120.00"));
    h.set("B", Sci(0.000123, 3)).unwrap();
    assert_eq!(h.get::<String>("B").unwrap().as_deref(), Some("1.23E-4"));
    h.set("C", 120.0).unwrap();
    assert_eq!(h.get::<String>("C").unwrap().as_deref(), Some("120.0"));
    h.set("D", 42_i64).unwrap();
    assert_eq!(h.get::<String>("D").unwrap().as_deref(), Some("42"));
}
