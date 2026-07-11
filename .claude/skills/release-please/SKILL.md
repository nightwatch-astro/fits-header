---
name: release-please
description: Drive releases with googleapis/release-please. Use for any release, tagging, or changelog work, or when the repo has release-please config. Covers setup, publishing, and recovery.
---

# release-please

Operate [googleapis/release-please](https://github.com/googleapis/release-please)
correctly and recover botched releases. release-please turns Conventional Commits
into a long-lived **release PR**; **merging that PR** is what cuts the tag and
GitHub Release. Manual tags/releases outside this flow are the top cause of a
permanently stuck release loop.

## Step 0 — Detect (mandatory gate)

Before any release, tag, changelog, or version-bump action, check whether the
repo is release-please-managed:

```bash
scripts/detect-release-please.sh          # exit 0 = managed, 1 = not; prints config facts
```

- **Managed (exit 0)** → using release-please is **MANDATORY**. Do NOT hand-cut
  tags or run `gh release create`; do NOT hand-merge a `release-please--branches--*`
  branch. Read the config facts it prints (mode, `separate_pull_requests`,
  `include_component_in_tag`, `tag_separator`) and read `references/how-it-works.md`.
- **Not managed (exit 1)** → this skill applies only if the user wants to *set up*
  release-please. Otherwise defer to the repo's own release process.

## Do the normal release (managed repo)

1. Land `feat:` / `fix:` / `deps:` commits on the default branch as Conventional
   Commits. **Squash-merge** feature PRs with a conventional title (that title
   becomes the released commit message). `chore`/`build`/`docs`/`style` alone cut
   no release.
2. release-please opens/updates the release PR, labeled `autorelease: pending`.
3. **Review and merge the release PR** — do not edit its title or labels by hand.
4. release-please tags the merge commit, creates the GitHub Release, and flips the
   label to `autorelease: tagged`. Publishing runs from that (see `references/publishing.md`).

Force a version with a `Release-As: X.Y.Z` footer on an empty commit. Details and
the SemVer mapping: `references/git-process.md`.

## NEVER do (on a managed repo)

- `gh release create` / `git tag vX.Y.Z` to "finish" a release — it masks state
  and re-breaks the loop on the *next* merge.
- Flip `autorelease: pending` → `autorelease: tagged` by hand.
- Hand-edit release-PR titles or `.release-please-manifest.json` versions (except
  the documented bootstrap case).


## Setup / config

- Single package: `references/setup-single.md` (workflow YAML, permissions —
  note `contents`, `issues`, AND `pull-requests: write`).
- Monorepo / manifest: `references/setup-monorepo.md` (`separate-pull-requests`,
  `group-pull-request-title-pattern`, `include-component-in-tag`, `tag-separator`,
  plugins, `exclude-paths`).
- Publishing / deploying on release, and the `GITHUB_TOKEN`
  downstream-trigger gotcha: `references/publishing.md`.

## When a release is stuck or botched

Read `references/pitfalls-recovery.md`. It maps each symptom
(`There are untagged, merged release PRs outstanding - aborting`,
`PR component: undefined ...`, duplicate tags, blocked release PRs, first-run
history dumps) to root cause → fix, and gives the **safe** recovery order (fix
root cause → remove stale `autorelease: pending` → re-run) versus the
anti-patterns that perpetuate the loop.

## Reference index

| File | Use for |
|------|---------|
| `references/how-it-works.md` | Mental model, manifest vs simple, label lifecycle, on-merge behavior |
| `references/git-process.md` | Conventional Commits, squash-merge, `Release-As:`, multi-change footers |
| `references/setup-single.md` | Single-package config + minimal workflow |
| `references/setup-monorepo.md` | Manifest/monorepo config keys and choices |
| `references/publishing.md` | Publish on release; `GITHUB_TOKEN` gotcha; same-job vs separate-jobs vs PAT/App-token; draft→gate→finalize for all-or-nothing multi-registry releases; cross-platform napi/maturin build matrices |
| `references/pitfalls-recovery.md` | Symptom → root cause → fix; safe vs unsafe recovery |
| `references/real-world-configs.md` | Real Google-org configs to copy from |
