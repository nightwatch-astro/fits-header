# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- **Pure Rust, MSVC-safe** — no C or system libraries; minimal dependency footprint.
- **Faithful and byte-exact** — every card is retained on parse; untouched cards
  (including long-string runs) re-serialize byte-for-byte, and only created or edited
  cards are re-rendered.
- **Strict keyword access** — a bare name addresses the sole occurrence of a keyword and
  errors when it is duplicated; `("NAME", n)` selects the n-th occurrence explicitly.
- **Full CRUD** — create, read, update, and delete single or multiple keywords; batch
  mutations are atomic (all or nothing).
- **Typed reads and writes** — one generic accessor, `get::<T>`, covering strings,
  numbers, booleans, and date/times; `Literal`/`Fixed`/`Sci` wrappers control number
  formatting on write.
- **Long strings** — the `CONTINUE` convention on read and write (with `LONGSTRN`).
- **Generic, not domain-specific** — an ordered header of `(keyword, value, comment)`
  cards; no application types in the API.

## Usage

```rust
use fits_header::{parse, FitsError, StructuralHints};

fn demo(bytes: &[u8]) -> Result<(), FitsError> {
    // Read every card from a FITS header unit.
    let mut header = parse(bytes)?;

    // Typed reads through one generic accessor. Access is strict: a bare
    // name errors if the keyword occurs more than once.
    let exptime: Option<f64> = header.get("EXPTIME")?;
    let object: Option<&str> = header.get_str("OBJECT")?;
    let gain: Option<i64> = header.get(("GAIN", 1))?; // second occurrence

    // Create, update, delete.
    header.set("OBJECT", "M31")?;
    header.set("EXPTIME", 300.0)?;
    header.remove("AIRMASS")?;

    // Batch mutations are atomic — all or nothing.
    header.set_many([("FILTER", "Ha"), ("TELESCOP", "EdgeHD 8")])?;

    // Serialize. Untouched cards come back byte-for-byte identical.
    let block: Vec<u8> = header.to_header_bytes();
    let whole_file = header.to_bytes(&StructuralHints::default())?;
    Ok(())
}
```

## Serialization outputs

- `to_header_bytes()` — the header block only (cards + `END`, padded to a 2880-byte
  multiple). The primary path when editing a real file: splice it onto the file's data.
- `to_bytes(&hints)` — a standalone FITS object. Missing `SIMPLE`/`BITPIX`/`NAXIS*`
  cards are synthesized from the hints, and the declared data segment is zero-filled.
  Data larger than `MAX_ZERO_FILL` (1 GiB) returns `FitsError::DataTooLarge` instead of
  allocating.

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on `Header`, `Record`,
  `Value`, and `StructuralHints`.
- `coords` *(off by default)* — astronomy helpers: sexagesimal RA/Dec parsing and
  formatting, MJD ↔ calendar conversion. No extra dependencies.

## Development

```sh
just verify                  # fmt-check + clippy (-D warnings) + tests
cargo test --all-features    # includes the serde and coords suites
just doc
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
