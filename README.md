# fits-header

A dependency-free, `std`-only Rust library for reading and writing the header of a
[FITS](https://fits.gsfc.nasa.gov/fits_standard.html) file.

- **Zero dependencies** — pure `std`, MSVC-safe, no C bindings. Trivially auditable
  and publishable.
- **Generic, not domain-specific** — exposes an ordered header of
  `(keyword, value, comment)` cards. No application types leak into the API.
- **Full CRUD** — create, read, update, and delete single or multiple keywords, then
  serialize the header back into a valid FITS object.
- **Round-trippable** — `parse(header.to_bytes(..))` reproduces the header for
  representative inputs.

## Status

Early scaffold, extracted from the [`nightwatch-astro/alm`](https://github.com/nightwatch-astro/alm)
metadata pipeline. The parser, the `Header` CRUD surface, and `to_bytes`
serialization are being implemented as follow-up work; the specification lives under
[`specs/`](specs/) (SpecKit).

## Planned API

```rust
use fits_header::{Header, StructuralHints};

// Read every card from raw FITS bytes.
let mut header = fits_header::parse(&bytes)?;

// Read typed values (exact-case, trimmed 8-char keyword match).
let exptime: Option<f64> = header.get_f64("EXPTIME");
let object: Option<&str> = header.get_str("OBJECT");

// Create / update / delete keywords.
header.set("OBJECT", "M31");
header.set_f64("EXPTIME", 120.0);
header.remove("HISTORY");

// Serialize back into a valid FITS object (80-byte cards, 2880-byte blocks).
let out: Vec<u8> = header.to_bytes(&StructuralHints::default());
```

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
