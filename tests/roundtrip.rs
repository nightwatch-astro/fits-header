// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Round-trip properties: byte-exact for untouched cards, semantic equality after parse.

use fits_header::Header;
use proptest::prelude::*;
use std::collections::HashSet;

const RESERVED: &[&str] = &[
    "END", "SIMPLE", "BITPIX", "NAXIS", "COMMENT", "HISTORY", "CONTINUE", "LONGSTRN",
];

fn keyword() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9]{0,6}".prop_filter("reserved", |k| {
        !RESERVED.contains(&k.as_str()) && !k.starts_with("NAXIS")
    })
}

fn value() -> impl Strategy<Value = String> {
    // Printable ASCII, no trailing spaces (which a FITS string value drops).
    "[ -~]{0,60}".prop_map(|s| s.trim_end().to_string())
}

fn build(entries: &[(String, String)]) -> Header {
    let mut h = Header::new();
    let mut seen = HashSet::new();
    for (k, v) in entries {
        if seen.insert(k.clone()) {
            h.set(k.as_str(), v.as_str()).unwrap();
        }
    }
    h
}

proptest! {
    #[test]
    fn semantic_roundtrip(entries in prop::collection::vec((keyword(), value()), 0..15)) {
        let h = build(&entries);
        let out = h.to_header_bytes();
        prop_assert_eq!(out.len() % 80, 0);
        prop_assert_eq!(out.len() % 2880, 0);
        let re = Header::parse(&out).unwrap();
        prop_assert_eq!(&h, &re);
    }

    #[test]
    fn byte_exact_reserialize(entries in prop::collection::vec((keyword(), value()), 0..15)) {
        let bytes = build(&entries).to_header_bytes();
        // An unmodified parsed header re-serializes to identical bytes.
        prop_assert_eq!(Header::parse(&bytes).unwrap().to_header_bytes(), bytes);
    }
}
