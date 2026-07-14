// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! US1 — faithful read.

mod common;
use common::{build, card};
use fits_header::Header;

#[test]
fn reads_string_and_numeric() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "OBJECT  = 'M31     '",
        "EXPTIME = 120.0 / seconds",
    ]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31"));
    assert_eq!(h.get::<f64>("EXPTIME").unwrap(), Some(120.0));
    assert_eq!(
        h.get::<String>("EXPTIME").unwrap().as_deref(),
        Some("120.0")
    );
}

#[test]
fn retains_commentary() {
    let bytes = build(&[
        "HISTORY calibrated 2026-07-11",
        "COMMENT hello world",
        "OBJECT  = 'X       '",
    ]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(
        h.get_all::<String>("HISTORY"),
        vec!["calibrated 2026-07-11".to_string()]
    );
    assert_eq!(
        h.get_all::<String>("COMMENT"),
        vec!["hello world".to_string()]
    );
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("X"));
}

#[test]
fn reassembles_continue() {
    let bytes = build(&["LONGKEY = 'aaa&'", "CONTINUE  'bbb'"]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("LONGKEY").unwrap(), Some("aaabbb"));
    assert_eq!(h.count("LONGKEY"), 1);
}

#[test]
fn literal_trailing_ampersand_is_kept() {
    let bytes = build(&["TAG     = 'a&b&'"]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("TAG").unwrap(), Some("a&b&"));
}

#[test]
fn stops_at_end() {
    let mut bytes = build(&["OBJECT  = 'X       '"]);
    bytes.extend(card("BOGUS   = 'Y       '"));
    while bytes.len() % 2880 != 0 {
        bytes.push(b' ');
    }
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("X"));
    assert_eq!(h.get_str("BOGUS").unwrap(), None);
}

#[test]
fn empty_string_reads_as_none() {
    let bytes = build(&["BLANK   = '        '"]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("BLANK").unwrap(), None);
    assert_eq!(h.count("BLANK"), 1);
}

#[test]
fn unescapes_doubled_quotes() {
    let bytes = build(&["NAME    = 'O''Brien '"]);
    let h = Header::parse(&bytes).unwrap();
    assert_eq!(h.get_str("NAME").unwrap(), Some("O'Brien"));
}
