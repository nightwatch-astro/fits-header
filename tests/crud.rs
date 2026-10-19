// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! US2 — strict CRUD.

use fits_header::{FitsError, Header};

#[test]
fn set_updates_and_appends() {
    let mut h = Header::new();
    h.set("GAIN", 100).unwrap();
    h.set("GAIN", 120).unwrap();
    assert_eq!(h.get::<i64>("GAIN").unwrap(), Some(120));
    assert_eq!(h.count("GAIN"), 1);

    h.set("FILTER", "Ha").unwrap();
    assert_eq!(h.get_str("FILTER").unwrap(), Some("Ha"));
}

#[test]
fn ambiguity_and_occurrence() {
    let mut h = Header::new();
    h.append("GAIN", 100).unwrap();
    h.append("GAIN", 200).unwrap();

    assert!(matches!(
        h.get::<i64>("GAIN"),
        Err(FitsError::AmbiguousKeyword { count: 2, .. })
    ));
    assert!(matches!(
        h.set("GAIN", 5),
        Err(FitsError::AmbiguousKeyword { .. })
    ));
    assert!(matches!(
        h.remove("GAIN"),
        Err(FitsError::AmbiguousKeyword { .. })
    ));

    assert_eq!(h.get::<i64>(("GAIN", 1)).unwrap(), Some(200));
    h.set(("GAIN", 1), 250).unwrap();
    assert_eq!(h.get::<i64>(("GAIN", 1)).unwrap(), Some(250));
    assert_eq!(h.get::<i64>(("GAIN", 0)).unwrap(), Some(100));

    assert!(h.remove(("GAIN", 0)).unwrap());
    assert_eq!(h.count("GAIN"), 1);
}

#[test]
fn history_append_and_get_all() {
    let mut h = Header::new();
    h.append("HISTORY", "one").unwrap();
    h.append("HISTORY", "two").unwrap();
    assert_eq!(
        h.get_all::<String>("HISTORY"),
        vec!["one".to_string(), "two".to_string()]
    );
    assert_eq!(h.count("HISTORY"), 2);
}

#[test]
fn atomic_batch_rejection() {
    let mut h = Header::new();
    h.set("A", 1).unwrap();
    let before = h.clone();

    let r = h.set_many([("B", 2), ("TOOLONGKEY", 3)]);
    assert!(matches!(r, Err(FitsError::KeywordTooLong { .. })));
    assert_eq!(h, before, "batch must be all-or-nothing");

    h.set_many([("B", 2), ("C", 3)]).unwrap();
    assert_eq!(h.get::<i64>("B").unwrap(), Some(2));
    assert_eq!(h.get::<i64>("C").unwrap(), Some(3));
}

#[test]
fn lowercase_rejected_but_raw_allowed() {
    let mut h = Header::new();
    assert!(matches!(
        h.set("obj", "x"),
        Err(FitsError::InvalidKeyword { .. })
    ));
    h.set_raw("obj", "x").unwrap();
    assert_eq!(h.get_str("obj").unwrap(), Some("x"));
}

#[test]
fn occurrence_out_of_range() {
    let mut h = Header::new();
    h.set("A", 1).unwrap();
    assert!(matches!(
        h.set(("A", 3), 9),
        Err(FitsError::OccurrenceOutOfRange { .. })
    ));
}

#[test]
fn set_comment_and_bool() {
    let mut h = Header::new();
    h.set("EXTEND", true).unwrap();
    assert_eq!(h.get::<bool>("EXTEND").unwrap(), Some(true));
    h.set_comment("EXTEND", "may contain extensions").unwrap();
    assert_eq!(h.cards()[0].comment(), Some("may contain extensions"));
}

#[test]
fn fits_error_is_a_std_error() {
    fn assert_traits<T: std::error::Error + Send + Sync + Clone + 'static>() {}
    assert_traits::<FitsError>();
    let e = FitsError::AmbiguousKeyword {
        keyword: "GAIN".to_string(),
        count: 2,
    };
    assert_eq!(
        e.to_string(),
        "keyword 'GAIN' occurs 2 times; select an occurrence"
    );
}
#[test]
fn hierarch_values_support_strict_crud_and_preserve_raw_cards() {
    use fits_header::Key;

    let mut bytes = Vec::new();
    for card in [
        "HIERARCH ESO DET DIT =               10.0 / integration time",
        "HIERARCH INSTRUMENT = 'Camera A' / detector",
        "UNKNOWN CARD",
    ] {
        let mut raw = card.as_bytes().to_vec();
        raw.resize(80, b' ');
        bytes.extend(raw);
    }
    let mut end = b"END".to_vec();
    end.resize(80, b' ');
    bytes.extend(end);

    let original = Header::parse(&bytes).unwrap();
    assert_eq!(original.cards()[0].keyword(), Some("ESO DET DIT"));
    assert_eq!(original.count("ESO DET DIT"), 0);
    assert_eq!(
        original.get_all::<String>("ESO DET DIT"),
        Vec::<String>::new()
    );
    assert_eq!(
        original.get::<f64>(Key::hierarch("ESO DET DIT")).unwrap(),
        Some(10.0)
    );
    assert_eq!(
        original.get_str(Key::hierarch("INSTRUMENT")).unwrap(),
        Some("Camera A")
    );
    assert_eq!(original.to_header_bytes()[..bytes.len()], bytes);

    let mut header = original;
    header.set(Key::hierarch("INSTRUMENT"), "Camera B").unwrap();
    assert_eq!(
        header.get_str(Key::hierarch("INSTRUMENT")).unwrap(),
        Some("Camera B")
    );
    assert_eq!(header.cards()[1].keyword(), Some("INSTRUMENT"));

    let updated = header.to_header_bytes();
    assert_eq!(
        &updated[80..160],
        format!("{:<80}", "HIERARCH INSTRUMENT = 'Camera B' / detector").as_bytes(),
        "edited value retains HIERARCH card layout and comment"
    );
    assert_eq!(
        &updated[..80],
        &bytes[..80],
        "untouched ESO card stays verbatim"
    );
    assert_eq!(
        &updated[160..240],
        &bytes[160..240],
        "opaque card stays verbatim"
    );
    assert!(header.remove(Key::hierarch("INSTRUMENT")).unwrap());
    assert_eq!(header.get_str(Key::hierarch("INSTRUMENT")).unwrap(), None);

    header.set(Key::hierarch("TELESCOPE"), "Scope").unwrap();
    assert_eq!(
        header.get_str(Key::hierarch("TELESCOPE")).unwrap(),
        Some("Scope")
    );
}

#[test]
fn hierarch_string_values_trim_fits_field_padding() {
    let mut bytes = Vec::new();
    for card in ["HIERARCH INSTRUMENT = 'ABC     '", "END"] {
        let mut raw = card.as_bytes().to_vec();
        raw.resize(80, b' ');
        bytes.extend(raw);
    }
    let header = Header::parse(&bytes).unwrap();
    assert_eq!(
        header
            .get_str(fits_header::Key::hierarch("INSTRUMENT"))
            .unwrap(),
        Some("ABC")
    );
}

#[test]
fn hierarch_value_must_fit_one_card_and_duplicate_names_are_ambiguous() {
    use fits_header::Key;

    let mut header = Header::new();
    assert!(matches!(
        header.set(Key::hierarch("LONG VALUE"), "x".repeat(80)),
        Err(FitsError::HierarchCardTooLong { .. })
    ));
    assert_eq!(
        header.count("LONG VALUE"),
        0,
        "rejected value was not appended"
    );

    let mut bytes = Vec::new();
    for card in ["HIERARCH ESO DET DIT = 10.0", "HIERARCH ESO DET DIT = 11.0"] {
        let mut raw = card.as_bytes().to_vec();
        raw.resize(80, b' ');
        bytes.extend(raw);
    }
    let mut end = b"END".to_vec();
    end.resize(80, b' ');
    bytes.extend(end);
    header = Header::parse(&bytes).unwrap();
    assert!(matches!(
        header.get::<f64>(Key::hierarch("ESO DET DIT")),
        Err(FitsError::AmbiguousKeyword { count: 2, .. })
    ));
    assert_eq!(
        header
            .get::<f64>(Key::hierarch_occurrence("ESO DET DIT", 1))
            .unwrap(),
        Some(11.0)
    );
    assert!(matches!(
        header.remove(Key::hierarch("ESO DET DIT")),
        Err(FitsError::AmbiguousKeyword { count: 2, .. })
    ));
}

#[test]
fn hierarch_set_replaces_cards_with_continuation_ampersands_or_repeated_spaces() {
    use fits_header::Key;

    for (card, name) in [
        ("HIERARCH ESO OBS NAME = 'abc&'", "ESO OBS NAME"),
        ("HIERARCH ESO  DET NDIT = 5", "ESO DET NDIT"),
    ] {
        let mut bytes = Vec::new();
        for card in [card, "END"] {
            let mut raw = card.as_bytes().to_vec();
            raw.resize(80, b' ');
            bytes.extend(raw);
        }
        let mut header = Header::parse(&bytes).unwrap();
        if name == "ESO OBS NAME" {
            assert_eq!(header.get_str(Key::hierarch(name)).unwrap(), Some("abc&"));
        } else {
            assert_eq!(header.get::<i64>(Key::hierarch(name)).unwrap(), Some(5));
        }
        let update_key = if name == "ESO OBS NAME" {
            Key::hierarch(name)
        } else {
            Key::hierarch("ESO DET NDIT")
        };
        header.set(update_key, "x").unwrap();
        let expected_name = if name == "ESO OBS NAME" {
            "ESO OBS NAME"
        } else {
            "ESO DET NDIT"
        };
        assert_eq!(
            header.cards().iter().filter(|r| matches!(&r.kind, fits_header::RecordKind::Hierarch { keyword, .. } if keyword == expected_name)).count(),
            1,
            "replaces {card}"
        );
        assert_eq!(
            header.get_str(Key::hierarch(expected_name)).unwrap(),
            Some("x")
        );
    }
}

#[test]
fn hierarch_names_follow_eso_keyword_charset() {
    use fits_header::Key;

    let mut header = Header::new();
    for invalid in ["telescope", "A=B", "ESO / DET", "ESO 'DET'"] {
        assert!(matches!(
            header.set(Key::hierarch(invalid), "value"),
            Err(FitsError::InvalidKeyword { .. })
        ));
    }
    header.set(Key::hierarch("TELESCOPE"), "Scope").unwrap();
    assert_eq!(
        header.cards().iter().filter(|r| matches!(&r.kind, fits_header::RecordKind::Hierarch { keyword, .. } if keyword == "TELESCOPE")).count(),
        1
    );
}

#[test]
fn standard_and_hierarch_keyword_names_have_separate_counts_and_get_all() {
    use fits_header::Key;

    let mut header = Header::new();
    header.set("TELESCOP", "standard").unwrap();
    header
        .set(Key::hierarch("TELESCOP"), "hierarchical")
        .unwrap();
    assert_eq!(header.count("TELESCOP"), 1);
    assert_eq!(header.get_all::<String>("TELESCOP"), ["standard"]);
    assert_eq!(
        header.get_str(Key::hierarch("TELESCOP")).unwrap(),
        Some("hierarchical")
    );
}

#[test]
fn malformed_hierarch_card_without_equals_stays_opaque() {
    let mut bytes = Vec::new();
    for card in ["HIERARCH ESO DET DIT 10.0 / note x=y", "END"] {
        let mut raw = card.as_bytes().to_vec();
        raw.resize(80, b' ');
        bytes.extend(raw);
    }
    let header = Header::parse(&bytes).unwrap();
    assert_eq!(header.cards().len(), 1);
    assert!(matches!(
        header.cards()[0].kind,
        fits_header::RecordKind::Opaque { .. }
    ));
    assert_eq!(&header.to_header_bytes()[..bytes.len()], &bytes);
}

#[test]
fn hierarchical_value_with_a_following_continue_is_read_as_one_hierarch_value() {
    let mut bytes = Vec::new();
    for card in ["HIERARCH ESO DET NAME = 'bad&'", "CONTINUE  'more'", "END"] {
        let mut raw = card.as_bytes().to_vec();
        raw.resize(80, b' ');
        bytes.extend(raw);
    }
    let header = Header::parse(&bytes).unwrap();
    assert_eq!(header.cards().len(), 1);
    assert!(matches!(
        header.cards()[0].kind,
        fits_header::RecordKind::Hierarch { .. }
    ));
    assert_eq!(&header.to_header_bytes()[..bytes.len()], &bytes);
}
