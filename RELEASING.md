# Releasing

1. Land Conventional Commits on `main`. `feat:` and `fix:` commits cut releases;
   `chore:` / `docs:` / `ci:` / `test:` alone do not.
2. release-please maintains a release PR (version bump + `CHANGELOG.md`). The PR is
   opened by the `sjorsr-release-bot` GitHub App so CI runs on it like any other PR.
3. Merge the release PR. release-please tags `vX.Y.Z` and creates the GitHub release;
   the `publish` job in `.github/workflows/release-please.yml` then ships the crate to
   crates.io via Trusted Publishing (OIDC). No registry token is stored anywhere.

Do not create tags or GitHub releases by hand — merging the release PR is what cuts
them. Force a specific version with an empty commit carrying a `Release-As: X.Y.Z`
footer.

## One-time configuration (done at bootstrap)

- App credentials are org-level: `RELEASE_APP_CLIENT_ID` (Actions variable) and
  `RELEASE_APP_PRIVATE_KEY` (Actions secret).
- crates.io trusted publisher (crate → Settings → Trusted Publishing): repository
  owner `nightwatch-astro`, repository `fits-header`, workflow file
  `release-please.yml`, no environment. crates.io only accepts this configuration
  after the crate exists, so version 0.1.0 was published manually with a
  since-revoked token.
