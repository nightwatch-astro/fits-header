//! US3 — byte-exact write-back and serialization.

mod common;
use common::build;
use fits_header::{parse, FitsError, Header, StructuralHints, MAX_ZERO_FILL};

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
    let out = h.to_bytes(&StructuralHints::default()).unwrap();
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
    let out = h.to_bytes(&StructuralHints::default()).unwrap();
    let re = parse(&out).unwrap();
    assert_eq!(re.count("SIMPLE"), 1);
    assert_eq!(re.count("BITPIX"), 1);
}

#[test]
fn to_bytes_rejects_oversized_declared_data() {
    // 100000 × 100000 × 8 bytes ≈ 80 GB — must error, not allocate.
    let bytes = build(&[
        "SIMPLE  =                    T",
        "BITPIX  =                   64",
        "NAXIS   =                    2",
        "NAXIS1  =               100000",
        "NAXIS2  =               100000",
    ]);
    let h = parse(&bytes).unwrap();
    assert!(matches!(
        h.to_bytes(&StructuralHints::default()),
        Err(FitsError::DataTooLarge { max, .. }) if max == MAX_ZERO_FILL
    ));
}

#[test]
fn to_bytes_overflowing_axes_error_not_wrap() {
    // The axis product overflows u64; saturation must report DataTooLarge, never wrap to
    // a small allocation.
    let bytes = build(&[
        "SIMPLE  =                    T",
        "BITPIX  =                   64",
        "NAXIS   =                    3",
        "NAXIS1  =  9223372036854775807",
        "NAXIS2  =  9223372036854775807",
        "NAXIS3  =  9223372036854775807",
    ]);
    let h = parse(&bytes).unwrap();
    assert!(matches!(
        h.to_bytes(&StructuralHints::default()),
        Err(FitsError::DataTooLarge { declared, .. }) if declared == u64::MAX
    ));
}

#[test]
fn to_bytes_zero_naxis_has_no_data_segment() {
    let bytes = build(&[
        "SIMPLE  =                    T",
        "BITPIX  =                    8",
        "NAXIS   =                    0",
    ]);
    let h = parse(&bytes).unwrap();
    let out = h.to_bytes(&StructuralHints::default()).unwrap();
    // Header block only: cards + END padded to one 2880 block, no zero-fill after it.
    assert_eq!(out.len(), 2880);
}

#[test]
fn untouched_continue_run_is_byte_exact() {
    let bytes = build(&[
        "LONGKEY = 'aaa&'",
        "CONTINUE  'bbb&'",
        "CONTINUE  'ccc'",
        "GAIN    = 1",
    ]);
    let h = parse(&bytes).unwrap();
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
    let mut h = parse(&bytes).unwrap();
    let long = "x".repeat(200);
    h.set("LONGKEY", long.as_str()).unwrap();
    let out = h.to_header_bytes();

    let re = parse(&out).unwrap();
    assert_eq!(re.get_str("LONGKEY").unwrap(), Some(long.as_str()));
    assert_eq!(re.count("LONGSTRN"), 1, "convention card added on re-split");
    assert_eq!(re.get::<i64>("GAIN").unwrap(), Some(1), "neighbors survive");
}

#[test]
fn raw_keyword_survives_serialization() {
    let mut h = Header::new();
    h.set_raw("obj", "x").unwrap();
    let re = parse(&h.to_header_bytes()).unwrap();
    assert_eq!(re.get_str("obj").unwrap(), Some("x"));
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
