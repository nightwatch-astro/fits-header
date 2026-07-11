# How release-please works

Sources: release-please README, `docs/manifest-releaser.md`, `docs/customizing.md`,
`schemas/config.json`, and core source (`src/strategies/base.ts`, `src/manifest.ts`).

## Mental model

> "Release Please automates CHANGELOG generation, the creation of GitHub releases,
> and version bumps" by "parsing your git history, looking for Conventional Commit
> messages, and creating release PRs."

Explicit scope limit: **it does not publish to package managers and does not do
complex branch management.** You pair it with your own publish tooling triggered
on the tag/release it creates (see `publishing.md`).

The **release PR is the core primitive.** Instead of releasing every push,
release-please keeps a single long-lived release PR up to date as releasable
commits land. When you are ready, **you merge the release PR** and that is the
release trigger. Both squash-merges and merge commits work on the release PR.

A **releasable unit** is a commit prefixed `feat`, `fix`, or `deps`. `chore` and
`build` are not releasable; some strategies add more (e.g. `docs` is releasable
for Java/Python). If only non-releasable commits land, **no release PR opens**.

**SemVer mapping:** `fix:` → patch; `feat:` → minor; a breaking change
(`feat!:`/`fix!:`/`refactor!:`, or a `BREAKING CHANGE:` / `BREAKING-CHANGE:`
footer) → major.

## What happens on merge

release-please does exactly three things when the release PR merges:

1. Updates the changelog file (and language-specific files, e.g. `package.json`).
2. **Tags** the merge commit with the version.
3. Creates a **GitHub Release** from the tag.

Publishing is NOT one of these — wire it yourself (`publishing.md`).

## The label lifecycle (the state machine — do not fight it)

| Label | Meaning | Config key / default |
|-------|---------|----------------------|
| `autorelease: pending` | Initial state of the release PR before merge | `label` / `autorelease: pending` |
| `autorelease: tagged` | Release PR merged AND the release was tagged | `release-label` / `autorelease: tagged` |
| `autorelease: snapshot` | Snapshot version bump | — |
| `autorelease: published` | A release was published — **release-please does NOT set this**; it is a convention for your publish tooling | — |

release-please finds **merged-but-untagged** release PRs by the
`autorelease: pending` label **plus the release-PR branch name**, then swaps
`pending` → `tagged` after it tags. This is exactly why hand-editing these labels
corrupts state (see `pitfalls-recovery.md`). The core library only applies
`pending`, `tagged`, `snapshot`, and `snooze`; `release-please:force-run`,
`autorelease: triggered`, and `autorelease:closed` are behaviors of the
**GitHub App**, not the core library / action.

## Simple vs manifest mode

- **Simple / single-strategy:** pass `release-type:` to the action; one component,
  no manifest authoring needed for the trivial case.
- **Manifest:** driven by two source-controlled files at the tip of the target
  branch:
  - `.release-please-manifest.json` — maps each package path to its last-released
    version. **Must exist** (may be `{}` on the very first run). `.` is the repo
    root key.
  - `release-please-config.json` — a `packages` object mapping each path to
    per-package settings that override the top-level defaults.

In `release-please-action@v4`/`@v5`, **omitting `release-type` selects manifest
mode by default.**

## release-type (strategy) values

`bazel`, `dart`, `elixir`, `go`, `helm`, `java`, `krm-blueprint`, `maven`,
`node` (default), `expo`, `ocaml`, `php`, `python`, `R`, `ruby`, `rust`, `sfdx`,
`simple`, `terraform-module`.

`simple` is language-agnostic (a `version.txt` + `CHANGELOG.md`); combined with
`extra-files` it bumps arbitrary files (this is how this repo bumps `apm.yml`).

## Action version note

`googleapis/release-please-action@v4` runs on Node 20; `@v5` differs **only** in
runtime (Node 24 — the sole v5.0.0 breaking change). Inputs and outputs are
identical between v4 and v5.
