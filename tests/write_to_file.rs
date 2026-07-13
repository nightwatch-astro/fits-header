//! `Header::write_to_file` — creates a new file (header block + caller data) and refuses
//! to touch a path that already exists.

mod common;
use common::build;
use fits_header::{FitsError, Header};
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
        "fits-header-write-to-file-{name}-{}-{nanos}-{n}.fits",
        std::process::id()
    ))
}

struct TempFile(std::path::PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn creates_new_file_with_header_and_data() {
    let mut header = Header::new();
    header.set("OBJECT", "M31").unwrap();
    let data: Vec<u8> = (0u8..=255).collect();

    let path = unique_path("basic");
    let f = TempFile(path.clone());
    header.write_to_file(&path, &data).unwrap();

    let expected_header = build(&["OBJECT  = 'M31     '"]);
    let on_disk = fs::read(&f.0).unwrap();
    assert_eq!(
        on_disk.len(),
        expected_header.len() + data.len(),
        "header block plus data"
    );
    assert_eq!(&on_disk[..expected_header.len()], &expected_header[..]);
    assert_eq!(&on_disk[expected_header.len()..], &data[..]);

    let parsed = Header::parse(&on_disk).unwrap();
    assert_eq!(parsed.get_str("OBJECT").unwrap(), Some("M31"));
}

#[test]
fn header_only_write_uses_empty_data() {
    let mut header = Header::new();
    header.set("OBJECT", "M31").unwrap();

    let path = unique_path("header-only");
    let f = TempFile(path.clone());
    header.write_to_file(&path, &[]).unwrap();

    let expected_header = build(&["OBJECT  = 'M31     '"]);
    let on_disk = fs::read(&f.0).unwrap();
    assert_eq!(on_disk, expected_header, "just the padded header block");
}

#[test]
fn errors_and_does_not_modify_an_existing_path() {
    let path = unique_path("existing");
    let f = TempFile(path.clone());
    let original = b"not a fits file, left untouched".to_vec();
    fs::write(&f.0, &original).unwrap();

    let mut header = Header::new();
    header.set("OBJECT", "NGC 7000").unwrap();
    let result = header.write_to_file(&path, &[1, 2, 3]);

    assert!(
        matches!(result, Err(FitsError::Io(_))),
        "expected an I/O error, got {result:?}"
    );
    assert_eq!(
        fs::read(&f.0).unwrap(),
        original,
        "existing file's contents must survive the failed write"
    );
}
