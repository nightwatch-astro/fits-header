# fits-header

Dependency-free, std-only FITS header reader/writer: parse all cards from a FITS file, CRUD single or multiple header keywords, and serialize back to a valid FITS object.

## Agent Guidance

- **Keep it dependency-free.** This crate is `std`-only and MSVC-safe. Do not add
  any crate dependency (no `thiserror`/`anyhow`/`nom`/C bindings) — it must stay
  publishable and trivially auditable. If you reach for a dep, hand-roll it instead.
- **No app domain types.** `fits-header` exposes a generic `(keyword, value, comment)`
  header only. Application-specific mapping (e.g. `RawFileMetadata`) belongs in the
  consuming adapter, not here.
- **Round-trip is the contract.** `parse(header.to_bytes(..))` must reproduce the
  header for representative inputs. Cards are exactly 80 bytes; headers pad to a
  2880-byte multiple. Add a round-trip test with every serialization change.
- **No AI attribution in commits.** A pre-commit `git commit` guard enforces this.

## AGENTS Layering

- This root `AGENTS.md` applies to the whole repository unless a deeper file overrides it.
- Put repo-wide workflow, architecture, tool, and source-of-truth guidance here.
- Add nested `AGENTS.md` files only for subtrees that need materially different rules.
- Prefer subtree placement over invented path metadata.

## Codex Project Settings

- Project and subfolder Codex overrides live in `.codex/config.toml`.
- MCP servers for this repo or subtree should be declared under `mcp_servers.<name>` in `.codex/config.toml`.
- Keep repo-specific Codex settings here and leave user-global defaults in `~/.codex/config.toml`.

## Architecture

<!-- BEGIN ps:architecture -->
Single library crate. The public surface lives in `src/lib.rs`:

- `Header` — an ordered `Vec` of cards `{ keyword, value, comment }` with typed
  getters (`get_str`/`get_f64`/`get_i64`/`get_u32`) and a setter/builder so
  arbitrary keywords can be written (vendor escape hatch). Supports CRUD over
  single or multiple keywords.
- `parse(&[u8]) -> Result<Header>` — reads 2880-byte blocks / 80-byte cards, stops
  at `END`, ignores `HIERARCH`/`COMMENT`/`HISTORY`, unescapes single-quoted strings.
- `Header::to_bytes(&StructuralHints) -> Vec<u8>` — serializes cards back to a valid
  FITS object (structural `SIMPLE`/`BITPIX`/`NAXIS*` cards, `END`, 2880 padding,
  minimal data block).
- Helpers: `sexagesimal_ra_to_deg`, `sexagesimal_dec_to_deg`, `parse_f64`, `parse_i64`.

The parser/writer implementation is follow-up work; this repo is the extraction
scaffold (see the SpecKit spec under `specs/`).
<!-- END ps:architecture -->

## Path Mapping

| Path | Contents |
|------|----------|
| `docs/` | Documentation — `architecture/`, `decisions/` (ADRs), `api/`, `research/`, `runbooks/`, `product/`, `engineering/`, `operations/` |
| `specs/` | Feature specifications (speckit) |
| `infrastructure/` | Infrastructure config — `environments/` and `terraform/{modules,stacks,environments}/` |
| `tests/` | Integration and E2E tests |
| `scripts/` | Build tooling, automation |
| `assets/` | Static files |
| `archive/` | Retired / archived material |

## Build & Run

Library crate — no binary. Use the `justfile`:

- `just build` — `cargo build`
- `just test` — `cargo test`
- `just lint` — `cargo clippy --all-targets --all-features -- -D warnings`
- `just fmt` / `just fmt-check` — format / check formatting
- `just verify` — fmt-check + lint + test (the local gate)
- `just doc` — build API docs

## Repo

- **GitHub**: nightwatch-astro/fits-header
- **Branch strategy**: feature branches off main, squash merge
