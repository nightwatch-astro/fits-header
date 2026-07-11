# Data Model

All types are generic FITS constructs. No application domain types.

## Constants

- `CARD_LEN = 80` — bytes per header card.
- `BLOCK_LEN = 2880` — bytes per FITS block (36 cards).

## `Value`

The value carried by a card.

| Variant | Holds | Meaning |
|---|---|---|
| `Str(String)` | unescaped string content (no quotes) | a single-quoted FITS string card; `Str("")` is a present-but-empty string |
| `Literal(String)` | verbatim token | an unquoted value: number, `T`/`F` logical, etc. |

Rules:
- On read, doubled `''` inside a string is collapsed to `'`; trailing spaces are trimmed.
- On write, `Str` is re-quoted and `'`→`''` re-escaped; `Literal` is emitted verbatim, right-justified.

## `Card`

One header entry.

| Field | Type | Rules |
|---|---|---|
| `keyword` | `String` | 1–8 chars from the set `A–Z 0–9 - _`; stored trimmed (no trailing pad) |
| `value` | `Value` | see above |
| `comment` | `Option<String>` | inline comment text (without the leading ` / `); `None` when absent |

## `Header`

An ordered collection of cards representing one primary header unit.

| Field | Type | Rules |
|---|---|---|
| `cards` | `Vec<Card>` | insertion/appearance order is preserved for the lifetime of the header and through `to_bytes` |

Behavioral rules:
- **Lookup** matches the exact-case trimmed keyword; the **first** occurrence wins.
- **Create/update** (`set`, `set_f64`, …): update the first occurrence in place; append a new card if
  absent.
- **Delete** (`remove`): remove **all** occurrences of the keyword.
- **Batch** (`set_many`, `remove_many`): validate every entry, then apply all or none (atomic).
- **Typed read** (`get::<T>`): returns `None` when the keyword is absent or the value does not convert
  to `T`. `get_str` borrows the `Str` content and returns `None` for `Str("")` (empty→absent) and for
  `Literal`.

State transitions: a `Header` is produced by `parse` or built empty via `Header::new()`/`default`,
mutated by CRUD, and consumed (by value or reference) by `to_bytes`. There are no invalid resting
states — every mutation either applies fully or is rejected before it changes the header.

## `StructuralHints`

Describes the FITS object structure for serialization only (never parsed out of a header).

| Field | Type | Default | Maps to card |
|---|---|---|---|
| `bitpix` | `i64` | `8` | `BITPIX` |
| `naxis1` | `u32` | `1` | `NAXIS1` |
| `naxis2` | `u32` | `1` | `NAXIS2` |

`NAXIS` is derived (`2` for the default image); `SIMPLE` is always `T`. `Default` yields a 1×1 8-bit
image, which produces a valid FITS object for any header.

## `FromCard` trait

The extension point behind `get::<T>()`.

```text
trait FromCard: Sized { fn from_card(card: &Card) -> Option<Self>; }
```

Provided impls: `String`, `f64`, `i64`, `u32`, `bool` (`T`/`F` and `Literal` `1`/`0`),
`time::PrimitiveDateTime`. Numeric impls accept decimal-form integers (`"20.0"` → `20`).

## `FitsError`

Error type for fallible operations (via `thiserror`).

| Variant | Raised by | Meaning |
|---|---|---|
| `KeywordTooLong { keyword }` | batch/validated mutation | keyword exceeds 8 characters |
| `InvalidKeyword { keyword }` | batch/validated mutation | keyword contains bytes outside `A–Z 0–9 - _` |

`parse` returns `Result<Header, FitsError>` for signature stability; it is lenient and currently
always returns `Ok`. `to_bytes` is infallible.
