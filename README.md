# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- Header-scoped: this crate never owns, inspects, or fabricates pixel data. Editing a
  file on disk means
  [`Header::update_file`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.update_file),
  which preserves the existing data unit byte-for-byte; creating a new file means
  [`Header::write_to_file`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.write_to_file),
  which writes your own pixel bytes right after the header and errors instead of
  overwriting an existing path.
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

A `Header` is an in-memory value; `set`/`remove`/`append` mutate it only — nothing is
written to disk until you persist it.

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

    // Create, update, delete. These change the in-memory header only.
    header.set("OBJECT", "M31")?;
    header.set("EXPTIME", 300.0)?;
    header.remove("AIRMASS")?;

    // Batch mutations are atomic — all or nothing.
    header.set_many([("FILTER", "Ha"), ("TELESCOP", "EdgeHD 8")])?;

    // The low-level building block: the header block alone. `update_file` and
    // `write_to_file` below cover the common cases directly.
    let block: Vec<u8> = header.to_header_bytes();
    Ok(())
}

// Editing a file on disk: the data unit (and any later HDUs) survives untouched. This
// is the common path — most callers only ever edit an existing file.
fn edit_in_place(path: &std::path::Path) -> Result<()> {
    Header::update_file(path, |h| {
        h.set("OBJECT", "M31")?;
        Ok(())
    })
}

// Creating a NEW file: build a header, then hand it your own pixel bytes. Errors if
// `path` already exists — this crate never fabricates pixel data or clobbers a file.
fn create_file(path: &std::path::Path) -> Result<()> {
    let mut header = Header::new();
    header.set("OBJECT", "M31")?;
    header.write_to_file(path, &[0u8; 2880]) // stand-in pixel data
}
```

See the [guide](https://docs.rs/fits-header/latest/fits_header/guide/index.html) for a
longer, task-oriented walkthrough backed by
[`examples/quickstart.rs`](https://github.com/nightwatch-astro/fits-header/blob/main/examples/quickstart.rs).

## Repeated keywords (HISTORY / COMMENT)

Commentary keywords like `HISTORY` and `COMMENT` repeat.
[`append`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.append)
adds an occurrence,
[`get_all`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get_all)
reads every one in order, and an `("HISTORY", n)` key addresses a single
occurrence to update it in place or remove it.

```rust
use fits_header::Header;

let mut header = Header::new();
header.append("HISTORY", "dark subtracted")?;
header.append("HISTORY", "flat fielded")?;
assert_eq!(header.count("HISTORY"), 2);

// Read the processing log in order.
assert_eq!(
    header.get_all::<String>("HISTORY"),
    ["dark subtracted", "flat fielded"],
);

// Update one occurrence in place, then drop another. Commentary cards carry no
// value, so read them through `get`/`get_all`, not `get_str`.
header.set(("HISTORY", 1), "flat fielded (master flat v2)")?;
header.remove(("HISTORY", 0))?;
assert_eq!(header.get_all::<String>("HISTORY"), ["flat fielded (master flat v2)"]);
# Ok::<(), fits_header::FitsError>(())
```

## Reading and writing real files

- [`Header::read_from_file(path)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.read_from_file)
  — read a header from disk. Parsing stops at `END`, so the data unit is read but never
  interpreted.
- [`Header::update_file(path, edit)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.update_file)
  — the common path: edit an existing file in place. Reads the file, locates the header by
  scanning for `END`, hands you the parsed header to mutate, then writes the new header
  back followed by everything that came after the original one (data unit, later HDUs)
  untouched. The write is atomic (temp file + rename). Errors with
  [`FitsError::MissingEnd`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.MissingEnd)
  if the file has no `END` card.
- [`Header::write_to_file(path, data)`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.write_to_file)
  — the rarer path: create a **new** file from a header and pixel bytes you already have.
  Errors instead of overwriting `path` if it already exists; pass `&[]` for a header-only
  file.
- [`Header::to_header_bytes()`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_header_bytes)
  — the lower-level building block behind both: the header block only (cards + `END`,
  padded to a 2880-byte multiple), for callers who assemble the file bytes themselves.

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
