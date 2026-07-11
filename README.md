# fits-header

A pure-Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- **Pure Rust, MSVC-safe** — no C or system libraries; painless Windows builds and
  crates.io-friendly. Minimal dependency footprint (`time`, `thiserror`).
- **Generic, not domain-specific** — exposes an ordered header of
  `(keyword, value, comment)` cards. No application types leak into the API.
- **Full CRUD** — create, read, update, and delete single or multiple keywords, then
  serialize the header back into a valid FITS object.
- **Typed reads** — one generic accessor, `get::<T>(keyword)`, covering `String`,
  `f64`, `i64`, `u32`, `bool`, and date/time; convenience wrappers (`get_str`, …) for
  readability.
- **Round-trippable** — `parse(header.to_bytes(..))` reproduces the header for
  representative inputs (property-tested).

## Status

Early scaffold, extracted from the [`nightwatch-astro/alm`](https://github.com/nightwatch-astro/alm)
metadata pipeline. The parser, the `Header` CRUD surface, `to_bytes` serialization, and the
coordinate/date helpers are being implemented as follow-up work; the specification lives under
[`specs/`](specs/) (SpecKit).

## Planned API

```rust
use fits_header::{Header, StructuralHints};

// Read every card from raw FITS bytes.
let mut header = fits_header::parse(&bytes)?;

// Typed reads via a single generic accessor (or the named wrappers).
let exptime: Option<f64> = header.get("EXPTIME");     // == header.get_f64("EXPTIME")
let object:  Option<&str> = header.get_str("OBJECT");
let simple:  Option<bool> = header.get("SIMPLE");
// Dates parse into `time` types; MJD ↔ calendar conversions are provided.
// let obs: Option<time::PrimitiveDateTime> = header.get("DATE-OBS");

// Create / update / delete — single or several at once.
header.set("OBJECT", "M31");
header.set_f64("EXPTIME", 120.0);
header.remove("HISTORY");

// Serialize back into a valid FITS object (80-byte cards, 2880-byte blocks).
let out: Vec<u8> = header.to_bytes(&StructuralHints::default());
```

## Features

- `serde` *(off by default)* — derive `Serialize`/`Deserialize` on `Header`, `Card`, and
  `StructuralHints` for JSON/other-format (de)serialization. Enable with
  `fits-header = { version = "…", features = ["serde"] }`.

## Development

Requires a stable Rust toolchain (pinned via `rust-toolchain.toml`) and, optionally,
[`just`](https://github.com/casey/just).

```sh
just verify   # fmt-check + clippy (-D warnings) + tests
just test
just doc
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
