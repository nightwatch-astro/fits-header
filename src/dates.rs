// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! FITS date/time interpretation.

use time::PrimitiveDateTime;

/// Parse a FITS civil date/time (`YYYY-MM-DD[Thh:mm:ss[.fff]]`), timezone-naive.
/// Delegates to [`skymath::parse_date_obs`] (itself extracted from this crate) so
/// this crate carries one implementation of FITS `DATE-OBS` parsing instead of a
/// hand-rolled duplicate; this also picks up tolerance for a trailing `Z` UTC
/// designator.
///
/// # Examples
///
/// ```
/// let dt = fits_header::parse_datetime("2026-07-11T22:15:03").unwrap();
/// assert_eq!(fits_header::format_datetime(&dt), "2026-07-11T22:15:03");
/// assert!(fits_header::parse_datetime("2026-07-11").is_some()); // date-only → midnight
/// ```
pub fn parse_datetime(s: &str) -> Option<PrimitiveDateTime> {
    skymath::parse_date_obs(s).ok()
}

/// Format a date/time back to the FITS civil form (`YYYY-MM-DDThh:mm:ss[.fff]`), dropping a zero
/// sub-second part.
///
/// # Examples
///
/// ```
/// let dt = fits_header::parse_datetime("2026-07-11T22:15:03.5").unwrap();
/// assert_eq!(fits_header::format_datetime(&dt), "2026-07-11T22:15:03.5");
/// ```
pub fn format_datetime(dt: &PrimitiveDateTime) -> String {
    let (d, t) = (dt.date(), dt.time());
    let base = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        d.year(),
        d.month() as u8,
        d.day(),
        t.hour(),
        t.minute(),
        t.second()
    );
    let nanos = t.nanosecond();
    if nanos == 0 {
        base
    } else {
        let frac = format!("{nanos:09}");
        format!("{base}.{}", frac.trim_end_matches('0'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Time;

    #[test]
    fn date_only_is_midnight() {
        let dt = parse_datetime("2026-07-11").unwrap();
        assert_eq!(dt.time(), Time::MIDNIGHT);
        assert_eq!(format_datetime(&dt), "2026-07-11T00:00:00");
    }

    #[test]
    fn seconds_and_fraction_are_optional() {
        assert_eq!(
            format_datetime(&parse_datetime("2026-07-11T22:15").unwrap()),
            "2026-07-11T22:15:00"
        );
        let dt = parse_datetime("2026-07-11T22:15:03.25").unwrap();
        assert_eq!(dt.time().nanosecond(), 250_000_000);
        assert_eq!(format_datetime(&dt), "2026-07-11T22:15:03.25");
    }

    #[test]
    fn fraction_beyond_nanoseconds_is_truncated() {
        let dt = parse_datetime("2026-07-11T00:00:00.1234567891234").unwrap();
        assert_eq!(dt.time().nanosecond(), 123_456_789);
    }

    #[test]
    fn quoted_input_is_tolerated() {
        assert!(parse_datetime("'2026-07-11T01:02:03'").is_some());
        assert!(parse_datetime("  2026-07-11  ").is_some());
    }

    #[test]
    fn trailing_z_designator_is_tolerated() {
        assert_eq!(
            parse_datetime("2026-07-11T22:15:03.25Z").unwrap(),
            parse_datetime("2026-07-11T22:15:03.25").unwrap()
        );
        assert_eq!(
            format_datetime(&parse_datetime("2026-07-11T22:15:03Z").unwrap()),
            "2026-07-11T22:15:03"
        );
    }

    #[test]
    fn invalid_forms_are_none() {
        for bad in [
            "2026-13-01",          // month
            "2026-02-30",          // day
            "2026-07-11T25:00:00", // hour
            "2026-07-11-05",       // extra date part
            "2026-07-11T1:2:3:4",  // extra time part
            "2026",                // no month/day
            "not a date",
        ] {
            assert!(parse_datetime(bad).is_none(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn format_trims_trailing_fraction_zeros() {
        let dt = parse_datetime("2026-07-11T00:00:00.100").unwrap();
        assert_eq!(format_datetime(&dt), "2026-07-11T00:00:00.1");
        let dt = parse_datetime("2026-07-11T00:00:00.000").unwrap();
        assert_eq!(format_datetime(&dt), "2026-07-11T00:00:00");
    }
}
