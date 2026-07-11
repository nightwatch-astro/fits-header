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
    let cont = content.ends_with('&');
    if cont {
        content.pop();
    }
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
