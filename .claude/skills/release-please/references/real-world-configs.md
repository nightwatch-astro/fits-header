# Real-world config references

Copy from these; they are production configs, verified against the repos.

## Single package — `googleapis/release-please` (its own config)

`release-type: node`, `include-component-in-tag: false`, one root `.` package,
plain `vX.Y.Z` tags.

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "node",
  "include-component-in-tag": false,
  "packages": { ".": { "extra-files": ["src/index.ts"] } }
}
```

<https://github.com/googleapis/release-please/blob/main/release-please-config.json>

## Large monorepo, one combined PR — `googleapis/google-cloud-node`

~229–380 packages, **`separate-pull-requests: false`** (one combined release PR),
`bump-minor-pre-major: true`, `initial-version: "0.1.0"`, `sentence-case` plugin,
per-package overrides mostly empty `{}`.

<https://github.com/googleapis/google-cloud-node/blob/main/release-please-config.json>

## Monorepo, one PR per component — `GoogleCloudPlatform/cloud-foundation-toolkit`

The canonical **`separate-pull-requests: true`** example: `tag-separator: "/"`,
`include-component-in-tag: true`, per-package `release-type: "go"`,
`package-name`, `component`, `pull-request-title-pattern`,
`bump-minor-pre-major: true`.

<https://github.com/GoogleCloudPlatform/cloud-foundation-toolkit/blob/main/release-please-config.json>

## Single package with `extra-files` XML — `GoogleCloudPlatform/functions-framework-dotnet`

Root `.` config with an `xml`/`xpath` `extra-files` updater and a full custom
`changelog-sections` list.

<https://github.com/GoogleCloudPlatform/functions-framework-dotnet/blob/main/release-please-config.json>

## This repo — `srobroek/agentic-packages` (117-package `simple` manifest)

The reference case for a **language-agnostic monorepo** that release-please does
not natively understand. Highlights:

- `release-type: "simple"` per package (no native ecosystem).
- **`separate-pull-requests: false`** + `include-component-in-tag: true` +
  `tag-separator: "--"` → one combined PR; each package tags as
  `<component>--vX.Y.Z` (the separator Claude `/plugin` + Codex `plugin add`
  expect).
- `extra-files` `yaml`/`jsonpath: "$.version"` to bump each package's `apm.yml`.
- The config + manifest are **generated** from package `apm.yml` files
  (`.apm/scripts/render-docs.py release-please`), not hand-edited, with a
  `--check` CI gate. Add a package → run the generator, never edit the JSON by
  hand.
- `release-please.yml` authors the PR with a **fine-grained PAT** (not
  `GITHUB_TOKEN`) so required checks run, and amends generated version-derived
  artifacts onto the release branch. `release.yml` fires on the `*-v*` tag push
  (the PAT-authored tag DOES trigger it — Fix B in `publishing.md`).

<https://github.com/srobroek/agentic-packages/blob/main/release-please-config.json>

## Quick decision table

| Situation | Do this |
|-----------|---------|
| Need CI/required checks on the release PR | PAT / GitHub App token via `token:` (Fix B) |
| Publish only when a release is cut, one workflow | Same-job gate on `release_created` (root) / `releases_created` or `<path>--release_created` (monorepo) (Fix A) |
| One atomic release PR for a monorepo | `separate-pull-requests: false` + `group-pull-request-title-pattern` |
| Components must ship independently | `separate-pull-requests: true` + per-package `pull-request-title-pattern` |
| Force a specific next version | `Release-As: X.Y.Z` empty commit (preferred) or config `release-as` (remove after merge) |
| First release pulls whole history | Top-level full-SHA `bootstrap-sha` and/or seed the manifest version |
| Tell RP the current version of a never-released package | Edit `.release-please-manifest.json` on the default branch |
| `untagged, merged release PRs ... aborting` | Fix root cause (usually title pattern) → remove `autorelease: pending` → retry workflow |
| Merged PR never tagged, `PR component: undefined` | #1205: manual `gh release create` + relabel; or set a non-empty `component` |
| Config fix not taking on the open release PR | Close the release PR + delete `release-please--branches--*`, then re-run |
| Rust/Node/Maven workspace deps not bumped | Add the matching `*-workspace` plugin (manifest mode) |
