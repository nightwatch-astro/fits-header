# Quickstart

A task-oriented walkthrough of [`fits-header`](https://docs.rs/fits-header). The snippets
below are adapted from
[`examples/quickstart.rs`](https://github.com/nightwatch-astro/fits-header/blob/main/examples/quickstart.rs),
which packages the same steps into one runnable file — run it yourself with:

```sh
cargo run --example quickstart
```

This page also renders at [`fits_header::guide`](https://docs.rs/fits-header/latest/fits_header/guide/index.html);
every code block below compiles and runs as a doctest, so the guide cannot drift from the
API. Each block rebuilds the fixture from scratch (hidden lines in the rendered doctest) so
it stands alone. Full API reference: [docs.rs/fits-header](https://docs.rs/fits-header/latest/fits_header/).

## The fixture

One header, reused for every step: a CCD image of M31. One string per 80-byte card,
space-padded, in appearance order.

```rust
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
# assert_eq!(SAMPLE_CARDS.len(), 12);
```

Pack it into a valid header unit —
[`CARD_LEN`](https://docs.rs/fits-header/latest/fits_header/constant.CARD_LEN.html)-byte
cards, an `END` card, padded to a
[`BLOCK_LEN`](https://docs.rs/fits-header/latest/fits_header/constant.BLOCK_LEN.html)
multiple — and
[`Header::parse`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.parse)
it into a [`Header`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html):

```rust
use fits_header::Header;

# const SAMPLE_CARDS: &[&str] = &[
#     "SIMPLE  =                    T / conforms to FITS standard",
#     "BITPIX  =                  -32 / IEEE single-precision float",
#     "NAXIS   =                    2 / number of data axes",
#     "NAXIS1  =                 1024 / axis 1 length",
#     "NAXIS2  =                 1024 / axis 2 length",
#     "OBJECT  = 'M31     '           / target name",
#     "EXPTIME =                120.0 / exposure time in seconds",
#     "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#     "GAIN    =                  1.0 / e-/ADU",
#     "FILTER  = 'Ha      '           / filter name",
#     "TELESCOP= 'EdgeHD 8'           / telescope",
#     "HISTORY dark subtracted",
# ];
let mut bytes = Vec::new();
for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
    let mut c = card.as_bytes().to_vec();
    c.resize(fits_header::CARD_LEN, b' ');
    bytes.extend(c);
}
while bytes.len() % fits_header::BLOCK_LEN != 0 {
    bytes.push(b' ');
}

let mut header: Header = Header::parse(&bytes).unwrap();
# assert_eq!(header.get_str("OBJECT").unwrap(), Some("M31"));
```

The same bytes read from disk instead of memory:
[`Header::read_from_file`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.read_from_file)
reads the file and parses it the same way; parsing already stops at `END`, so the data
unit is read but never interpreted.

Every card is retained, including ones this guide never touches — they re-serialize
byte-for-byte at the end.

## Read

[`Header::get`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get)
is one generic accessor for every value type; string keywords also have a borrowing
shortcut,
[`Header::get_str`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get_str):

```rust
# fn sample_header() -> fits_header::Header {
#     const SAMPLE_CARDS: &[&str] = &[
#         "SIMPLE  =                    T / conforms to FITS standard",
#         "BITPIX  =                  -32 / IEEE single-precision float",
#         "NAXIS   =                    2 / number of data axes",
#         "NAXIS1  =                 1024 / axis 1 length",
#         "NAXIS2  =                 1024 / axis 2 length",
#         "OBJECT  = 'M31     '           / target name",
#         "EXPTIME =                120.0 / exposure time in seconds",
#         "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#         "GAIN    =                  1.0 / e-/ADU",
#         "FILTER  = 'Ha      '           / filter name",
#         "TELESCOP= 'EdgeHD 8'           / telescope",
#         "HISTORY dark subtracted",
#     ];
#     let mut bytes = Vec::new();
#     for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
#         let mut c = card.as_bytes().to_vec();
#         c.resize(fits_header::CARD_LEN, b' ');
#         bytes.extend(c);
#     }
#     while bytes.len() % fits_header::BLOCK_LEN != 0 {
#         bytes.push(b' ');
#     }
#     fits_header::Header::parse(&bytes).unwrap()
# }
# let header = sample_header();
let object: Option<&str> = header.get_str("OBJECT").unwrap();
let exptime: Option<f64> = header.get("EXPTIME").unwrap();
assert_eq!(object, Some("M31"));
assert_eq!(exptime, Some(120.0));
```

`COMMENT`, `HISTORY`, and blank-keyword cards are free-text
[`RecordKind::Commentary`](https://docs.rs/fits-header/latest/fits_header/enum.RecordKind.html#variant.Commentary)
records rather than addressable
[`RecordKind::Value`](https://docs.rs/fits-header/latest/fits_header/enum.RecordKind.html#variant.Value)
cards, so they repeat. Count occurrences with
[`Header::count`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.count)
and read them all with
[`Header::get_all`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get_all):

```rust
# fn sample_header() -> fits_header::Header {
#     const SAMPLE_CARDS: &[&str] = &[
#         "SIMPLE  =                    T / conforms to FITS standard",
#         "BITPIX  =                  -32 / IEEE single-precision float",
#         "NAXIS   =                    2 / number of data axes",
#         "NAXIS1  =                 1024 / axis 1 length",
#         "NAXIS2  =                 1024 / axis 2 length",
#         "OBJECT  = 'M31     '           / target name",
#         "EXPTIME =                120.0 / exposure time in seconds",
#         "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#         "GAIN    =                  1.0 / e-/ADU",
#         "FILTER  = 'Ha      '           / filter name",
#         "TELESCOP= 'EdgeHD 8'           / telescope",
#         "HISTORY dark subtracted",
#     ];
#     let mut bytes = Vec::new();
#     for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
#         let mut c = card.as_bytes().to_vec();
#         c.resize(fits_header::CARD_LEN, b' ');
#         bytes.extend(c);
#     }
#     while bytes.len() % fits_header::BLOCK_LEN != 0 {
#         bytes.push(b' ');
#     }
#     fits_header::Header::parse(&bytes).unwrap()
# }
# let header = sample_header();
assert_eq!(header.count("HISTORY"), 1);
assert_eq!(
    header.get_all::<String>("HISTORY"),
    vec!["dark subtracted".to_string()]
);
```

Value cards are read by bare name, and that access is strict: nothing stops a keyword
like `GAIN` from appearing more than once, so if it does, `header.get::<f64>("GAIN")`
returns
[`FitsError::AmbiguousKeyword`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.AmbiguousKeyword)
instead of guessing. Select one occurrence with a
[`Key`](https://docs.rs/fits-header/latest/fits_header/enum.Key.html) pair, e.g.
`header.get::<f64>(("GAIN", 1))` for the second occurrence.

`HIERARCH` cards and other non-standard or malformed cards parse as opaque
[`RecordKind::Opaque`](https://docs.rs/fits-header/latest/fits_header/enum.RecordKind.html#variant.Opaque)
records. They pass through unmodified on re-serialization, but they carry no addressable
keyword — `get`, `set`, and `remove` never see them, and
[`Header::count`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.count)
reports them as absent.

## Mutate

[`Header::set`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.set)
updates the addressed card in place, or appends one when the (unique) keyword is absent.
[`Header::append`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.append)
always adds a card, which is how repeatable keywords like `HISTORY` grow.
[`Header::set_comment`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.set_comment)
and
[`Header::remove`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.remove)
round out single-card CRUD:

```rust
# fn sample_header() -> fits_header::Header {
#     const SAMPLE_CARDS: &[&str] = &[
#         "SIMPLE  =                    T / conforms to FITS standard",
#         "BITPIX  =                  -32 / IEEE single-precision float",
#         "NAXIS   =                    2 / number of data axes",
#         "NAXIS1  =                 1024 / axis 1 length",
#         "NAXIS2  =                 1024 / axis 2 length",
#         "OBJECT  = 'M31     '           / target name",
#         "EXPTIME =                120.0 / exposure time in seconds",
#         "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#         "GAIN    =                  1.0 / e-/ADU",
#         "FILTER  = 'Ha      '           / filter name",
#         "TELESCOP= 'EdgeHD 8'           / telescope",
#         "HISTORY dark subtracted",
#     ];
#     let mut bytes = Vec::new();
#     for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
#         let mut c = card.as_bytes().to_vec();
#         c.resize(fits_header::CARD_LEN, b' ');
#         bytes.extend(c);
#     }
#     while bytes.len() % fits_header::BLOCK_LEN != 0 {
#         bytes.push(b' ');
#     }
#     fits_header::Header::parse(&bytes).unwrap()
# }
# let mut header = sample_header();
header.set("OBJECT", "NGC 7000").unwrap(); // updates in place
header.append("HISTORY", "flat fielded").unwrap(); // HISTORY repeats, so this adds a second card
header.set(("HISTORY", 0), "dark subtracted (master dark v2)").unwrap(); // update one occurrence in place
header.set_comment("EXPTIME", "seconds, revised").unwrap();
header.remove("GAIN").unwrap();
# assert_eq!(header.get_str("OBJECT").unwrap(), Some("NGC 7000"));
# assert_eq!(header.count("HISTORY"), 2);
# assert_eq!(header.get_all::<String>("HISTORY"), ["dark subtracted (master dark v2)", "flat fielded"]);
```

## Atomic batches

[`Header::set_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.set_many)
and
[`Header::remove_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.remove_many)
validate every entry before applying any of them — a rejected batch leaves the header
untouched:

```rust
# fn sample_header() -> fits_header::Header {
#     const SAMPLE_CARDS: &[&str] = &[
#         "SIMPLE  =                    T / conforms to FITS standard",
#         "BITPIX  =                  -32 / IEEE single-precision float",
#         "NAXIS   =                    2 / number of data axes",
#         "NAXIS1  =                 1024 / axis 1 length",
#         "NAXIS2  =                 1024 / axis 2 length",
#         "OBJECT  = 'M31     '           / target name",
#         "EXPTIME =                120.0 / exposure time in seconds",
#         "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#         "GAIN    =                  1.0 / e-/ADU",
#         "FILTER  = 'Ha      '           / filter name",
#         "TELESCOP= 'EdgeHD 8'           / telescope",
#         "HISTORY dark subtracted",
#     ];
#     let mut bytes = Vec::new();
#     for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
#         let mut c = card.as_bytes().to_vec();
#         c.resize(fits_header::CARD_LEN, b' ');
#         bytes.extend(c);
#     }
#     while bytes.len() % fits_header::BLOCK_LEN != 0 {
#         bytes.push(b' ');
#     }
#     fits_header::Header::parse(&bytes).unwrap()
# }
# let mut header = sample_header();
header
    .set_many([("FILTER", "OIII"), ("TELESCOP", "EdgeHD 11")])
    .unwrap();
# assert_eq!(header.get_str("FILTER").unwrap(), Some("OIII"));
```

## Serialize

[`Header::to_header_bytes`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_header_bytes)
writes the header block alone — cards plus `END`, padded to a `BLOCK_LEN` multiple:

```rust
# fn sample_header() -> fits_header::Header {
#     const SAMPLE_CARDS: &[&str] = &[
#         "SIMPLE  =                    T / conforms to FITS standard",
#         "BITPIX  =                  -32 / IEEE single-precision float",
#         "NAXIS   =                    2 / number of data axes",
#         "NAXIS1  =                 1024 / axis 1 length",
#         "NAXIS2  =                 1024 / axis 2 length",
#         "OBJECT  = 'M31     '           / target name",
#         "EXPTIME =                120.0 / exposure time in seconds",
#         "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
#         "GAIN    =                  1.0 / e-/ADU",
#         "FILTER  = 'Ha      '           / filter name",
#         "TELESCOP= 'EdgeHD 8'           / telescope",
#         "HISTORY dark subtracted",
#     ];
#     let mut bytes = Vec::new();
#     for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
#         let mut c = card.as_bytes().to_vec();
#         c.resize(fits_header::CARD_LEN, b' ');
#         bytes.extend(c);
#     }
#     while bytes.len() % fits_header::BLOCK_LEN != 0 {
#         bytes.push(b' ');
#     }
#     fits_header::Header::parse(&bytes).unwrap()
# }
# let header = sample_header();
let block: Vec<u8> = header.to_header_bytes();
assert_eq!(block.len() % fits_header::BLOCK_LEN, 0);
```

`BITPIX`, `NAXIS*`, and `DATE-OBS` were never touched above, so they come back
byte-for-byte identical to the input.

This crate is header-only: it never owns, inspects, or fabricates pixel data. That
shapes the two ways real files get written:

- **Creating a new file** — write `to_header_bytes()`, then append your own pixel data
  after it. The caller owns the data; this crate never invents it.
- **Editing an existing file** —
  [`Header::update_file`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.update_file)
  reads the file, locates the header by scanning for `END`, hands you the parsed header
  to mutate, then writes the new header back followed by everything that came after the
  original one (the data unit, and any later HDUs), untouched:

  ```rust
  use fits_header::Header;

  # const SAMPLE_CARDS: &[&str] = &[
  #     "SIMPLE  =                    T / conforms to FITS standard",
  #     "BITPIX  =                  -32 / IEEE single-precision float",
  #     "NAXIS   =                    2 / number of data axes",
  #     "NAXIS1  =                 1024 / axis 1 length",
  #     "NAXIS2  =                 1024 / axis 2 length",
  #     "OBJECT  = 'M31     '           / target name",
  #     "EXPTIME =                120.0 / exposure time in seconds",
  #     "DATE-OBS= '2026-07-11T22:15:03' / UTC start of exposure",
  #     "GAIN    =                  1.0 / e-/ADU",
  #     "FILTER  = 'Ha      '           / filter name",
  #     "TELESCOP= 'EdgeHD 8'           / telescope",
  #     "HISTORY dark subtracted",
  # ];
  # let mut bytes = Vec::new();
  # for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
  #     let mut c = card.as_bytes().to_vec();
  #     c.resize(fits_header::CARD_LEN, b' ');
  #     bytes.extend(c);
  # }
  # while bytes.len() % fits_header::BLOCK_LEN != 0 {
  #     bytes.push(b' ');
  # }
  # bytes.extend_from_slice(&[0u8; 4]); // stand-in pixel data
  # let path = std::env::temp_dir().join("fits-header-guide-doctest-update_file.fits");
  # std::fs::write(&path, &bytes).unwrap();
  Header::update_file(&path, |h| {
      h.set("OBJECT", "NGC 7000")?;
      Ok(())
  })
  .unwrap();
  # let header = Header::read_from_file(&path).unwrap();
  # assert_eq!(header.get_str("OBJECT").unwrap(), Some("NGC 7000"));
  # std::fs::remove_file(&path).ok();
  ```

  The write is atomic (temp file in the same directory, then rename), so a crash cannot
  leave a truncated file. It errors with
  [`FitsError::MissingEnd`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.MissingEnd)
  if the file has no `END` card.

## Next

- [README](https://github.com/nightwatch-astro/fits-header/blob/main/README.md) for the
  feature summary and install instructions.
- [docs.rs/fits-header](https://docs.rs/fits-header/latest/fits_header/) for the full
  API reference, including number-formatting wrappers
  ([`Literal`](https://docs.rs/fits-header/latest/fits_header/struct.Literal.html),
  [`Fixed`](https://docs.rs/fits-header/latest/fits_header/struct.Fixed.html),
  [`Sci`](https://docs.rs/fits-header/latest/fits_header/struct.Sci.html)), the
  number parsers
  ([`parse_f64`](https://docs.rs/fits-header/latest/fits_header/fn.parse_f64.html),
  [`parse_i64`](https://docs.rs/fits-header/latest/fits_header/fn.parse_i64.html)), and
  the date/time helpers
  ([`parse_datetime`](https://docs.rs/fits-header/latest/fits_header/fn.parse_datetime.html),
  [`format_datetime`](https://docs.rs/fits-header/latest/fits_header/fn.format_datetime.html)).
- Extending the typed read/write layer: implement
  [`FromCard`](https://docs.rs/fits-header/latest/fits_header/trait.FromCard.html) for a
  new read type behind `Header::get`, or
  [`IntoValue`](https://docs.rs/fits-header/latest/fits_header/trait.IntoValue.html) for
  a new write type behind `Header::set`.
