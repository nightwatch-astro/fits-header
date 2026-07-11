# Publishing / deploying on release

release-please tags and creates the GitHub Release, but **does not publish**. You
wire publishing yourself. The one thing that trips everyone is the `GITHUB_TOKEN`
downstream-trigger rule.

## The GITHUB_TOKEN downstream-trigger gotcha

GitHub deliberately suppresses new workflow runs from events created with the
default `GITHUB_TOKEN`, to prevent recursion. Verbatim (GitHub Actions docs,
"Triggering a workflow from a workflow"):

> "When you use the repository's `GITHUB_TOKEN` to perform tasks, events triggered
> by the `GITHUB_TOKEN` will not create a new workflow run, with the following
> exceptions:"

Exceptions: `workflow_dispatch` and `repository_dispatch` **always** run;
`pull_request` opened/synchronize/reopened **do** create runs but in an
**approval-required** state. **All other events — including the `release` and
`push`/tag events release-please creates — are suppressed.**

Two consequences when release-please runs under `GITHUB_TOKEN`:

1. A **separate publish workflow keyed on `on: release: [published]`** (or
   `created`) **never fires.** The action README confirms: resources created by
   release-please "will not trigger future GitHub actions workflows, and workflows
   normally triggered by `release.created` events will also not run."
2. **CI / required status checks on the release PR never run** (the PR was opened
   by `GITHUB_TOKEN`), so branch protection can leave the release PR
   **permanently blocked**.

## Fix A — same-job gating (no PAT needed) — PREFERRED

Put the publish/deploy steps in the **same job** as the release-please step and
gate them on its output. This is the canonical pattern and needs no secret beyond
the built-in token.

```yaml
- uses: googleapis/release-please-action@v4
  id: release
- name: Publish
  if: ${{ steps.release.outputs.release_created }}
  run: ./publish.sh
```

**Monorepo caveat (important):** the bare singular `release_created` /
`tag_name` outputs are emitted **only for the ROOT component (path `.`)**. For a
manifest/monorepo, gate on the universal boolean **`releases_created`** (plural),
or on the **per-path** output:

```yaml
if: ${{ steps.release.outputs['packages/my-module--release_created'] }}
```

Gating a monorepo publish on the bare root `release_created` **silently never
runs** for non-root package releases.

### Full example — checkout the tag, build reproducibly, publish with OIDC

For publishers that need OIDC (`id-token: write`), reproducible builds from the
released tag, and strict step ordering, do it **all inside the release-please
job**, gated on `release_created` — deriving the tag from the action output
(`steps.release.outputs.tag_name`), NOT `github.event.release.tag_name` (there is
no `release` event to read). This is how you keep OIDC + provenance without the
broken cross-workflow `on: release` trigger and without a PAT:

```yaml
permissions:
  contents: write
  issues: write
  pull-requests: write
  id-token: write            # OIDC for the publish step

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        id: release

      # everything below is gated on a release actually being cut
      - uses: actions/checkout@v4
        if: ${{ steps.release.outputs.release_created }}
        with:
          ref: ${{ steps.release.outputs.tag_name }}   # build the released tag
      - name: Publish (OIDC, provenance)
        if: ${{ steps.release.outputs.release_created }}
        run: npm publish --provenance --access public   # no NPM_TOKEN; OIDC
```

## Fix B — PAT / GitHub App token

Pass a fine-grained PAT or a GitHub App installation token via the action
`token:` input so the PRs, tags, and releases release-please creates **do**
trigger downstream workflows and PR checks:

```yaml
- uses: googleapis/release-please-action@v4
  with:
    token: ${{ secrets.MY_RELEASE_PLEASE_TOKEN }}   # secret name is arbitrary
    release-type: simple
```

- A fine-grained PAT needs at least **Contents: Read/Write + Pull requests:
  Read/Write** (this repo's PAT is exactly that, this-repo-scoped).
- **GitHub Apps are preferred over user PATs in orgs** — scoped, non-personal,
  survive the author leaving, and can be added to a ruleset bypass list while
  humans stay restricted.
- The default `GITHUB_TOKEN` also **cannot create fork PRs** (`fork: true` needs a
  real user token).

## A vs B

Alternatives, not both: **A** keeps everything in one job under the built-in
token (least privilege, one file) — use it whenever the publish can live in the
release-please job. **B** restores event-driven fan-out to separate
`on: release` / CI workflows at the cost of managing a secret — use it when
required checks must run on the release PR, or when publish/deploy genuinely
belongs in another workflow.

## Action outputs reference

Outputs are set at runtime (`core.setOutput`) — **`action.yml` has no `outputs:`
block, so don't look there.**

- **Universal:** `releases_created` (bool), `paths_released` (JSON array),
  `prs_created` (bool), `pr` (JSON of first PR, unset if none), `prs` (JSON array).
- **Root component (path `.` / unset):** `release_created`, `tag_name`,
  `upload_url`, `html_url`, `version`, `major`, `minor`, `patch`, `sha`, `body`.
- **Non-root (monorepo):** every root output prefixed `<path>--`, e.g.
  `<path>--release_created`, `<path>--tag_name`. If the path has a `/`, use
  bracket access: `steps.release.outputs['packages/my-module--release_created']`.

Gate on truthiness — unset/false outputs are falsy. There is **no** `id` output;
the release-identifying outputs are `tag_name` and `upload_url`.

## Fix C — separate publish jobs keyed on `on: push: main` (multi-artifact)

Fix A puts publish steps **in** the release-please job. That doesn't scale to a
matrixed, multi-registry release (per-OS build fan-out, several ecosystems). The
clean pattern: keep ONE workflow triggered `on: push: [main]` (NOT `on: release`
— that event is suppressed under `GITHUB_TOKEN`, see the gotcha above), and put
build/publish in **separate jobs** that `needs: release-please` and gate on its
output:

```yaml
on:
  push:
    branches: [main]

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      releases_created: ${{ steps.release.outputs.releases_created }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release

  build-x:                       # matrix, one per platform
    needs: release-please
    if: ${{ needs.release-please.outputs.releases_created == 'true' }}
    # ... build + upload-artifact ...

  publish-x:
    needs: [release-gate, build-x]   # gate + own artifacts
    environment: release
    permissions: { contents: read, id-token: write }   # OIDC
    # ... download-artifact + publish ...
```

This still runs under the built-in `GITHUB_TOKEN` (no PAT), because the jobs live
in the SAME workflow run that the `push` to main triggered — there is no
cross-workflow `on: release` hop to be suppressed. Gate every downstream job on
the plural **`releases_created`** (monorepo-safe; the singular `release_created`
is root-component-only — see the monorepo caveat above).

## Announcement integrity — draft releases + a single gate + finalize

For a release that publishes to MULTIPLE registries (e.g. crates.io + PyPI +
npm), you want an all-or-nothing guarantee: either every registry gets the
artifacts and the GitHub Release goes public, or nothing is announced. Three
pieces:

### 1. Cut the GitHub Release as a DRAFT

In `release-please-config.json`:

```jsonc
{
  "draft": true,
  "force-tag-creation": true   // REQUIRED with draft — see below
}
```

`draft: true` makes release-please create the GitHub Release in draft state (not
public, not in the releases feed, no `release: published` notifications). The
release workflow flips it to public only after everything succeeds.

**`force-tag-creation: true` is mandatory when `draft: true`.** GitHub does NOT
create the underlying git tag for a *draft* release ("lazy tag creation") — the
tag only appears when the draft is published. Without the tag, release-please's
**next** run cannot find the previous release and generates a wrong/empty
changelog base. `force-tag-creation` makes it create the tag immediately. This is
documented in the config schema itself.

### 2. ONE gate job = "did everything pass?"

Every build matrix and every test suite feeds a single `release-gate` job. It
runs `if: always()` and asserts each dependency's `result == 'success'`. All
publish jobs depend on this one gate and nothing else. One place answers go/no-go:

```yaml
release-gate:
  needs: [release-please, release-tests, build-wheels, build-node]
  if: ${{ always() && needs.release-please.outputs.releases_created == 'true' }}
  runs-on: ubuntu-latest
  steps:
    - env:
        R_TESTS: ${{ needs.release-tests.result }}
        R_WHEELS: ${{ needs.build-wheels.result }}
        R_NODE: ${{ needs.build-node.result }}
      run: |
        set -euo pipefail
        for pair in "tests:$R_TESTS" "wheels:$R_WHEELS" "node:$R_NODE"; do
          [ "${pair##*:}" = success ] || { echo "::error::${pair%%:*} not green"; exit 1; }
        done
```

Why `always()` + explicit result checks: a job with a plain `if:` that doesn't
call `always()`/`failure()` is auto-skipped when a `needs` failed, so it can't
report the failure. `always()` forces the gate to run and *inspect* the results.
Because publish jobs `needs: release-gate` with the DEFAULT success() gate, a
failed gate skips them — nothing publishes.

**Re-running the tests here is deliberate.** ci.yml already ran them on the
release PR, but a workflow cannot depend on another workflow's result, so re-run
the SAME tasks (call the identical moon/npm/make targets — one source of truth)
as a hard pre-publish gate.

### 3. finalize-release — flip draft → public, or fail loudly

After all publish jobs, a finalize job verifies each published and flips the
draft(s) to public. If any publish failed, leave them as drafts and exit 1 — the
release is visibly incomplete and retriable, never half-announced.

```yaml
finalize-release:
  needs: [release-please, publish-crates, publish-python, publish-npm]
  if: ${{ always() && needs.release-please.outputs.releases_created == 'true' }}
  permissions: { contents: write }
  steps:
    - run: |   # fail unless every registry published
        for r in "${{ needs.publish-crates.result }}" "${{ needs.publish-python.result }}" "${{ needs.publish-npm.result }}"; do
          [ "$r" = success ] || { echo "::error::a publish failed — leaving drafts"; exit 1; }
        done
    - env: { GH_TOKEN: "${{ secrets.GITHUB_TOKEN }}" }
      run: |   # monorepo: N component drafts share the lockstep version -v<ver>
        VERSION="$(node -e "const m=require('./.release-please-manifest.json');console.log(Object.values(m)[0])")"
        gh release list --json tagName,isDraft \
          --jq ".[]|select(.isDraft and (.tagName|endswith(\"-v${VERSION}\")))|.tagName" \
        | while read -r tag; do gh release edit "$tag" --draft=false; done
```

**Monorepo note:** with N `packages` entries and no root `.`, release-please cuts
**N draft releases + N tags** per version (`<component>-v<version>`), kept in
lockstep by the `linked-versions` plugin. `linked-versions` rolls up the release
*PR* and *changelog grouping* and syncs the version numbers — but it does NOT
merge the tags/releases. finalize-release must flip all N. (There is no native
"bump N packages, cut 1 tag" mode; a single tag needs a root `.` component +
`x-release-please-version` anchors in each manifest via `extra-files`, which is
more fragile — prefer flipping N drafts unless one tag is a hard requirement.)

### Concurrency

Add a non-cancelling concurrency group so two quick pushes to main can't race on
the same draft set / tags:

```yaml
concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false   # a half-finished publish is worse than a queued one
```

## Cross-platform native artifacts (napi / maturin)

A native package (Rust addon via napi-rs, or a PyO3 wheel via maturin) publishes
BROKEN if built on one OS — `npm install` / `pip install` then fails on every
other platform. Build a per-OS matrix and publish all artifacts together.

- **Native runner wherever one is free; cross-compile only where it isn't.** A
  public repo gets free `ubuntu-24.04-arm` — so build Linux glibc x64+arm64,
  macOS Intel (`macos-13`) + ARM (`macos-14`), and Windows x64 all NATIVELY (no
  QEMU). The only target with no native runner is **musl** (Alpine) — cross-
  compile it with **zig** (`mlugg/setup-zig` + `taiki-e/install-action` for
  `cargo-zigbuild`; napi's `-x` flag). Don't use zig for macOS/Windows: native is
  more reliable and the runners are free.
- **npm (napi):** bundle every prebuilt `.node` into the ONE package rather than
  the `optionalDependencies` platform-package split. The napi loader resolves
  `./<name>.<platform>.node` locally first, so one package covers all platforms —
  and needs just ONE npm Trusted Publisher (the split needs a pre-registered
  Trusted Publisher per platform-package name, and OIDC can't first-publish names
  that don't exist yet). Also upload the loader `index.js`/`index.d.ts` (identical
  every job; `merge-multiple` on download flattens all `.node` files + one loader
  into the package dir).
- **PyPI (maturin):** build one `abi3-pyXY` wheel per platform (one wheel serves
  all Python ≥ X.Y — no per-Python matrix) + an sdist. `maturin-action` with
  `manylinux: musllinux_1_2` builds musl IN-CONTAINER automatically (no zig needed
  on the Python side). Don't pass `-i python3.12` — Windows runners ship `python`,
  not `python3.12`; abi3 auto-discovers any interpreter and always tags `cpXY-abi3`.

## Splitting PR-creation from tagging

v4 removed the v3 `command` input. Equivalents:
`command: github-release` → `skip-github-pull-request: true`;
`command: release-pr` → `skip-github-release: true`. Do **not** set
`skip-github-release: true` unless you tag elsewhere — it stops the label from
flipping to `tagged` and permanently aborts the loop (issue #1561).
