//! Parse a FITS header unit from raw bytes.

use crate::error::FitsError;
use crate::header::Header;
use crate::record::{RecordKind, Value};
use crate::CARD_LEN;

/// Parse one FITS header unit from raw bytes.
///
/// Reads 80-byte cards in order, stops at `END`, and retains every card (including commentary,
/// `HIERARCH`, and unrecognized cards) so untouched cards serialize verbatim. `CONTINUE` runs are
/// reassembled into a single logical value.
pub fn parse(bytes: &[u8]) -> Result<Header, FitsError> {
    let cards: Vec<[u8; CARD_LEN]> = bytes
        .chunks_exact(CARD_LEN)
        .map(|c| {
            let mut a = [b' '; CARD_LEN];
            a.copy_from_slice(c);
            a
        })
        .collect();

    let mut records = Vec::new();
    let mut i = 0;
    while i < cards.len() {
        let card = cards[i];
        let card_str = String::from_utf8_lossy(&card).into_owned();
        let keyword = card_str[..8].trim().to_string();

        if keyword == "END" {
            break;
        }

        let is_value = card.get(8) == Some(&b'=') && card.get(9) == Some(&b' ');

        if is_value {
            let field = card_str[10..].trim_start();
            if field.starts_with('\'') {
                // String value — may continue across CONTINUE cards.
                let (mut content, mut comment, mut cont) = parse_string_field(field);
                let mut raw = vec![card];
                while cont && i + 1 < cards.len() {
                    let next = cards[i + 1];
                    let next_kw = String::from_utf8_lossy(&next[..8]).trim().to_string();
                    if next_kw != "CONTINUE" {
                        break;
                    }
                    // Confirmed continuation: drop the '&' marker before appending the next piece.
                    content.pop();
                    let next_str = String::from_utf8_lossy(&next).into_owned();
                    let nf = next_str[10..].trim_start();
                    let (piece, c2, cont2) = parse_string_field(nf);
                    content.push_str(&piece);
                    if c2.is_some() {
                        comment = c2;
                    }
                    cont = cont2;
                    raw.push(next);
                    i += 1;
                }
                let content = content.trim_end().to_string();
                records.push(crate::record::Record::from_raw(
                    RecordKind::Value {
                        keyword,
                        value: Value::Str(content),
                        comment,
                    },
                    raw,
                ));
            } else {
                let (token, comment) = split_comment(field);
                records.push(crate::record::Record::from_raw(
                    RecordKind::Value {
                        keyword,
                        value: Value::Literal(token.trim().to_string()),
                        comment,
                    },
                    vec![card],
                ));
            }
        } else if keyword == "COMMENT" || keyword == "HISTORY" {
            let text = card_str[8..].trim_end().to_string();
            records.push(crate::record::Record::from_raw(
                RecordKind::Commentary { keyword, text },
                vec![card],
            ));
        } else if keyword.is_empty() {
            let rest = &card_str[8..];
            if rest.trim().is_empty() {
                records.push(crate::record::Record::from_raw(
                    RecordKind::Opaque {
                        text: String::new(),
                    },
                    vec![card],
                ));
            } else {
                records.push(crate::record::Record::from_raw(
                    RecordKind::Commentary {
                        keyword,
                        text: rest.trim_end().to_string(),
                    },
                    vec![card],
                ));
            }
        } else {
            // HIERARCH / non-standard / stray CONTINUE.
            records.push(crate::record::Record::from_raw(
                RecordKind::Opaque {
                    text: card_str.trim_end().to_string(),
                },
                vec![card],
            ));
        }

        i += 1;
    }

    Ok(Header::from_records(records))
}

/// Parse a quoted string field: return the unescaped content (trailing `&` continuation marker
/// removed), any inline comment, and whether the value continues.
fn parse_string_field(field: &str) -> (String, Option<String>, bool) {
    let rest = match field.strip_prefix('\'') {
        Some(r) => r,
        None => return (String::new(), None, false),
    };
    let chars: Vec<char> = rest.chars().collect();
    let mut content = String::new();
    let mut k = 0;
    while k < chars.len() {
        if chars[k] == '\'' {
            if chars.get(k + 1) == Some(&'\'') {
                content.push('\'');
                k += 2;
                continue;
            }
            k += 1;
            break;
        }
        content.push(chars[k]);
        k += 1;
    }
    let remainder: String = chars[k..].iter().collect();
    let comment = extract_comment(&remainder);
    // `content` keeps a trailing '&'; the caller drops it only when a CONTINUE card follows.
    let cont = content.ends_with('&');
    (content, comment, cont)
}

/// A comment after a value: text following the first `/`.
fn extract_comment(s: &str) -> Option<String> {
    let idx = s.find('/')?;
    let c = s[idx + 1..].trim().to_string();
    (!c.is_empty()).then_some(c)
}

/// Split a literal field into its token and optional comment.
fn split_comment(s: &str) -> (&str, Option<String>) {
    match s.find('/') {
        Some(idx) => (&s[..idx], extract_comment(&s[idx..])),
        None => (s, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::RecordKind;

    fn block(cards: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for c in cards {
            let mut b = c.as_bytes().to_vec();
            b.resize(CARD_LEN, b' ');
            out.extend(b);
        }
        let mut end = b"END".to_vec();
        end.resize(CARD_LEN, b' ');
        out.extend(end);
        out
    }

    #[test]
    fn string_field_plain_with_comment() {
        let (content, comment, cont) = parse_string_field("'abc     ' / a note");
        assert_eq!(content, "abc     ");
        assert_eq!(comment.as_deref(), Some("a note"));
        assert!(!cont);
    }

    #[test]
    fn string_field_unescapes_doubled_quotes() {
        let (content, _, _) = parse_string_field("'ab''c'");
        assert_eq!(content, "ab'c");
    }

    #[test]
    fn string_field_continuation_marker() {
        let (content, comment, cont) = parse_string_field("'abc&'");
        // The '&' stays in the content; the caller drops it only when CONTINUE follows.
        assert_eq!(content, "abc&");
        assert_eq!(comment, None);
        assert!(cont);
    }

    #[test]
    fn string_field_without_quote_is_empty() {
        assert_eq!(parse_string_field("T"), (String::new(), None, false));
    }

    #[test]
    fn comment_extraction() {
        assert_eq!(extract_comment(" / hi"), Some("hi".to_string()));
        assert_eq!(extract_comment(" / "), None, "empty comment is None");
        assert_eq!(extract_comment("no slash"), None);
        let (token, comment) = split_comment("T / yes");
        assert_eq!(token.trim(), "T");
        assert_eq!(comment.as_deref(), Some("yes"));
        let (token, comment) = split_comment("42");
        assert_eq!(token, "42");
        assert_eq!(comment, None);
    }

    #[test]
    fn hierarch_and_unrecognized_are_opaque() {
        let h = parse(&block(&["HIERARCH ESO DET DIT = 10.0", "JUNK CARD"])).unwrap();
        assert_eq!(h.cards().len(), 2);
        for r in h.cards() {
            assert!(matches!(r.kind, RecordKind::Opaque { .. }));
            assert_eq!(r.keyword(), None);
        }
    }

    #[test]
    fn fully_blank_card_is_opaque_blank_with_text_is_commentary() {
        let h = parse(&block(&["", "        some annotation"])).unwrap();
        assert!(matches!(
            h.cards()[0].kind,
            RecordKind::Opaque { ref text } if text.is_empty()
        ));
        assert!(matches!(
            h.cards()[1].kind,
            RecordKind::Commentary { ref keyword, ref text }
                if keyword.is_empty() && text == "some annotation"
        ));
    }

    #[test]
    fn stray_continue_is_opaque() {
        // CONTINUE without a preceding '&'-terminated string is not a value card.
        let h = parse(&block(&["OBJECT  = 'X'", "CONTINUE  'orphan'"])).unwrap();
        assert_eq!(h.get_str("OBJECT").unwrap(), Some("X"));
        assert!(matches!(h.cards()[1].kind, RecordKind::Opaque { .. }));
    }

    #[test]
    fn continue_comment_comes_from_last_card() {
        let h = parse(&block(&["LONG    = 'aaa&'", "CONTINUE  'bbb' / tail note"])).unwrap();
        assert_eq!(h.get_str("LONG").unwrap(), Some("aaabbb"));
        assert_eq!(h.cards()[0].comment(), Some("tail note"));
    }

    #[test]
    fn continue_run_at_end_of_input_keeps_marker() {
        // '&' with no following CONTINUE card: marker is literal content.
        let h = parse(&block(&["LONG    = 'aaa&'"])).unwrap();
        assert_eq!(h.get_str("LONG").unwrap(), Some("aaa&"));
    }

    #[test]
    fn trailing_partial_card_is_dropped() {
        let mut bytes = block(&["OBJECT  = 'X'"]);
        bytes.extend_from_slice(b"GAIN    = 1"); // 11 bytes, not a full card
        let h = parse(&bytes).unwrap();
        assert_eq!(h.get_str("OBJECT").unwrap(), Some("X"));
        assert_eq!(h.count("GAIN"), 0);
    }

    #[test]
    fn non_utf8_bytes_parse_lossily() {
        let mut cards = block(&["OBJECT  = 'X'"]);
        cards[15] = 0xff; // inside the value field
        let h = parse(&cards).unwrap();
        assert_eq!(h.cards().len(), 1, "card is retained, lossily decoded");
    }

    #[test]
    fn value_needs_equals_space() {
        // '=' not followed by a space is not a value indicator.
        let h = parse(&block(&["WEIRD   =X"])).unwrap();
        assert!(matches!(h.cards()[0].kind, RecordKind::Opaque { .. }));
    }
}
