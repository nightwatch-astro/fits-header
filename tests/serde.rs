//! The `serde` feature: public types (de)serialize and survive a JSON round-trip.
#![cfg(feature = "serde")]

mod common;
use common::build;
use fits_header::Header;

#[test]
fn header_json_roundtrip_is_semantically_equal() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "EXPTIME = 120.0 / seconds",
        "COMMENT a note",
        "HIERARCH ESO DET DIT = 10.0",
    ]);
    let h = Header::parse(&bytes).unwrap();
    let json = serde_json::to_string(&h).unwrap();
    let back: Header = serde_json::from_str(&json).unwrap();
    // Retained card bytes are #[serde(skip)]; equality is semantic, so this holds.
    assert_eq!(h, back);
}

#[test]
fn deserialized_header_still_serializes_to_fits() {
    let bytes = build(&["OBJECT  = 'M31     '", "GAIN    = 120"]);
    let h = Header::parse(&bytes).unwrap();
    let back: Header = serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
    // The byte-backing is lost over JSON, so cards re-render: byte-exactness is not
    // promised, semantic round-trip is.
    let out = back.to_header_bytes();
    assert_eq!(Header::parse(&out).unwrap(), back);
    assert_eq!(back.get::<i64>("GAIN").unwrap(), Some(120));
}
