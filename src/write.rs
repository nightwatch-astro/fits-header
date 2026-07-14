// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Serialize a header back to bytes.

use crate::header::Header;
use crate::record::{Record, RecordKind, Value};
use crate::{BLOCK_LEN, CARD_LEN};

/// Serialize the header block only (cards + `END`, padded to a 2880 multiple).
pub fn to_header_bytes(header: &Header) -> Vec<u8> {
    let mut out = Vec::new();
    emit_records(&mut out, header.cards(), has_longstrn(header));
    finish_header(&mut out);
    out
}

fn finish_header(out: &mut Vec<u8>) {
    push(out, pad80("END"));
    pad_to_block(out, b' ');
}

fn emit_records(out: &mut Vec<u8>, records: &[Record], longstrn_present: bool) {
    let mut longstrn_done = longstrn_present;
    for record in records {
        let (cards, used_continue) = render_record(record);
        if used_continue && !longstrn_done {
            push(out, longstrn_card());
            longstrn_done = true;
        }
        for card in cards {
            out.extend_from_slice(&card);
        }
    }
}

fn render_record(record: &Record) -> (Vec<[u8; CARD_LEN]>, bool) {
    if let Some(raw) = record.raw_cards() {
        return (raw.to_vec(), false);
    }
    match &record.kind {
        RecordKind::Value {
            keyword,
            value: Value::Str(s),
            comment,
        } => string_cards(keyword, s, comment.as_deref()),
        RecordKind::Value {
            keyword,
            value: Value::Literal(l),
            comment,
        } => (vec![literal_card(keyword, l, comment.as_deref())], false),
        RecordKind::Commentary { keyword, text } => (commentary_cards(keyword, text), false),
        RecordKind::Opaque { text } => (vec![pad80(text)], false),
    }
}

fn literal_card(keyword: &str, token: &str, comment: Option<&str>) -> [u8; CARD_LEN] {
    let mut s = format!("{keyword:<8}= ");
    if token.len() <= 20 {
        s.push_str(&format!("{token:>20}"));
    } else {
        s.push_str(token);
    }
    if let Some(c) = comment {
        s.push_str(&format!(" / {c}"));
    }
    pad80(&s)
}

fn string_cards(
    keyword: &str,
    content: &str,
    comment: Option<&str>,
) -> (Vec<[u8; CARD_LEN]>, bool) {
    if let Some(card) = string_single(keyword, content, comment) {
        return (vec![card], false);
    }
    let pieces = chunk_for_continue(content);
    let mut cards = Vec::new();
    let last_idx = pieces.len() - 1;
    for (idx, piece) in pieces.iter().enumerate() {
        let esc = piece.replace('\'', "''");
        let body = if idx == last_idx {
            format!("'{esc}'")
        } else {
            format!("'{esc}&'")
        };
        let mut s = if idx == 0 {
            format!("{keyword:<8}= {body}")
        } else {
            format!("{:<8}  {body}", "CONTINUE")
        };
        if idx == last_idx {
            if let Some(c) = comment {
                let with = format!("{s} / {c}");
                if with.len() <= CARD_LEN {
                    s = with;
                }
            }
        }
        cards.push(pad80(&s));
    }
    (cards, true)
}

fn string_single(keyword: &str, content: &str, comment: Option<&str>) -> Option<[u8; CARD_LEN]> {
    let esc = content.replace('\'', "''");
    let inner = format!("{esc:<8}");
    let mut s = format!("{keyword:<8}= '{inner}'");
    if let Some(c) = comment {
        s.push_str(&format!(" / {c}"));
    }
    (s.len() <= CARD_LEN).then(|| pad80(&s))
}

/// Split content so each escaped piece fits a continuation card's value field.
fn chunk_for_continue(content: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut cur = String::new();
    let mut esc_len = 0usize;
    for ch in content.chars() {
        let add = if ch == '\'' { 2 } else { 1 };
        if esc_len + add > 66 {
            pieces.push(std::mem::take(&mut cur));
            esc_len = 0;
        }
        cur.push(ch);
        esc_len += add;
    }
    pieces.push(cur);
    pieces
}

fn commentary_cards(keyword: &str, text: &str) -> Vec<[u8; CARD_LEN]> {
    if text.is_empty() {
        return vec![pad80(&format!("{keyword:<8}"))];
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(72)
        .map(|chunk| {
            let t: String = chunk.iter().collect();
            pad80(&format!("{keyword:<8}{t}"))
        })
        .collect()
}

fn longstrn_card() -> [u8; CARD_LEN] {
    pad80(&format!(
        "{:<8}= '{:<8}' / {}",
        "LONGSTRN", "OGIP 1.0", "The OGIP long string convention may be used."
    ))
}

fn has_longstrn(header: &Header) -> bool {
    header
        .cards()
        .iter()
        .any(|r| r.keyword() == Some("LONGSTRN"))
}

fn push(out: &mut Vec<u8>, card: [u8; CARD_LEN]) {
    out.extend_from_slice(&card);
}

fn pad_to_block(out: &mut Vec<u8>, fill: u8) {
    while out.len() % BLOCK_LEN != 0 {
        out.push(fill);
    }
}

/// Left-justify `s` into an 80-byte card, space-padded (truncated at 80 bytes).
fn pad80(s: &str) -> [u8; CARD_LEN] {
    let mut card = [b' '; CARD_LEN];
    let bytes = s.as_bytes();
    let n = bytes.len().min(CARD_LEN);
    card[..n].copy_from_slice(&bytes[..n]);
    card
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(card: &[u8; CARD_LEN]) -> String {
        String::from_utf8_lossy(card).into_owned()
    }

    #[test]
    fn pad80_pads_and_truncates() {
        assert_eq!(&pad80("END")[..3], b"END");
        assert!(pad80("END")[3..].iter().all(|&b| b == b' '));
        let long = "x".repeat(100);
        assert_eq!(pad80(&long), [b'x'; CARD_LEN]);
    }

    #[test]
    fn literal_card_right_justifies_short_tokens() {
        let c = text(&literal_card("BITPIX", "8", Some("bits")));
        assert!(c.starts_with("BITPIX  =                    8 / bits"));
        // A token wider than the fixed 20-char field is emitted unpadded.
        let wide = "1234567890123456789012345";
        let c = text(&literal_card("K", wide, None));
        assert!(c.starts_with(&format!("K       = {wide}")));
    }

    #[test]
    fn string_single_pads_content_to_eight() {
        let c = text(&string_single("OBJECT", "M31", None).unwrap());
        assert!(c.starts_with("OBJECT  = 'M31     '"));
    }

    #[test]
    fn string_single_rejects_overflow() {
        // 69 escaped chars + quotes + "KEY     = " no longer fit in 80.
        assert!(string_single("KEY", &"x".repeat(68), None).is_some());
        assert!(string_single("KEY", &"x".repeat(69), None).is_none());
    }

    #[test]
    fn string_single_drops_to_continue_when_comment_overflows() {
        let content = "x".repeat(60);
        assert!(string_single("KEY", &content, Some(&"c".repeat(20))).is_none());
    }

    #[test]
    fn chunks_fit_continuation_cards_and_split_escapes() {
        // Doubled quotes count as two; every escaped piece must fit 66 columns.
        let content = "'".repeat(50) + &"a".repeat(50);
        for piece in chunk_for_continue(&content) {
            let esc = piece.replace('\'', "''");
            assert!(esc.len() <= 66, "escaped piece too long: {}", esc.len());
        }
        assert_eq!(chunk_for_continue(&content).concat(), content);
    }

    #[test]
    fn commentary_chunks_at_72() {
        let cards = commentary_cards("COMMENT", &"y".repeat(100));
        assert_eq!(cards.len(), 2);
        assert!(text(&cards[0]).starts_with(&format!("COMMENT {}", "y".repeat(72))));
        assert!(text(&cards[1]).starts_with(&format!("COMMENT {}", "y".repeat(28))));
        // Empty commentary still emits its bare keyword card.
        assert_eq!(commentary_cards("HISTORY", "").len(), 1);
    }

    #[test]
    fn longstrn_emitted_once_before_first_continue() {
        let mut h = Header::new();
        h.set("A", "x".repeat(100).as_str()).unwrap();
        h.set("B", "y".repeat(100).as_str()).unwrap();
        let out = to_header_bytes(&h);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(s.matches("LONGSTRN").count(), 1);
        // LONGSTRN precedes the first long-string card.
        assert!(s.find("LONGSTRN").unwrap() < s.find('A').unwrap_or(usize::MAX));
    }

    #[test]
    fn existing_longstrn_not_duplicated() {
        let mut h = Header::new();
        h.set("LONGSTRN", "OGIP 1.0").unwrap();
        h.set("A", "x".repeat(100).as_str()).unwrap();
        let out = to_header_bytes(&h);
        let s = String::from_utf8_lossy(&out);
        assert_eq!(s.matches("LONGSTRN").count(), 1);
    }
}
