# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

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
  `(keyword, value, comment)`
  [`Record`](https://docs.rs/fits-header/latest/fits_header/struct.Record.html)s; it
  contains no application types.

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

See [`docs/guide.md`](docs/guide.md) for a longer, task-oriented walkthrough backed by
[`examples/quickstart.rs`](examples/quickstart.rs).

## Serialization outputs

- [`to_header_bytes()`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_header_bytes)
  — the header block only (cards + `END`, padded to a 2880-byte multiple). The primary
  path when editing a real file: splice it onto the file's data.
- [`to_bytes(&hints)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_bytes)
  — a standalone FITS object. Missing `SIMPLE`/`BITPIX`/`NAXIS*` cards are synthesized
  from the
  [`StructuralHints`](https://docs.rs/fits-header/latest/fits_header/struct.StructuralHints.html),
  and the declared data segment is zero-filled. Data larger than
  [`MAX_ZERO_FILL`](https://docs.rs/fits-header/latest/fits_header/constant.MAX_ZERO_FILL.html)
  (1 GiB) returns
  [`FitsError::DataTooLarge`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.DataTooLarge)
  instead of allocating.

## Documentation

- [`docs/guide.md`](docs/guide.md) — task-oriented quickstart.
- [docs.rs/fits-header](https://docs.rs/fits-header) — full API reference, generated
  from the crate's rustdoc. Every public item is documented; the examples are compiled
  and run as part of the test suite. Build it locally with
  `cargo doc --no-deps --all-features --open`.

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on `Header`, `Record`,
  `Value`, and `StructuralHints`.

## Development

```sh
just verify                  # fmt-check + clippy (-D warnings) + tests
cargo test --all-features    # includes the serde suite
just doc
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
