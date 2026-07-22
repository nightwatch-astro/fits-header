# Changelog

## [0.4.2](https://github.com/nightwatch-astro/fits-header/compare/v0.4.1...v0.4.2) (2026-07-22)


### Bug Fixes

* use skymath 0.6.0 for FITS date parsing ([e241001](https://github.com/nightwatch-astro/fits-header/commit/e24100101aadc77c3dfab21a388241713a8ff642))


### Miscellaneous Chores

* release 0.4.2 ([9436286](https://github.com/nightwatch-astro/fits-header/commit/9436286609c3d8f915e67409066cc8e55e332431))

## [0.4.1](https://github.com/nightwatch-astro/fits-header/compare/v0.4.0...v0.4.1) (2026-07-17)


### Bug Fixes

* parse DATE-OBS via skymath for trailing-Z UTC support ([#23](https://github.com/nightwatch-astro/fits-header/issues/23)) ([7383b90](https://github.com/nightwatch-astro/fits-header/commit/7383b90e0bea6c5287a07138f6253971bdb2f0fa))

## [0.4.0](https://github.com/nightwatch-astro/fits-header/compare/v0.3.4...v0.4.0) (2026-07-17)


### ⚠ BREAKING CHANGES

* relicense from Apache-2.0 to MPL-2.0 ([#18](https://github.com/nightwatch-astro/fits-header/issues/18))

### Bug Fixes

* store CLA signatures on unprotected branch, allowlist owner ([#21](https://github.com/nightwatch-astro/fits-header/issues/21)) ([8bf5126](https://github.com/nightwatch-astro/fits-header/commit/8bf51264c1331d76d45ec8bf91de010ba7e3aad8))
* use GitHub App token for CLA bot instead of PAT ([#20](https://github.com/nightwatch-astro/fits-header/issues/20)) ([f4f1615](https://github.com/nightwatch-astro/fits-header/commit/f4f161560961066363987b7719e37e79820c3011))


### Miscellaneous Chores

* relicense from Apache-2.0 to MPL-2.0 ([#18](https://github.com/nightwatch-astro/fits-header/issues/18)) ([a87faf5](https://github.com/nightwatch-astro/fits-header/commit/a87faf52da2dbc364a80fa4879399d22a3a00966))

## [0.3.4](https://github.com/nightwatch-astro/fits-header/compare/v0.3.3...v0.3.4) (2026-07-13)


### Documentation

* add status badges ([bf24f52](https://github.com/nightwatch-astro/fits-header/commit/bf24f52b246e9c26695964e9c1a857fee7893727))

## [0.3.3](https://github.com/nightwatch-astro/fits-header/compare/v0.3.2...v0.3.3) (2026-07-13)


### Features

* add Header::write_to_file for creating new FITS files ([b857b42](https://github.com/nightwatch-astro/fits-header/commit/b857b426017e3674bfe981b478aa6b09550f7bc8))
* add Header::write_to_file for creating new FITS files ([fd351e5](https://github.com/nightwatch-astro/fits-header/commit/fd351e50e22b49dedcb7eca58a564d2a949d9a4f))

## [0.3.2](https://github.com/nightwatch-astro/fits-header/compare/v0.3.1...v0.3.2) (2026-07-13)


### Documentation

* show updating and removing repeated HISTORY/COMMENT cards ([a30aff6](https://github.com/nightwatch-astro/fits-header/commit/a30aff63df0e0338e209e3ca617256b532b9758d))

## [0.3.1](https://github.com/nightwatch-astro/fits-header/compare/v0.3.0...v0.3.1) (2026-07-13)


### Miscellaneous Chores

* release 0.3.1 ([74a7f45](https://github.com/nightwatch-astro/fits-header/commit/74a7f45d2affffaa11805c910c6b4a15a8db5f33))

## [0.3.0](https://github.com/nightwatch-astro/fits-header/compare/v0.2.0...v0.3.0) (2026-07-13)


### ⚠ BREAKING CHANGES

* removes Header::to_bytes, write::to_bytes, StructuralHints, MAX_ZERO_FILL, and FitsError::DataTooLarge. This crate never owns pixel data, so it has no business synthesizing or zero-filling it; create a file with to_header_bytes() and append your own data, or edit one in place with the new update_file.

### Features

* add header-scoped file I/O, drop data-fabricating writers ([a0ea5c5](https://github.com/nightwatch-astro/fits-header/commit/a0ea5c5d83817b7b05551f90bf6919172bb55285))


### Bug Fixes

* harden update_file atomic write and guard truncated headers ([7fb8025](https://github.com/nightwatch-astro/fits-header/commit/7fb8025bbceb998747bb96ede9847b13d1f7ac6b))

## [0.2.0](https://github.com/nightwatch-astro/fits-header/compare/v0.1.0...v0.2.0) (2026-07-11)


### ⚠ BREAKING CHANGES

* the coords cargo feature and its functions (sexagesimal_ra_to_deg, sexagesimal_dec_to_deg, deg_to_sexagesimal_ra, deg_to_sexagesimal_dec, mjd_to_datetime, datetime_to_mjd) are removed.

### Features

* remove the coords feature (sexagesimal RA/Dec and MJD helpers) ([f19cace](https://github.com/nightwatch-astro/fits-header/commit/f19cace70b2890b78c973b557e5d773483099138))
