# Data Model

All types are generic FITS constructs. No application domain types.

## Constants

- `CARD_LEN = 80` — bytes per header card.
- `BLOCK_LEN = 2880` — bytes per FITS block (36 cards).

## `Value`

A value card's payload.

| Variant | Holds | Meaning |
|---|---|---|
| `Str(String)` | unescaped string content (no quotes) | single-quoted string; `Str("")` is present-but-empty |
| `Literal(String)` | verbatim token | unquoted value: number, `T`/`F`, etc. |

## `Record`

A `Record` is `{ kind: RecordKind, raw }`. `raw: Option<Vec<[u8; 80]>>` holds the original bytes of the
record's physical card(s) — `Some` when parsed and unmodified (a long-string run holds more than one
card), `None` once created or edited. Untouched records serialize from `raw`. `RecordKind`:

| Variant | Fields | Notes |
|---|---|---|
| `Value` | `keyword: String`, `value: Value`, `comment: Option<String>` | addressable value card |
| `Commentary` | `keyword: String` (`COMMENT`/`HISTORY`/blank), `text: String` | repeatable free-text card |
| `Opaque` | `text: String` | `HIERARCH`/unrecognized; preserved, not addressable |

Rules:
- `keyword` is 1–8 chars from `A–Z 0–9 - _`, stored trimmed.
- On read, `''`→`'` is unescaped and trailing spaces trimmed for `Str`.
- A long value plus its `CONTINUE` cards is one `Value` record whose `raw` holds all those cards;
  there is no separate continuation variant.

## `Header`

| Field | Type | Rules |
|---|---|---|
| `records` | `Vec<Record>` | appearance order preserved for the header's lifetime and through serialization |

Behavioral rules:
- **Lookup / mutation** uses a `Key`. A bare name is **strict**: `get`/`set`/`remove` return
  `Err(AmbiguousKeyword)` if the name occurs more than once. `(name, occurrence)` targets one record.
- `set`: update the addressed record in place (clears its `raw`); append when the unique name is absent.
- `append`: always add a record (a value card, or a commentary card when the keyword is
  `COMMENT`/`HISTORY`/blank).
- `remove`: delete the addressed record (and its `Continuation` run for a long value).
- `get::<T>`: `None` on absent or type mismatch; `get_str` borrows `Str` content, `None` for `""`.
- Batch (`set_many`/`remove_many`): validate all entries, then apply all or none.
- Equality is semantic (keyword/value/comment/text); `raw` and the modified flag are not compared.

State: a `Header` comes from `parse` or `Header::new()`, is mutated by CRUD (each mutation applies fully
or is rejected before changing anything), and is serialized by `to_header_bytes` / `to_bytes`.

## `Key`

| Form | Constructed from | Meaning |
|---|---|---|
| `Name(String)` | `&str`, `String` | strict: the sole occurrence, else `Err(AmbiguousKeyword)` |
| `Occurrence(String, usize)` | `(&str, usize)` | the n-th occurrence (0-based) |

## `StructuralHints`

Used only when synthesizing missing structural cards on write.

| Field | Type | Default | Card |
|---|---|---|---|
| `bitpix` | `i64` | `8` | `BITPIX` |
| `naxis1` | `u32` | `1` | `NAXIS1` |
| `naxis2` | `u32` | `1` | `NAXIS2` |

`SIMPLE` is always `T`; `NAXIS` is derived. `Default` = 1×1 8-bit image.

## Conversion traits

- `FromCard { fn from_card(record: &Record) -> Option<Self> }` — impls: `String`, `f64`, `i64`, `u32`,
  `bool` (`T`/`F`, `1`/`0`), `time::PrimitiveDateTime`. Numeric impls accept decimal-form integers.
- `IntoValue { fn into_value(self) -> Value }` — impls: `&str`/`String` → `Str`; `f64`/`i64`/`u32`/
  `bool` → `Literal`; wrappers `Literal(text)`, `Fixed(f64, u8)`, `Sci(f64, u8)`.

## `FitsError`

| Variant | Raised by | Meaning |
|---|---|---|
| `AmbiguousKeyword { keyword, count }` | bare-name `get`/`set`/`remove` on a duplicated keyword | select an occurrence |
| `KeywordTooLong { keyword }` | validated mutation | keyword exceeds 8 characters |
| `InvalidKeyword { keyword }` | validated mutation | keyword has bytes outside `A–Z 0–9 - _` |
| `OccurrenceOutOfRange { keyword, occurrence, count }` | `set` on a non-existent occurrence | pick an existing occurrence |

`parse` returns `Result<Header, FitsError>` (lenient); `to_header_bytes`/`to_bytes` are infallible.
`FromCard`/`IntoValue` numeric impls cover `i8`–`i64`/`u8`–`u64`/`f32`/`f64`; `set_raw` is the
vendor-keyword escape hatch.
