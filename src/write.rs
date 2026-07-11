//! Serialize a header back to bytes.

use crate::error::FitsError;
use crate::header::Header;
use crate::record::{Record, RecordKind, Value};
use crate::{BLOCK_LEN, CARD_LEN};

/// Largest data segment [`Header::to_bytes`] will zero-fill, in bytes (1 GiB).
///
/// A header can declare an arbitrarily large image; zero-filling it would allocate that many
/// bytes from header content alone. Above this cap `to_bytes` returns
/// [`FitsError::DataTooLarge`] — serialize with [`Header::to_header_bytes`] and supply the
/// data yourself.
pub const MAX_ZERO_FILL: u64 = 1 << 30;

/// Image geometry used only when [`Header::to_bytes`] must synthesize missing structural cards.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructuralHints {
    /// `BITPIX` — bits per pixel (sign encodes integer vs. float).
    pub bitpix: i64,
    /// `NAXIS1` — first axis length.
    pub naxis1: u32,
    /// `NAXIS2` — second axis length.
    pub naxis2: u32,
}

impl Default for StructuralHints {
    /// A 1×1 8-bit image.
    fn default() -> Self {
        StructuralHints {
            bitpix: 8,
            naxis1: 1,
            naxis2: 1,
        }
    }
}

/// Serialize the header block only (cards + `END`, padded to a 2880 multiple).
pub fn to_header_bytes(header: &Header) -> Vec<u8> {
    let mut out = Vec::new();
    emit_records(&mut out, header.cards(), has_longstrn(header));
    finish_header(&mut out);
    out
}

/// Serialize a standalone FITS object (header + minimal zero data block).
///
/// Errors with [`FitsError::DataTooLarge`] when the declared data segment exceeds
/// [`MAX_ZERO_FILL`].
pub fn to_bytes(header: &Header, hints: &StructuralHints) -> Result<Vec<u8>, FitsError> {
    let synth = !header.cards().iter().any(|r| r.keyword() == Some("SIMPLE"));

    let declared = data_len(header, hints, synth);
    if declared > MAX_ZERO_FILL {
        return Err(FitsError::DataTooLarge {
            declared,
            max: MAX_ZERO_FILL,
        });
    }

    let mut out = Vec::new();
    if synth {
        push(
            &mut out,
            literal_card("SIMPLE", "T", Some("conforms to FITS standard")),
        );
        push(
            &mut out,
            literal_card("BITPIX", &hints.bitpix.to_string(), None),
        );
        push(&mut out, literal_card("NAXIS", "2", None));
        push(
            &mut out,
            literal_card("NAXIS1", &hints.naxis1.to_string(), None),
        );
        push(
            &mut out,
            literal_card("NAXIS2", &hints.naxis2.to_string(), None),
        );
    }
    emit_records(&mut out, header.cards(), has_longstrn(header));
    finish_header(&mut out);

    let mut data = vec![0u8; declared as usize];
    if !data.is_empty() {
        pad_to_block(&mut data, 0);
    }
    out.extend_from_slice(&data);
    Ok(out)
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

/// The data size a header declares, in bytes (saturated on overflow, so a pathological
/// geometry reads as "too large" instead of wrapping).
fn data_len(header: &Header, hints: &StructuralHints, synth: bool) -> u64 {
    let (bitpix, axes): (i64, Vec<u64>) = if synth {
        (hints.bitpix, vec![hints.naxis1 as u64, hints.naxis2 as u64])
    } else {
        let bitpix = header.get::<i64>("BITPIX").ok().flatten().unwrap_or(8);
        let naxis = header
            .get::<i64>("NAXIS")
            .ok()
            .flatten()
            .unwrap_or(0)
            .max(0) as usize;
        let axes = (1..=naxis)
            .map(|k| {
                header
                    .get::<u64>(format!("NAXIS{k}"))
                    .ok()
                    .flatten()
                    .unwrap_or(0)
            })
            .collect();
        (bitpix, axes)
    };
    if axes.is_empty() || axes.contains(&0) {
        return 0;
    }
    let elt = bitpix.unsigned_abs() / 8;
    axes.iter().fold(elt, |acc, &n| acc.saturating_mul(n))
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
    fn data_len_geometry() {
        let hints = StructuralHints::default();
        // Synthesized: 1×1 8-bit → 1 byte.
        assert_eq!(data_len(&Header::new(), &hints, true), 1);

        let mut h = Header::new();
        h.set("SIMPLE", crate::value::Literal("T")).unwrap();
        // No NAXIS → no data.
        assert_eq!(data_len(&h, &hints, false), 0);
        // A zero axis → no data.
        h.set("BITPIX", -32).unwrap();
        h.set("NAXIS", 2).unwrap();
        h.set("NAXIS1", 100).unwrap();
        h.set("NAXIS2", 0).unwrap();
        assert_eq!(data_len(&h, &hints, false), 0);
        // Negative BITPIX (float) still sizes by magnitude: 100×50×4.
        h.set("NAXIS2", 50).unwrap();
        assert_eq!(data_len(&h, &hints, false), 100 * 50 * 4);
        // A missing NAXISk reads as 0 → no data.
        h.set("NAXIS", 3).unwrap();
        assert_eq!(data_len(&h, &hints, false), 0);
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
