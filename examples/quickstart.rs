//! End-to-end walkthrough: parse a header, read and mutate it, then the two real file
//! flows this crate supports — creating a new FITS object and editing an existing one
//! in place.
//!
//! Run with `cargo run --example quickstart`. Every section here also appears in
//! `docs/guide.md`, so the two stay in sync.

use fits_header::{FitsError, Header};

/// The header used throughout this walkthrough: a CCD image of M31.
///
/// One string per 80-byte card, space-padded, in appearance order.
const SAMPLE_CARDS: &[&str] = &[
    "SIMPLE  =                    T / conforms to FITS standard",
    "BITPIX  =                  -32 / IEEE single-precision float",
    "NAXIS   =                    2 / number of data axes",
    "NAXIS1  =                 1024 / axis 1 length",
    "NAXIS2  =                 1024 / axis 2 length",
    "OBJECT  = 'M31     '           / target name",
    "EXPTIME =                120.0 / exposure time in seconds",
    "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
    "GAIN    =                  1.0 / e-/ADU",
    "FILTER  = 'Ha      '           / filter name",
    "TELESCOP= 'EdgeHD 8'           / telescope",
    "HISTORY dark subtracted",
];

/// Pack [`SAMPLE_CARDS`] into a valid header unit: 80-byte cards, `END`, padded to a
/// [`fits_header::BLOCK_LEN`] multiple.
fn sample_header_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
        let mut c = card.as_bytes().to_vec();
        c.resize(fits_header::CARD_LEN, b' ');
        bytes.extend(c);
    }
    while bytes.len() % fits_header::BLOCK_LEN != 0 {
        bytes.push(b' ');
    }
    bytes
}

fn main() -> Result<(), FitsError> {
    // Parse every card. Cards left untouched below re-serialize byte-for-byte.
    let mut header: Header = Header::parse(&sample_header_bytes()).unwrap();

    // Typed reads through one generic accessor.
    let object: Option<&str> = header.get_str("OBJECT")?;
    let exptime: Option<f64> = header.get("EXPTIME")?;
    assert_eq!(object, Some("M31"));
    assert_eq!(exptime, Some(120.0));

    // Commentary keywords (COMMENT/HISTORY/blank) repeat; read every occurrence back.
    assert_eq!(header.count("HISTORY"), 1);
    assert_eq!(
        header.get_all::<String>("HISTORY"),
        vec!["dark subtracted".to_string()]
    );

    // Create, update, delete.
    header.set("OBJECT", "NGC 7000")?; // updates in place
    header.append("HISTORY", "flat fielded")?; // HISTORY repeats, so this adds a second card
    header.set_comment("EXPTIME", "seconds, revised")?;
    header.remove("GAIN")?;

    // Batch mutations are atomic: every entry validates before any of them applies.
    header.set_many([("FILTER", "OIII"), ("TELESCOP", "EdgeHD 11")])?;

    // Serialize the header alone — cards plus END, padded to a block multiple. BITPIX,
    // NAXIS*, and DATE-OBS were never touched, so they come back byte-for-byte identical
    // to the input.
    let block: Vec<u8> = header.to_header_bytes();
    assert_eq!(block.len() % fits_header::BLOCK_LEN, 0);

    // --- Create a new file from scratch ---
    // This crate is header-only: it never fabricates pixel data. Append your own bytes
    // after the header block.
    let pixel_data = vec![0u8; 1024]; // caller-owned data, e.g. from an image buffer
    let mut file_bytes = header.to_header_bytes();
    file_bytes.extend_from_slice(&pixel_data);
    let path = std::env::temp_dir().join("fits-header-quickstart.fits");
    std::fs::write(&path, &file_bytes).expect("write new file");

    // --- Edit an existing file in place ---
    // update_file reads the file, hands you the header to mutate, and splices the new
    // header back onto the original data unit (and any later HDUs), untouched.
    Header::update_file(&path, |h| {
        h.set("TELESCOP", "EdgeHD 14")?;
        Ok(())
    })
    .expect("update file in place");

    let roundtrip = std::fs::read(&path).expect("read back");
    let tail = &roundtrip[roundtrip.len() - pixel_data.len()..];
    assert_eq!(tail, pixel_data, "data survives the header edit");

    let _ = std::fs::remove_file(&path);

    println!(
        "wrote {} header bytes; created and updated {}",
        block.len(),
        path.display()
    );
    Ok(())
}
