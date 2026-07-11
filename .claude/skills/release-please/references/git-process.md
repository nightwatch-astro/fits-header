# The git process agents MUST follow

Source: release-please README, `docs/customizing.md`.

## Commit in Conventional Commits

release-please parses commit messages; a non-conventional message yields no
releasable unit and therefore no release. Prefixes that bump a version:

- `feat:` → minor
- `fix:` → patch
- `feat!:` / `fix!:` / `refactor!:`, or a `BREAKING CHANGE:` / `BREAKING-CHANGE:`
  footer → major
- `deps:` → releasable (dependency bumps)
- `chore`, `build`, `style`, `test`, `ci` alone → **no release**

## Squash-merge (strongly recommended)

> "We highly recommend that you use squash-merges when merging pull requests."

With a squash-merge, the **squash commit message (the PR title/body) is what
release-please parses.** A non-conventional squash title yields no release — and
is a common cause of "the release PR reappears after I merged it." Keep PR titles
conventional. Linear history also keeps the changelog, bisect, and revert clean.

## Multiple changes in one commit — footers at the bottom

> "Release Please allows you to represent multiple changes in a single commit,
> using footers. Important: The additional messages must be added to the bottom
> of the commit."

```
feat: adds v4 UUID to crypto

This adds support for v4 UUIDs to the library.

fix(utils): unicode no longer throws exception
  BREAKING-CHANGE: encode method no longer throws.
  Source-Link: googleapis/googleapis@5e0dcb2
```

## Force a specific version — `Release-As:` footer (preferred)

> "When a commit to the main branch has `Release-As: x.x.x` (case insensitive) in
> the commit body, Release Please will open a new pull request for the specified
> version."

```bash
git commit --allow-empty -m "chore: release 2.0.0" -m "Release-As: 2.0.0"
```

The config-file `release-as` key still works (root or per-package) but is marked
**`[DEPRECATED]`** in the schema in favor of the commit footer. If you do use the
config key, **remove it after the PR merges** or every subsequent run re-proposes
the same version. Per-package `"release-as": ""` reverts that package to normal
conventional-commit bumping.

## Fix the release notes on a squashed PR

Put `BEGIN_COMMIT_OVERRIDE` / `END_COMMIT_OVERRIDE` blocks in the merged PR body.
This does **not** work with plain merges (release-please can't tell which commits
the override applies to).

## The release PR flow (what you actually do)

1. Land `feat`/`fix`/`deps` commits on the default branch (squash-merge,
   conventional titles).
2. release-please opens/updates the release PR (`autorelease: pending`).
3. **Merge the release PR** — do not edit its title or labels by hand.
4. release-please tags the merge commit, creates the GitHub Release, flips the
   label to `autorelease: tagged`; your publish job runs from that.

## What you must NEVER do manually (on a managed repo)

- Never `gh release create` or `git tag vX.Y.Z` to "finish" a release-please
  release — it perpetuates the stuck loop (see `pitfalls-recovery.md`).
- Never flip `autorelease: pending` → `autorelease: tagged` by hand.
- Never edit the release-PR title to something the parser can't read.
- Never hand-edit `.release-please-manifest.json` versions except the documented
  bootstrap case.
- Never push feature commits directly onto a `release-please--branches--*` branch.
