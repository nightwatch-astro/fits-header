// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! US3 — byte-exact write-back and serialization.

mod common;
use common::build;
use fits_header::Header;

#[test]
fn untouched_cards_are_byte_exact() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "EXPTIME = 120.0 / seconds",
        "FILTER  = 'Ha      '",
    ]);
    let mut h = Header::parse(&bytes).unwrap();
    h.set("EXPTIME", 300.0).unwrap();
    let out = h.to_header_bytes();

    // OBJECT (card 1) and FILTER (card 3) are untouched → identical bytes.
    assert_eq!(&out[80..160], &bytes[80..160]);
    assert_eq!(&out[240..320], &bytes[240..320]);
    // EXPTIME (card 2) changed.
    assert_ne!(&out[160..240], &bytes[160..240]);
    // and re-reads correctly.
    let re = Header::parse(&out).unwrap();
    assert_eq!(re.get::<f64>("EXPTIME").unwrap(), Some(300.0));
}

#[test]
fn unmodified_header_roundtrips_byte_exact() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "COMMENT some note",
    ]);
    let h = Header::parse(&bytes).unwrap();
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
fn untouched_continue_run_is_byte_exact() {
    let bytes = build(&[
        "LONGKEY = 'aaa&'",
        "CONTINUE  'bbb&'",
        "CONTINUE  'ccc'",
        "GAIN    = 1",
    ]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("LONGKEY").unwrap(), Some("aaabbbccc"));
    assert_eq!(
        h.to_header_bytes(),
        bytes,
        "untouched run re-emits verbatim"
    );
}

#[test]
fn edited_continue_run_resplits_with_longstrn() {
    let bytes = build(&["LONGKEY = 'aaa&'", "CONTINUE  'bbb'", "GAIN    = 1"]);
    let mut h = Header::parse(&bytes).unwrap();
    let long = "x".repeat(200);
    h.set("LONGKEY", long.as_str()).unwrap();
    let out = h.to_header_bytes();

    let re = Header::parse(&out).unwrap();
    assert_eq!(re.get_str("LONGKEY").unwrap(), Some(long.as_str()));
    assert_eq!(re.count("LONGSTRN"), 1, "convention card added on re-split");
    assert_eq!(re.get::<i64>("GAIN").unwrap(), Some(1), "neighbors survive");
}

#[test]
fn raw_keyword_survives_serialization() {
    let mut h = Header::new();
    h.set_raw("obj", "x").unwrap();
    let re = Header::parse(&h.to_header_bytes()).unwrap();
    assert_eq!(re.get_str("obj").unwrap(), Some("x"));
}

#[test]
fn long_string_uses_continue_and_roundtrips() {
    let mut h = Header::new();
    let long = "abcdefghij".repeat(15); // 150 chars
    h.set("LONGKEY", long.as_str()).unwrap();
    let out = h.to_header_bytes();
    assert_eq!(out.len() % 80, 0);

    let re = Header::parse(&out).unwrap();
    assert_eq!(re.get_str("LONGKEY").unwrap(), Some(long.as_str()));
    assert_eq!(re.count("LONGSTRN"), 1);
    assert_eq!(re.count("LONGKEY"), 1);
}
