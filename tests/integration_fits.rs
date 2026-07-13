//! Integration tests against genuine, standards-valid FITS files (real primary HDU with a
//! sized data array, openable by any FITS reader) rather than minimal byte fixtures.
//!
//! Covers three header-size regimes — a header that fits one 2880-byte block, one that
//! spans two to three, and one built from hundreds of cards spanning many — and checks
//! that reading, a no-op `update_file`, a same-length edit, a block-crossing growth edit,
//! a shrinking edit, and a second HDU all leave the data unit byte-identical.

use fits_header::{Header, RecordKind};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

const BLOCK_LEN: usize = fits_header::BLOCK_LEN;
const CARD_LEN: usize = fits_header::CARD_LEN;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_path(name: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fits-header-integration-{name}-{}-{nanos}-{n}.fits",
        std::process::id()
    ))
}

struct TempFile(std::path::PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// One 80-byte card, space-padded/truncated.
fn card(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.resize(CARD_LEN, b' ');
    b
}

/// A header block from card strings: the cards, `END`, space-padded to a 2880 multiple.
fn header_block(cards: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for c in cards {
        out.extend(card(c));
    }
    out.extend(card("END"));
    while out.len() % BLOCK_LEN != 0 {
        out.push(b' ');
    }
    out
}

fn literal(keyword: &str, value: &str, comment: &str) -> String {
    format!("{keyword:<8}= {value:>20} / {comment}")
}

fn string_val(keyword: &str, value: &str, comment: &str) -> String {
    format!("{keyword:<8}= '{value:<8}' / {comment}")
}

/// A deterministic, non-constant byte pattern so preservation is easy to assert exactly.
fn ramp(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 256) as u8).collect()
}

fn pad_block(mut bytes: Vec<u8>, fill: u8) -> Vec<u8> {
    while bytes.len() % BLOCK_LEN != 0 {
        bytes.push(fill);
    }
    bytes
}

/// Realistic card content shared by every fixture, plus `filler` extra `HISTORY` cards to
/// push the header into a chosen size regime.
fn realistic_cards(naxis1: usize, naxis2: usize, filler: usize) -> Vec<String> {
    let mut cards = vec![
        literal("SIMPLE", "T", "conforms to FITS standard"),
        literal("BITPIX", "8", "unsigned 8-bit integer"),
        literal("NAXIS", "2", "number of data axes"),
        literal("NAXIS1", &naxis1.to_string(), "axis 1 length"),
        literal("NAXIS2", &naxis2.to_string(), "axis 2 length"),
        string_val("OBJECT", "M31", "target name"),
        string_val("DATE-OBS", "2026-07-11T22:15:03", "UTC start of exposure"),
        literal("EXPTIME", "120.0", "exposure time in seconds"),
        "HIERARCH ESO DET DIT =               10.0 / integration time".to_string(),
        "COMMENT calibration frame".to_string(),
        "HISTORY dark subtracted".to_string(),
    ];
    for i in 0..filler {
        cards.push(format!("HISTORY filler card {i}"));
    }
    cards
}

/// A standards-valid primary HDU: `SIMPLE`/`BITPIX`/`NAXIS`/`NAXIS1`/`NAXIS2` sized to the
/// data, realistic content cards, `END`, 2880-padded header; then a byte-ramp data array,
/// zero-padded to a 2880 multiple. Returns `(file_bytes, data_bytes, header_len)`.
fn mk_fits(naxis1: usize, naxis2: usize, filler: usize) -> (Vec<u8>, Vec<u8>, usize) {
    let cards = realistic_cards(naxis1, naxis2, filler);
    let header = header_block(&cards);
    let data = pad_block(ramp(naxis1 * naxis2), 0);
    let mut file = header.clone();
    file.extend_from_slice(&data);
    (file, data, header.len())
}

fn write_fixture(name: &str, bytes: &[u8]) -> TempFile {
    let path = unique_path(name);
    fs::write(&path, bytes).unwrap();
    TempFile(path)
}

/// Runs the full assertion suite (read, no-op, same-length edit, growth, shrink) against
/// one header-size regime.
fn verify_regime(name: &str, naxis1: usize, naxis2: usize, filler: usize) {
    let (file, data, header_len) = mk_fits(naxis1, naxis2, filler);
    assert_eq!(header_len % BLOCK_LEN, 0, "header is block-aligned");
    assert_eq!(file.len(), header_len + data.len());

    // 1. Parse / read_from_file reads the header correctly.
    let f = write_fixture(name, &file);
    let h = Header::read_from_file(&f.0).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31"));
    assert_eq!(h.get_str("DATE-OBS").unwrap(), Some("2026-07-11T22:15:03"));
    assert_eq!(h.get::<f64>("EXPTIME").unwrap(), Some(120.0));
    assert_eq!(h.get::<i64>("NAXIS1").unwrap(), Some(naxis1 as i64));
    assert_eq!(h.get::<i64>("NAXIS2").unwrap(), Some(naxis2 as i64));
    assert_eq!(
        h.get_all::<String>("HISTORY").len(),
        1 + filler,
        "the realistic HISTORY card plus every filler card"
    );
    let hierarch_preserved = h.cards().iter().any(|r| {
        matches!(&r.kind, RecordKind::Opaque { text } if text.starts_with("HIERARCH ESO DET DIT"))
    });
    assert!(hierarch_preserved, "HIERARCH card preserved as opaque");
    assert_eq!(
        h.count("HIERARCH"),
        0,
        "opaque HIERARCH card is not addressable by keyword"
    );

    // 2. No-op update_file reproduces the file byte-for-byte.
    Header::update_file(&f.0, |_h| Ok(())).unwrap();
    assert_eq!(fs::read(&f.0).unwrap(), file, "no-op round-trip is exact");

    // 3. A same-length keyword edit changes the header; the data unit stays byte-identical.
    Header::update_file(&f.0, |h| {
        h.set("OBJECT", "NGC 7000")?;
        Ok(())
    })
    .unwrap();
    let after_edit = fs::read(&f.0).unwrap();
    assert_eq!(
        after_edit.len(),
        header_len + data.len(),
        "card count unchanged, so header size is unchanged"
    );
    assert_eq!(
        &after_edit[header_len..],
        &data[..],
        "data unit byte-identical after a same-length edit"
    );
    let reread = Header::parse(&after_edit[..header_len]).unwrap();
    assert_eq!(reread.get_str("OBJECT").unwrap(), Some("NGC 7000"));

    // 4. An edit that grows the header across a block boundary preserves the data.
    Header::update_file(&f.0, |h| {
        for i in 0..80 {
            h.append("HISTORY", format!("growth card {i}"))?;
        }
        Ok(())
    })
    .unwrap();
    let after_growth = fs::read(&f.0).unwrap();
    let grown_header_len = after_growth.len() - data.len();
    assert_eq!(
        grown_header_len % BLOCK_LEN,
        0,
        "data unit starts on a block boundary after growth"
    );
    assert!(
        grown_header_len > header_len,
        "the header grew from the added cards"
    );
    assert_eq!(
        (grown_header_len - header_len) % BLOCK_LEN,
        0,
        "grew by whole blocks"
    );
    assert_eq!(
        &after_growth[grown_header_len..],
        &data[..],
        "data unit preserved across the growth edit"
    );
    let after_growth_header = Header::parse(&after_growth[..grown_header_len]).unwrap();
    assert_eq!(
        after_growth_header.get_all::<String>("HISTORY").len(),
        1 + filler + 80
    );

    // 5. An edit that shrinks the header (removing cards) likewise preserves the data.
    Header::update_file(&f.0, |h| {
        let count = h.count("HISTORY");
        for _ in 0..count {
            h.remove(("HISTORY", 0))?;
        }
        Ok(())
    })
    .unwrap();
    let after_shrink = fs::read(&f.0).unwrap();
    let shrunk_header_len = after_shrink.len() - data.len();
    assert_eq!(
        shrunk_header_len % BLOCK_LEN,
        0,
        "data unit starts on a block boundary after shrinking"
    );
    assert!(
        shrunk_header_len < grown_header_len,
        "the header shrank once the HISTORY cards were removed"
    );
    assert_eq!(
        &after_shrink[shrunk_header_len..],
        &data[..],
        "data unit preserved across the shrinking edit"
    );
    let after_shrink_header = Header::parse(&after_shrink[..shrunk_header_len]).unwrap();
    assert_eq!(after_shrink_header.count("HISTORY"), 0);
    assert_eq!(
        after_shrink_header.get_str("OBJECT").unwrap(),
        Some("NGC 7000")
    );
}

#[test]
fn small_header_fits_one_block() {
    // 11 realistic cards + END = 12 cards, well under the 36 that fit one block.
    verify_regime("small", 10, 10, 0);
}

#[test]
fn normal_header_spans_two_to_three_blocks() {
    // 11 + 40 filler + END = 52 cards -> 4160 bytes -> 2 blocks.
    verify_regime("normal", 15, 15, 40);
}

#[test]
fn oversized_header_spans_many_blocks() {
    // 11 + 500 filler + END = 512 cards -> 40960 bytes -> 15 blocks.
    verify_regime("oversized", 8, 8, 500);
}

#[test]
fn second_hdu_after_primary_data_unit_is_preserved() {
    let (primary_file, primary_data, primary_header_len) = mk_fits(12, 12, 5);

    let second_hdu_cards = vec![
        string_val("XTENSION", "IMAGE", "extension type"),
        string_val("OBJECT", "DARK", "calibration frame"),
        literal("BITPIX", "8", "unsigned 8-bit integer"),
        literal("NAXIS", "2", "number of data axes"),
        literal("NAXIS1", "4", "axis 1 length"),
        literal("NAXIS2", "4", "axis 2 length"),
    ];
    let second_header = header_block(&second_hdu_cards);
    let second_data = pad_block(ramp(16), 0xAA);

    let mut file = primary_file.clone();
    file.extend_from_slice(&second_header);
    file.extend_from_slice(&second_data);

    let f = write_fixture("second-hdu", &file);

    Header::update_file(&f.0, |h| {
        h.set("OBJECT", "M31-edited")?;
        Ok(())
    })
    .unwrap();

    let after = fs::read(&f.0).unwrap();
    let mut expected_tail = primary_data.clone();
    expected_tail.extend_from_slice(&second_header);
    expected_tail.extend_from_slice(&second_data);

    assert_eq!(
        &after[primary_header_len..],
        &expected_tail[..],
        "primary data unit and second HDU (header + data) untouched"
    );

    let h = Header::parse(&after[..primary_header_len]).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31-edited"));
}
