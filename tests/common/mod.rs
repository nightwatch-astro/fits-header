// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Shared test helpers.

/// A single 80-byte card from a string (space-padded, truncated at 80).
pub fn card(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.resize(80, b' ');
    b
}

/// Build a header block from card strings: the cards, then `END`, padded to a 2880 multiple.
pub fn build(cards: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    for c in cards {
        out.extend(card(c));
    }
    out.extend(card("END"));
    while out.len() % 2880 != 0 {
        out.push(b' ');
    }
    out
}
