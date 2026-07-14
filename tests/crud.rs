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
