# Decision Log

Running log for the unattended implementation. Two sections: decisions made autonomously, and
decisions that want your input (defaults chosen so work can proceed).

## Decisions made autonomously

- **Access & model** (from the grill): faithful editor; byte-exact via raw retention; `Value { Str | Literal }`;
  strict unified `Key` (`Name` | `(Name, occurrence)`) with `AmbiguousKeyword`; `FromCard`/`IntoValue` with
  `Literal`/`Fixed`/`Sci`; full `CONTINUE` read+write; two outputs (`to_header_bytes` / `to_bytes`).
- **xisf-alignment items 1–4 + minors**: adopted in full (no divergence) — `coords` feature off by default;
  per-logical-value byte-exact for long strings; keyword charset validation + `set_raw` escape hatch, lowercase
  rejected by `set`; `to_bytes` synth-in-FITS-order + zero-fill declared size; generalized integer type set;
  `get` returns `Err` only on ambiguity, `Ok(None)` otherwise, never panics.
- **CONTINUE representation**: a long-string run is one logical `Value` record that retains the raw bytes of
  **all** its physical cards (no separate `Continuation` record variant). Untouched → emit all retained cards;
  edited → re-split + `LONGSTRN`. (Simplifies lookup/counting; docs updated to match.)
- **`get::<String>` on a literal** returns the literal token as text (e.g. `"120.0"`); `get_str` returns only
  `Str` content and `None` for empty/literal.
- **`time` stays a core dependency** (datetime interpretation is core per xisf item 1); `coords` adds no deps,
  only gates sexagesimal + MJD functions.

## Decisions wanting your input (proceeding with the marked default)

- **License**: single **Apache-2.0** (your earlier choice). The brief mentioned dual `MIT OR Apache-2.0`.
  *Default:* Apache-2.0. Revisit before the first crates.io publish.
- **`to_bytes` on a large declared image with no data**: zero-fills the declared array size (a valid standalone
  object). A pathological geometry could allocate a lot. *Default:* zero-fill; real-file edits should use
  `to_header_bytes` + original data. Open: cap size / error above a threshold?
- **Branch completion**: opened **PR #1** (`feat/fits-header-io` → `main`) rather than auto-merging.
  Merge when you're happy with it, or tell me to merge.
- **APM packages are unpinned** (`@srobroek-agentic` → marketplace HEAD); `apm audit` warns of drift.
  *Default:* leave unpinned for now; pin with `#tag`/`#sha` when reproducibility matters.

## Status at handover

Implementation complete and green: `just verify` passes; `clippy -D warnings` clean on default and
`--all-features`; 27 tests default, 30 with `coords,serde` (incl. proptest byte-exact + semantic
round-trip). All work committed and pushed to `feat/fits-header-io`; PR #1 open. Nothing is blocked —
the four items above are refinements, not blockers.
