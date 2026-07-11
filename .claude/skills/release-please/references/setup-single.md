# Canonical setup — single package

Source: release-please-action README + `action.yml`; release-please README.

## Required workflow permissions

```yaml
permissions:
  contents: write
  issues: write
  pull-requests: write
```

**`issues: write` is required** (release-please manages issue-style label
resources). The commonly-copied `contents: write` + `pull-requests: write` pair
is INCOMPLETE. For **org** repos you may also need
**Settings → Actions → General → "Allow GitHub Actions to create and approve
pull requests"** (separate from the token gotcha in `publishing.md`).

## Minimal single-package workflow (publish gated in the same job)

```yaml
on:
  push:
    branches: [main]

permissions:
  contents: write
  issues: write
  pull-requests: write

name: release-please

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          release-type: node
      # publish only when a release was actually cut:
      - uses: actions/checkout@v4
        if: ${{ steps.release.outputs.release_created }}
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          registry-url: 'https://registry.npmjs.org'
        if: ${{ steps.release.outputs.release_created }}
      - run: npm ci
        if: ${{ steps.release.outputs.release_created }}
      - run: npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
        if: ${{ steps.release.outputs.release_created }}
```

- `token:` is optional and defaults to `${{ github.token }}` (the built-in
  `GITHUB_TOKEN`). Pass a PAT/App token **only** if you need CI/required checks to
  run on the release PR or event-driven downstream workflows — see `publishing.md`.
- `@v4` = Node 20, `@v5` = Node 24; inputs/outputs identical.

## Optional config-file form (single package)

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "node",
  "include-component-in-tag": false,
  "packages": {
    ".": { "extra-files": ["src/index.ts"] }
  }
}
```

with `.release-please-manifest.json`:

```json
{ ".": "1.4.0" }
```

For a single package set **`include-component-in-tag: false`** so tags are plain
`vX.Y.Z` (not `<name>-vX.Y.Z`). This also sidesteps the #1205 tagging-deadlock
family — but see the nuance in `pitfalls-recovery.md`: with a *derived* component
and the Merge plugin active, `false` is exactly the config that triggered #1205.
The robust single-package choice is `include-component-in-tag: false` **plus** a
single root `.` component so no component is derived at all.
