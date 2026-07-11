//! US3 — byte-exact write-back and serialization.

mod common;
use common::build;
use fits_header::{parse, Header, StructuralHints};

#[test]
fn untouched_cards_are_byte_exact() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "EXPTIME = 120.0 / seconds",
        "FILTER  = 'Ha      '",
    ]);
    let mut h = parse(&bytes).unwrap();
    h.set("EXPTIME", 300.0).unwrap();
    let out = h.to_header_bytes();

    // OBJECT (card 1) and FILTER (card 3) are untouched → identical bytes.
    assert_eq!(&out[80..160], &bytes[80..160]);
    assert_eq!(&out[240..320], &bytes[240..320]);
    // EXPTIME (card 2) changed.
    assert_ne!(&out[160..240], &bytes[160..240]);
    // and re-reads correctly.
    let re = parse(&out).unwrap();
    assert_eq!(re.get::<f64>("EXPTIME").unwrap(), Some(300.0));
}

#[test]
fn unmodified_header_roundtrips_byte_exact() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "COMMENT some note",
    ]);
    let h = parse(&bytes).unwrap();
    assert_eq!(h.to_header_bytes(), bytes);
}

#[test]
fn cards_are_80_and_padded_2880() {
    let mut h = Header::new();
    h.set("OBJECT", "X").unwrap();
    let out = h.to_header_bytes();
    assert_eq!(out.len() % 80, 0);
    assert_eq!(out.len() % 2880, 0);
}

#[test]
fn synthesizes_structural_when_absent() {
    let mut h = Header::new();
    h.set("OBJECT", "X").unwrap();
    let out = h.to_bytes(&StructuralHints::default());
    assert!(String::from_utf8_lossy(&out[0..80]).starts_with("SIMPLE"));
    assert_eq!(out.len() % 2880, 0);
    let re = parse(&out).unwrap();
    assert_eq!(re.count("SIMPLE"), 1);
    assert_eq!(re.get::<i64>("NAXIS").unwrap(), Some(2));
}

#[test]
fn does_not_duplicate_existing_structural() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "BITPIX  =                    8",
        "NAXIS   =                    0",
        "OBJECT  = 'X       '",
    ]);
    let h = parse(&bytes).unwrap();
    let out = h.to_bytes(&StructuralHints::default());
    let re = parse(&out).unwrap();
    assert_eq!(re.count("SIMPLE"), 1);
    assert_eq!(re.count("BITPIX"), 1);
}

#[test]
fn long_string_uses_continue_and_roundtrips() {
    let mut h = Header::new();
    let long = "abcdefghij".repeat(15); // 150 chars
    h.set("LONGKEY", long.as_str()).unwrap();
    let out = h.to_header_bytes();
    assert_eq!(out.len() % 80, 0);

    let re = parse(&out).unwrap();
    assert_eq!(re.get_str("LONGKEY").unwrap(), Some(long.as_str()));
    assert_eq!(re.count("LONGSTRN"), 1);
    assert_eq!(re.count("LONGKEY"), 1);
}
