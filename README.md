# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- Header-scoped: this crate never owns, inspects, or fabricates pixel data. Creating a
  file means appending your own data bytes to the header; editing a file means
  [`Header::update_file`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.update_file),
  which preserves the existing data unit byte-for-byte.
- Pure Rust, no C or system libraries. Builds with the MSVC toolchain.
- Every card is retained on parse; untouched cards (including long-string runs)
  re-serialize byte-for-byte. Only created or edited cards are re-rendered.
- Keyword access is strict: a bare name addresses the sole occurrence of a keyword and
  errors when it is duplicated; `("NAME", n)` selects the n-th occurrence — see
  [`Key`](https://docs.rs/fits-header/latest/fits_header/enum.Key.html).
- Create, read, update, and delete single or multiple keywords; batch mutations
  ([`set_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.set_many),
  [`remove_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.remove_many))
  are atomic (all or nothing).
- Typed reads and writes:
  [`get::<T>`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get)
  covers strings, numbers, booleans, and date/times;
  [`Literal`](https://docs.rs/fits-header/latest/fits_header/struct.Literal.html)/[`Fixed`](https://docs.rs/fits-header/latest/fits_header/struct.Fixed.html)/[`Sci`](https://docs.rs/fits-header/latest/fits_header/struct.Sci.html)
  wrappers control number formatting on write.
- Long strings use the `CONTINUE` convention on read and write (with `LONGSTRN`).
- The API is an ordered
  [`Header`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html) of
  [`Record`](https://docs.rs/fits-header/latest/fits_header/struct.Record.html)s — value
  cards, repeatable commentary cards, and opaque pass-through cards; it contains no
  application types.
- `HIERARCH` and other non-standard or malformed cards parse as
  [`RecordKind::Opaque`](https://docs.rs/fits-header/latest/fits_header/enum.RecordKind.html#variant.Opaque)
  records: preserved byte-for-byte on re-serialization, but not addressable by keyword
  (`get`/`set`/`remove` never see them).

## Install

```sh
cargo add fits-header
# opt into serde derives on the public types:
cargo add fits-header --features serde
```

## Usage

```rust
use fits_header::{Header, Result};

fn demo(bytes: &[u8]) -> Result<()> {
    // Read every card from a FITS header unit.
    let mut header = Header::parse(bytes)?;

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

    // Serialize the header block. Untouched cards come back byte-for-byte identical.
    // Creating a file: append your own pixel data after this — this crate never
    // fabricates it.
    let block: Vec<u8> = header.to_header_bytes();
    Ok(())
}

// Editing a file on disk: the data unit (and any later HDUs) survives untouched.
fn edit_in_place(path: &std::path::Path) -> Result<()> {
    Header::update_file(path, |h| {
        h.set("OBJECT", "M31")?;
        Ok(())
    })
}
```

See the [guide](https://docs.rs/fits-header/latest/fits_header/guide/index.html) for a
longer, task-oriented walkthrough backed by
[`examples/quickstart.rs`](https://github.com/nightwatch-astro/fits-header/blob/main/examples/quickstart.rs).

## Reading and writing real files

- [`Header::read_from_file(path)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.read_from_file)
  — read a header from disk. Parsing stops at `END`, so the data unit is read but never
  interpreted.
- [`Header::to_header_bytes()`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_header_bytes)
  — the header block only (cards + `END`, padded to a 2880-byte multiple). Creating a new
  FITS object: write this, then append your own pixel bytes.
- [`Header::update_file(path, edit)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.update_file)
  — edit an existing file in place. Reads the file, locates the header by scanning for
  `END`, hands you the parsed header to mutate, then writes the new header back followed
  by everything that came after the original one (data unit, later HDUs) untouched. The
  write is atomic (temp file + rename). Errors with
  [`FitsError::MissingEnd`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.MissingEnd)
  if the file has no `END` card.

## Documentation

- [guide](https://docs.rs/fits-header/latest/fits_header/guide/index.html) — task-oriented quickstart.
- [docs.rs/fits-header](https://docs.rs/fits-header) — full API reference, generated
  from the crate's rustdoc. Every public item is documented; the examples are compiled
  and run as part of the test suite. Build it locally with
  `cargo doc --no-deps --all-features --open`.

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on
  [`Header`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html),
  [`Record`](https://docs.rs/fits-header/latest/fits_header/struct.Record.html), and
  [`Value`](https://docs.rs/fits-header/latest/fits_header/enum.Value.html).

## Development

```sh
just verify                  # fmt-check + clippy (-D warnings) + tests
cargo test --all-features    # includes the serde suite
just doc
```

## License

Licensed under the [Apache License, Version 2.0](https://github.com/nightwatch-astro/fits-header/blob/main/LICENSE).
