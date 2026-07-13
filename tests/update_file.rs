//! `Header::update_file` byte-exactness: editing a file's header never disturbs the data
//! unit or anything after it.

mod common;
use common::build;
use fits_header::Header;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A path in the system temp dir unique to this test run and call.
fn unique_path(name: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fits-header-update-file-{name}-{}-{nanos}-{n}.fits",
        std::process::id()
    ))
}

struct TempFile(std::path::PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_fixture(name: &str, header: &[u8], data: &[u8]) -> TempFile {
    let path = unique_path(name);
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(data);
    fs::write(&path, &bytes).unwrap();
    TempFile(path)
}

#[test]
fn noop_update_reproduces_file_byte_for_byte() {
    let header = build(&["OBJECT  = 'M31     '", "EXPTIME = 120.0"]);
    let data: Vec<u8> = (0u8..=255).collect();
    let mut original = header.clone();
    original.extend_from_slice(&data);

    let f = write_fixture("noop", &header, &data);
    Header::update_file(&f.0, |_h| Ok(())).unwrap();

    assert_eq!(fs::read(&f.0).unwrap(), original);
}

#[test]
fn edit_changes_header_preserves_data_tail() {
    let header = build(&["OBJECT  = 'M31     '", "EXPTIME = 120.0"]);
    let data: Vec<u8> = (0u8..=255).collect();

    let f = write_fixture("edit", &header, &data);
    Header::update_file(&f.0, |h| {
        h.set("OBJECT", "NGC 7000")?;
        Ok(())
    })
    .unwrap();

    let after = fs::read(&f.0).unwrap();
    // No card was added/removed, so the header block size is unchanged.
    assert_eq!(after.len(), header.len() + data.len());
    assert_ne!(&after[..header.len()], &header[..], "header block changed");
    assert_eq!(
        &after[header.len()..],
        &data[..],
        "data tail preserved byte-for-byte"
    );

    let h = Header::parse(&after[..header.len()]).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("NGC 7000"));
}

#[test]
fn edit_crossing_block_boundary_grows_by_one_block_preserves_tail() {
    let header = build(&["OBJECT  = 'M31     '"]);
    assert_eq!(header.len(), 2880, "fixture fits in a single block");
    let data = vec![0x42u8; 50];

    let f = write_fixture("boundary", &header, &data);
    Header::update_file(&f.0, |h| {
        for i in 0..40 {
            h.append("HISTORY", format!("step {i}"))?;
        }
        Ok(())
    })
    .unwrap();

    let after = fs::read(&f.0).unwrap();
    // 1 OBJECT + 40 HISTORY + END = 42 cards * 80 = 3360 bytes -> 2 blocks (5760 bytes).
    let new_header_len = 5760;
    assert_eq!(after.len(), new_header_len + data.len());
    assert_eq!(
        new_header_len,
        header.len() + 2880,
        "grew by exactly one block"
    );
    assert_eq!(
        &after[new_header_len..],
        &data[..],
        "data tail preserved across the boundary crossing"
    );

    let h = Header::parse(&after[..new_header_len]).unwrap();
    assert_eq!(h.get_all::<String>("HISTORY").len(), 40);
}

#[test]
fn second_hdu_after_data_unit_stays_intact() {
    let primary_header = build(&["SIMPLE  =                    T", "OBJECT  = 'M31     '"]);
    let primary_data = vec![0x11u8; 2880];
    let second_hdu = build(&["XTENSION= 'IMAGE   '", "OBJECT  = 'DARK    '"]);
    let second_data = vec![0x22u8; 100];

    let mut rest = primary_data.clone();
    rest.extend_from_slice(&second_hdu);
    rest.extend_from_slice(&second_data);

    let f = write_fixture("second-hdu", &primary_header, &rest);
    Header::update_file(&f.0, |h| {
        h.set("OBJECT", "M31-edited")?;
        Ok(())
    })
    .unwrap();

    let after = fs::read(&f.0).unwrap();
    // The edit didn't change the primary header's card count, so its block count is stable.
    assert_eq!(
        &after[primary_header.len()..],
        &rest[..],
        "data unit and second HDU untouched"
    );

    let h = Header::parse(&after[..primary_header.len()]).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31-edited"));
}

#[test]
fn missing_end_card_errors() {
    let mut bytes = vec![b' '; 2880];
    bytes[..6].copy_from_slice(b"OBJECT");
    let f = write_fixture("missing-end", &bytes, &[]);
    assert!(matches!(
        Header::update_file(&f.0, |_h| Ok(())),
        Err(fits_header::FitsError::MissingEnd)
    ));
}
