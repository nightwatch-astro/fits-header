//! US4 — coordinate/epoch helpers (behind the `coords` feature).
#![cfg(feature = "coords")]

use fits_header::{
    datetime_to_mjd, deg_to_sexagesimal_dec, deg_to_sexagesimal_ra, mjd_to_datetime,
    parse_datetime, sexagesimal_dec_to_deg, sexagesimal_ra_to_deg,
};

#[test]
fn sexagesimal_parse() {
    assert_eq!(sexagesimal_ra_to_deg("10 00 00"), Some(150.0));
    assert_eq!(sexagesimal_ra_to_deg("10:00:00"), Some(150.0));
    assert_eq!(sexagesimal_dec_to_deg("-00 30 00"), Some(-0.5));
    assert_eq!(sexagesimal_dec_to_deg("+45 00 00"), Some(45.0));
}

#[test]
fn sexagesimal_format_roundtrip() {
    for deg in [150.0, 0.0, 359.9, 12.3456] {
        let s = deg_to_sexagesimal_ra(deg);
        assert!((sexagesimal_ra_to_deg(&s).unwrap() - deg).abs() < 1e-3);
    }
    let d = deg_to_sexagesimal_dec(-0.5);
    assert!(d.starts_with('-'));
    assert!((sexagesimal_dec_to_deg(&d).unwrap() + 0.5).abs() < 1e-3);
}

#[test]
fn mjd_known_and_roundtrip() {
    let y2k = parse_datetime("2000-01-01T00:00:00").unwrap();
    assert!((datetime_to_mjd(&y2k) - 51544.0).abs() < 1e-6);

    let dt = parse_datetime("2026-07-11T12:34:56").unwrap();
    let mjd = datetime_to_mjd(&dt);
    let back = mjd_to_datetime(mjd);
    assert!((datetime_to_mjd(&back) - mjd).abs() < 1e-9);
}
