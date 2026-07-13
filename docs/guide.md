# Quickstart

A task-oriented walkthrough of [`fits-header`](https://docs.rs/fits-header). Every step
below is the code in
[`examples/quickstart.rs`](https://github.com/nightwatch-astro/fits-header/blob/main/examples/quickstart.rs)
— run it yourself with:

```sh
cargo run --example quickstart
```

Full API reference: [docs.rs/fits-header](https://docs.rs/fits-header/latest/fits_header/).

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
```

Pack it into a valid header unit —
[`CARD_LEN`](https://docs.rs/fits-header/latest/fits_header/constant.CARD_LEN.html)-byte
cards, an `END` card, padded to a
[`BLOCK_LEN`](https://docs.rs/fits-header/latest/fits_header/constant.BLOCK_LEN.html)
multiple — and
[`parse`](https://docs.rs/fits-header/latest/fits_header/fn.parse.html) it into a
[`Header`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html):

```rust
let mut bytes = Vec::new();
for card in SAMPLE_CARDS.iter().chain(["END"].iter()) {
    let mut c = card.as_bytes().to_vec();
    c.resize(fits_header::CARD_LEN, b' ');
    bytes.extend(c);
}
while bytes.len() % fits_header::BLOCK_LEN != 0 {
    bytes.push(b' ');
}

let mut header: Header = fits_header::parse(&bytes).unwrap();
```

Every card is retained, including ones this guide never touches — they re-serialize
byte-for-byte at the end.

## Read

[`Header::get`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get)
is one generic accessor for every value type; string keywords also have a borrowing
shortcut,
[`Header::get_str`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.get_str):

```rust
let object: Option<&str> = header.get_str("OBJECT")?;
let exptime: Option<f64> = header.get("EXPTIME")?;
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
assert_eq!(header.count("HISTORY"), 1);
assert_eq!(
    header.get_all::<String>("HISTORY"),
    vec!["dark subtracted".to_string()]
);
```

A non-repeatable keyword is different: access by bare name is strict. If a keyword like
`GAIN` occurred more than once, `header.get::<f64>("GAIN")` would return
[`FitsError::AmbiguousKeyword`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.AmbiguousKeyword)
instead of guessing. Select one occurrence with a
[`Key`](https://docs.rs/fits-header/latest/fits_header/enum.Key.html) pair, e.g.
`header.get::<f64>(("GAIN", 1))` for the second occurrence.

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
header.set("OBJECT", "NGC 7000")?; // updates in place
header.append("HISTORY", "flat fielded")?; // HISTORY repeats, so this adds a second card
header.set_comment("EXPTIME", "seconds, revised")?;
header.remove("GAIN")?;
```

## Atomic batches

[`Header::set_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.set_many)
and
[`Header::remove_many`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.remove_many)
validate every entry before applying any of them — a rejected batch leaves the header
untouched:

```rust
header.set_many([("FILTER", "OIII"), ("TELESCOP", "EdgeHD 11")])?;
```

## Serialize

[`Header::to_header_bytes`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_header_bytes)
writes the header block alone — cards plus `END`, padded to a `BLOCK_LEN` multiple —
for splicing onto an existing file's data:

```rust
let block: Vec<u8> = header.to_header_bytes();
assert_eq!(block.len() % fits_header::BLOCK_LEN, 0);
```

`BITPIX`, `NAXIS*`, and `DATE-OBS` were never touched above, so they come back
byte-for-byte identical to the input.

[`Header::to_bytes`](https://docs.rs/fits-header/latest/fits_header/struct.Header.html#method.to_bytes)
writes a standalone FITS object instead: missing structural cards are synthesized from a
[`StructuralHints`](https://docs.rs/fits-header/latest/fits_header/struct.StructuralHints.html),
and the declared data segment is zero-filled (capped at
[`MAX_ZERO_FILL`](https://docs.rs/fits-header/latest/fits_header/constant.MAX_ZERO_FILL.html),
returning
[`FitsError::DataTooLarge`](https://docs.rs/fits-header/latest/fits_header/enum.FitsError.html#variant.DataTooLarge)
above it):

```rust
let whole_file: Vec<u8> = header.to_bytes(&StructuralHints::default())?;
assert!(whole_file.starts_with(b"SIMPLE"));
```

## Next

- [README](../README.md) for the feature summary and install instructions.
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
