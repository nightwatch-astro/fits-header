# Canonical setup — monorepo / manifest

Source: `docs/manifest-releaser.md`, `schemas/config.json`, `docs/customizing.md`.

Two source-controlled files drive everything; **both must exist at the tip of the
target branch** (`.release-please-manifest.json` may be `{}` on first run).

## Config keys and exact defaults

| Key | Type / default | Effect |
|-----|----------------|--------|
| `packages` | object, **required** | Maps a repo-relative **directory** (not a file) to per-package options. `.` = repo root. |
| `separate-pull-requests` | bool, **`false`** | `false` = one combined manifest release PR; `true` = one PR per package. |
| `group-pull-request-title-pattern` | string, **`chore: release ${branch}`** | Titles the combined PR. **No effect when `separate-pull-requests: true`.** Vars: `${branch}` `${scope}` `${component}` `${version}`. |
| `include-component-in-tag` | bool, **`true`** | `true` → tag `<component><sep>v<version>`; `false` → `v<version>`. Settable globally and per-package. |
| `tag-separator` | string, **`-`** | Char between component and version, e.g. `"/"` → `infra/blueprint-test/0.18.0`. |
| `component` | per-package string | Component name in tags and the `${component}` title token. |
| `plugins` | array, **`[]`** | Post-processing plugins (see below). |
| `exclude-paths` | array | Skip a commit for a component if all its files are under one of these paths. |
| `changelog-sections` | array of `{type, section, hidden}` | Override commit-type → heading. |
| `changelog-path` | string, **`CHANGELOG.md`** | Relative to the **package** dir. |
| `bump-minor-pre-major` / `bump-patch-for-minor-pre-major` | bool, false | For `<1.0.0`: demote breaking→minor / feat→patch. |
| `bootstrap-sha` / `last-release-sha` | string, **top-level ONLY**, full SHA | First-run bounding / last-release override — see `pitfalls-recovery.md`. |
| `initial-version` | string, default `0.0.0` (node: `0.1.0`) | First release version, root or per-package. |

Keys under `packages.<path>` override the top-level defaults. **Only releasable
directory paths belong under `packages`** — putting `bootstrap-sha`/`release-type`
there makes release-please look for a package at that "path" (issue #656).

## Plugins (top-level `plugins` array)

- `linked-versions` — `{ "type": "linked-versions", "groupName": "...",
  "components": ["a","b"] }`: bump all listed components to the highest version
  among them. Optional `merge` (default true).
- `node-workspace` / `cargo-workspace` / `maven-workspace` — build a local
  dependency graph and update cross-package refs, patch-bumping dependents.
  **Rust/Node/Maven workspaces need the matching plugin** (the strategy alone does
  not update deps). When combined with `linked-versions`, set the workspace
  plugin `merge: false`.
- `sentence-case` — capitalize the leading word of changelog entries.
- `group-priority` — limit proposed PRs to the highest-priority group present.

## `extra-files` (bump arbitrary files)

String paths use the Generic annotation updater; typed objects target a field:

```json
"extra-files": [
  "src/index.ts",
  { "type": "yaml", "path": "apm.yml", "jsonpath": "$.version" },
  { "type": "xml",  "path": "src/CommonProperties.xml", "xpath": "//Project/PropertyGroup/Version" }
]
```

Types: `generic`, `json`, `yaml`, `toml` (need `jsonpath`), `xml` (needs `xpath`),
`pom`. NOTE: per-package `extra-files` **paths are joined to the package path** —
a repo-root path from inside a package silently no-ops.

## Canonical monorepo workflow

Identical to the single-package workflow, but reference the config/manifest
instead of `release-type`:

```yaml
- uses: googleapis/release-please-action@v4
  id: release
  with:
    token: ${{ secrets.MY_RELEASE_PLEASE_TOKEN }}   # optional; see publishing.md
    config-file: release-please-config.json          # default
    manifest-file: .release-please-manifest.json     # default
```

`separate-pull-requests` and `group-pull-request-title-pattern` are
**config-file keys, NOT action inputs.**

## Choosing `separate-pull-requests`

- **`false` (default) — one combined release PR.** Each component still gets its
  own version, tag, and changelog; the PR title comes from
  `group-pull-request-title-pattern`. This is what Google's largest monorepos use
  (`googleapis/google-cloud-node`, `google-cloud-java`). Prefer this for one
  atomic release.
- **`true` — one PR per component.** Used by
  `GoogleCloudPlatform/cloud-foundation-toolkit`. Every release PR (combined or
  separate) writes the single shared `.release-please-manifest.json`, but the
  updater rewrites **only that component's own key** and release-please
  regenerates open PRs against HEAD each run, so separate PRs are **not guaranteed
  to conflict**. In practice, with many components, concurrent PRs *can* develop
  merge friction that you drain by merging and letting regeneration reconcile the
  rest. Prefer `true` only when components must ship independently.

> Practical note from this monorepo: with ~19 components, `separate-pull-requests:
> true` produced enough friction (many PRs all touching the manifest, occasional
> duplicate-tag failures mid-drain, version drift if a PR was closed mid-cascade)
> that switching to the default `false` + `group-pull-request-title-pattern` — one
> combined PR — was the clean fix.
