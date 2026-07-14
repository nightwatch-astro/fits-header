// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `Header::update_file` byte-exactness: editing a file's header never disturbs the data
//! unit or anything after it.

mod common;
use common::{build, card};
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

#[test]
fn end_not_padded_to_block_errors_without_panic() {
    // A valid card + END = 160 bytes, never padded to 2880. The header region rounds up to a
    // full block, which runs past the file; this must Err, not panic on an out-of-range slice.
    let mut bytes = card("OBJECT  = 'M31     '");
    bytes.extend(card("END"));
    assert_eq!(bytes.len(), 160);
    let f = write_fixture("unpadded-end", &bytes, &[]);
    assert!(matches!(
        Header::update_file(&f.0, |_h| Ok(())),
        Err(fits_header::FitsError::TruncatedHeader)
    ));
    // File is untouched by the failed edit.
    assert_eq!(fs::read(&f.0).unwrap(), bytes);
}

#[test]
fn shrink_dropping_a_whole_block_preserves_data_on_boundary() {
    // 40 cards + END = 41 cards -> 3280 bytes -> 2 blocks (5760).
    let mut cards: Vec<String> = (0..40).map(|i| format!("HISTORY line {i}")).collect();
    cards.insert(0, "OBJECT  = 'M31     '".to_string());
    let header = {
        let mut out = Vec::new();
        for c in &cards {
            out.extend(card(c));
        }
        out.extend(card("END"));
        while out.len() % 2880 != 0 {
            out.push(b' ');
        }
        out
    };
    assert_eq!(header.len(), 5760, "fixture spans two blocks");
    let data: Vec<u8> = (0u8..=255).collect();

    let f = write_fixture("shrink-block", &header, &data);
    Header::update_file(&f.0, |h| {
        let count = h.count("HISTORY");
        for _ in 0..count {
            h.remove(("HISTORY", 0))?;
        }
        Ok(())
    })
    .unwrap();

    let after = fs::read(&f.0).unwrap();
    let new_header_len = after.len() - data.len();
    // 1 OBJECT + END = 2 cards -> one block.
    assert_eq!(new_header_len, 2880, "header shrank to a single block");
    assert_eq!(
        new_header_len % 2880,
        0,
        "data re-lands on a block boundary"
    );
    assert_eq!(
        &after[new_header_len..],
        &data[..],
        "data preserved across the whole-block shrink"
    );
    let h = Header::parse(&after[..new_header_len]).unwrap();
    assert_eq!(h.count("HISTORY"), 0);
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("M31"));
}

#[cfg(unix)]
#[test]
fn update_follows_symlink_and_preserves_mode() {
    use std::os::unix::fs::PermissionsExt;

    let header = build(&["OBJECT  = 'M31     '"]);
    let data = vec![0x55u8; 32];
    let f = write_fixture("symlink-target", &header, &data);
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o600)).unwrap();

    // A symlink beside the real file; editing through it must edit the target, not replace
    // the link with a plain file.
    let link = f.0.with_extension("link");
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(&f.0, &link).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _guard = Cleanup(link.clone());

    Header::update_file(&link, |h| {
        h.set("OBJECT", "NGC 7000")?;
        Ok(())
    })
    .unwrap();

    // The link is still a symlink pointing at the real file.
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    // The real file was edited in place and kept its 0600 mode.
    let target_meta = fs::metadata(&f.0).unwrap();
    assert_eq!(target_meta.permissions().mode() & 0o777, 0o600);
    let after = fs::read(&f.0).unwrap();
    assert_eq!(
        &after[after.len() - data.len()..],
        &data[..],
        "data preserved"
    );
    let h = Header::parse(&after[..after.len() - data.len()]).unwrap();
    assert_eq!(h.get_str("OBJECT").unwrap(), Some("NGC 7000"));
}
