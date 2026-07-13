# Changelog

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
