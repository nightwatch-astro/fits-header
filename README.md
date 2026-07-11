# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- **Pure Rust, MSVC-safe** — no C or system libraries. Minimal dependency footprint.
- **Generic, not domain-specific** — an ordered header of `(keyword, value, comment)` cards.
  No application types in the API.
- **Full CRUD** — create, read, update, and delete single or multiple keywords, then
  serialize the header back into a valid FITS object. Batch mutations are atomic.
- **Typed reads** — one generic accessor, `get::<T>(keyword)`, covering `String`, `f64`,
  `i64`, `u32`, `bool`, and date/time; convenience wrappers (`get_str`, …) for readability.
- **Round-trippable** — `parse(header.to_bytes(..))` reproduces the header.

## API

```rust
use fits_header::{Header, StructuralHints};

// Read every card from raw FITS bytes.
let mut header = fits_header::parse(&bytes)?;

// Typed reads via a single generic accessor (or the named wrappers).
let exptime: Option<f64> = header.get("EXPTIME");     // == header.get_f64("EXPTIME")
let object:  Option<&str> = header.get_str("OBJECT");
let simple:  Option<bool> = header.get("SIMPLE");
let obs: Option<time::PrimitiveDateTime> = header.get("DATE-OBS");

// Create / update / delete a single keyword.
header.set("OBJECT", "M31");
header.set_f64("EXPTIME", 120.0);
header.remove("HISTORY");

// Batch mutations apply atomically — all or nothing.
header.set_many([("FILTER", "Ha"), ("GAIN", "120")])?;
header.remove_many(["TEMP", "NOTES"]);

// Serialize back into a valid FITS object (80-byte cards, 2880-byte blocks).
let out: Vec<u8> = header.to_bytes(&StructuralHints::default());
```

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on `Header`, `Card`, and
  `StructuralHints`. Enable with `fits-header = { version = "…", features = ["serde"] }`.

## Development

```sh
just verify   # fmt-check + clippy (-D warnings) + tests
just test
just doc
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
