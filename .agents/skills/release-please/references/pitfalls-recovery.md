# Pitfalls, known issues & recovery

Each pitfall: **symptom → root cause → fix.** Then the safe recovery order and the
anti-patterns that keep a release stuck forever.

## THE core rule

**Fix the root cause, not the symptom.** release-please matches merged-but-untagged
release PRs by the `autorelease: pending` label **plus** the branch name. Faking a
tag (`gh release create`) or flipping a label to `tagged` masks the state for
exactly one release — the next merged release PR re-enters the abort loop
(canonical demonstration: googleapis/release-please#1946). The user story this
skill was written for is exactly this: manual releases botched the loop
indefinitely because they never fixed why release-please could not tag.

---

## "There are untagged, merged release PRs outstanding - aborting"

**Symptom.** release-please stops opening/tagging releases; the run logs this
abort. The single most common stuck state.

**Root cause.** A merged release PR still carries `autorelease: pending` because
release-please could not tag it. Usual reasons: a **PR-title-pattern mismatch**
(the title no longer parses against `pull-request-title-pattern` /
`group-pull-request-title-pattern`), `skip-github-release: true`, a detached
release tag, a release authored under `GITHUB_TOKEN`, or (occasionally) a
transient GitHub Actions incident.

**Fix.** Fix the actual root cause (most often: remove a custom title pattern so
titles default to `chore(main): release X.Y.Z`, or correct the merged PR's title
to exactly the expected pattern). Then **remove the stale `autorelease: pending`
label** from the offending merged PR and re-run. Removing `pending` is the
documented unstick — the *opposite* of flipping it to `tagged`.

## Merged release PR never tagged — `PR component: undefined ...` (#1205)

**Symptom.** In release-please-action **v5.0.0** (embedding CLI ~17.6.0), a
single-package `node` repo with `include-component-in-tag: false` and
`separate-pull-requests` at its default `false` merges the release PR but **never
tags it**. Logs:

```
⚠ PR component: undefined does not match configured component: <package.json#name>
⚠ There are untagged, merged release PRs outstanding - aborting
```

Recurs on every push. **#1205 is OPEN, no maintainer fix.** (Note: `17.6.0` is the
embedded CLI version; the *action* is `v5.0.0` — the two artifacts have different
version lines.)

**Root cause (confirmed in `src/strategies/base.ts`).** `getComponent()` returns
`''` when `include-component-in-tag` is false, but `getBranchComponent()` has **no
such guard** and derives the component from `package.json#name`. An explicit
`component: ""` is ignored because `this.component || getDefaultComponent()`
treats empty string as falsy. With `separate-pull-requests: false` the Merge
plugin rewrites the head branch to the component-less
`release-please--branches--main`, so the merged PR parses to `component:
undefined` and the standalone-PR guard aborts without tagging.

**Fix (in order of confidence):**
1. **Documented recovery to unstick each merge:** `gh release create
   v<version> --target <merge-sha>` and relabel the merged PR from
   `autorelease: pending` to `autorelease: tagged`. (This is the ONE case where a
   manual release is the sanctioned workaround — because the tool literally cannot
   tag — but it must be repeated every release until the config is fixed.)
2. **Config workaround:** set the package `component` to a **non-empty** value
   equal to the derived name, so both sides of the comparison align (confirmed for
   the sibling nested-path bug #2214).
3. **Untested hypothesis:** setting `separate-pull-requests: true` skips the Merge
   plugin so the component branch survives and the guard matches; the tag is
   unchanged (`v<version>`, since `include-component-in-tag: false` drops the
   component from the tag either way). Mechanistically sound from source but **not
   documented, not tested in #1205, not maintainer-confirmed** — treat as a
   hypothesis, not a known fix, and it changes PR ergonomics to one-PR-per-component.

> Robust avoidance for a genuine single-package repo: use a single **root `.`
> component** with `include-component-in-tag: false` so no component is ever
> derived — there is nothing for `getBranchComponent()` to mismatch.

## The many-component `separate-pull-requests: true` cascade

**Symptom.** With `separate-pull-requests: true` in a large manifest, many
concurrent per-component release PRs all touch the single shared
`.release-please-manifest.json`; merging becomes a serial slog and mid-drain you
hit duplicate-tag failures or version drift.

**Root cause / reality.** The manifest updater rewrites **only** the merged
component's own key and preserves the others, and release-please regenerates open
PRs against HEAD each run — so conflicts are **not guaranteed** (large Google
monorepos run `true` in production). But with many components, concurrent PRs
*can* develop real merge friction, and closing a PR mid-drain can leave that
component's version ahead of the manifest (drift).

**Fix.** Prefer the default **`separate-pull-requests: false`** (one combined
manifest PR) + `group-pull-request-title-pattern` for a stable title when you want
one atomic release. Use `true` only when components must ship independently; drain
by merging and letting regeneration reconcile the rest.

## Duplicate tag / "Reference already exists" (422)

**Symptom.** Tagging fails, or a tag already exists.

**Root cause.** Usually a **hand-created tag** (from a manual `gh release create`)
that release-please then cannot reconcile; in practice this surfaces as the
untagged-merged-PR abort rather than a distinct 422.

**Fix.** Never pre-create tags on a managed repo. If a bad tag exists, delete it,
correct the manifest / `last-release-sha` if needed, remove stale
`autorelease: pending`, and re-run.

## Release PR blocked by required checks that never ran

**Symptom.** The release PR opens but no CI runs; required checks stay pending /
`action_required`; branch protection blocks the merge forever.

**Root cause.** The PR was opened by `GITHUB_TOKEN`, whose events do not spawn the
required checks.

**Fix.** Pass a PAT / GitHub App token via `token:` (see `publishing.md`, Fix B).
For monorepos with path-filtered required checks, a check that doesn't run stays
pending and blocks the merge — make a single unscoped aggregator the only required
check.

## `skip-github-release` never flips the label (#1561)

**Symptom.** Release PR merges, label stays `pending`, next run aborts.

**Root cause.** `skip-github-release: true` stops creation of the GitHub Release —
which is what tags and flips the label — so it never leaves `pending`.

**Fix.** Don't set `skip-github-release` unless you tag elsewhere. For annotated
tags, let release-please create the release then convert the tag in a follow-up
step gated on `release_created`.

## Custom PR-title-pattern mismatch (#1946, #2546, #1444)

**Symptom.** release-please stops opening PRs or reopens the same PR after merge,
with the untagged-PR abort.

**Root cause.** A custom or hand-edited title (e.g. `chore: release all packages`
with no `${version}`, or `[HOTFIX] - chore...`) fails to parse against
`pull-request-title-pattern` (default `chore${scope}: release${component}
${version}`) → `Bad pull request title` → no release built → PR keeps `pending` →
abort.

**Fix.** Remove the custom title-pattern config (let it default), or set the
merged PR's title to exactly the expected pattern; then remove stale `pending` and
re-run. Clearing the label alone unblocks one PR but the loop returns until the
pattern is fixed.

## Path-moves / detached tags (#1263)

**Symptom.** Each run pulls the entire history into the changelog; logs
`⚠ No latest release pull request found` then the untagged-PR abort.

**Root cause.** The release tag/commit is not reachable on the target branch —
caused by squash/rebase changing SHAs, force-pushes, **moving package paths**, or
a `last-release-sha` pointing off-branch. (This is intrinsic to git tagging, not a
release-please bug — a path move invalidates pre-move tags.)

**Fix.** Keep the release tag/commit reachable on the target branch; keep
`.release-please-manifest.json` accurate; remove any stale off-branch
`last-release-sha`. Do not hand-tag (it can create the very detached tag that
causes this).

## First release dumps the whole history / wrong start version

**Symptom.** The first-ever release PR pulls all history into the changelog, or
proposes an unexpected starting version.

**Root cause.** No bootstrap point and no known prior version, so release-please
walks all history and assumes a default initial version.

**Fix.** Add top-level **`"bootstrap-sha": "<full-sha>"`** (one commit before the
first you want included) to bound the first-run changelog — it is ignored on all
subsequent runs once a release-please PR has merged, and can be removed. And/or
seed the version by adding `{ "path/to/pkg": "1.1.1" }` to
`.release-please-manifest.json` on the default branch. `initial-version` sets the
first release's version. `bootstrap-sha` / `last-release-sha` are **top-level
only** and need **full** SHAs.

---

## Safe recovery order (do this)

1. Confirm a releasable unit (`feat`/`fix`/`deps`) merged since the last release.
2. **Fix the actual root cause** (most often a title-pattern mismatch; also
   `skip-github-release`, detached tag, `GITHUB_TOKEN` authoring, or a GHA incident).
3. **Remove the stale `autorelease: pending`** (or `autorelease: triggered`) label
   from the offending merged PR — the documented unstick.
4. **Re-run:** GitHub **App** users add the `release-please:force-run` label to the
   merged PR; **Action** users **retry the failed workflow run** (force-run is
   App-only).

**Config fix not taking effect on an open release PR?** release-please **reuses**
an already-open release-PR branch and does **not** re-run `extra-files` updaters
against a corrected config — it only appends manifest/CHANGELOG commits. So after
fixing config, **close the stale release PR and delete its
`release-please--branches--*` branch** to force a fresh, correct run. Release state
lives in the manifest, tags, and labels — not the branch — so deleting the branch
is safe.

**Closed-then-reopened release PR.** On close it gets `autorelease:closed`; on
reopen release-please does NOT re-add `autorelease: pending`, so nothing triggers.
Fix: manually remove `autorelease:closed`, add `autorelease: pending` (and
`release-please:force-run` for App users).

**Debug without side effects:**

```bash
release-please release-pr --token=$GITHUB_TOKEN --repo-url=<owner>/<repo> --debug --dry-run
```

(add `--trace`) shows which merged PR it considers pending and why the build
produced nothing — without opening PRs or tagging.

## Anti-patterns (do NOT do these on a managed repo)

- `gh release create` to "finish" a release — masks state, re-breaks on the next
  merge. (Only exception: the documented #1205 workaround, repeated every release
  until config is fixed.)
- Hand-flipping `autorelease: pending` → `autorelease: tagged`.
- Hand-editing manifest **versions** outside the documented bootstrap case.
- Expecting a "Do not merge" label to be honored — that is not a release-please
  feature; its equivalent is `autorelease: snooze`.
- Treating `autorelease: published` as auto-managed — release-please never sets it.

## Not release-please's fault (don't chase these as RP bugs)

- **Path moves invalidating pre-move tags** — intrinsic to git's tagging model.
- **`GITHUB_TOKEN` not firing required checks / downstream workflows** — a GitHub
  Actions platform rule; the fix is a PAT/App token or same-job gating
  (`publishing.md`), not a release-please config change.
