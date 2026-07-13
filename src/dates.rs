//! FITS date/time interpretation.

use time::{Date, Month, PrimitiveDateTime, Time};

/// Parse a FITS civil date/time (`YYYY-MM-DD[Thh:mm:ss[.fff]]`), timezone-naive.
///
/// # Examples
///
/// ```
/// let dt = fits_header::parse_datetime("2026-07-11T22:15:03").unwrap();
/// assert_eq!(fits_header::format_datetime(&dt), "2026-07-11T22:15:03");
/// assert!(fits_header::parse_datetime("2026-07-11").is_some()); // date-only → midnight
/// ```
pub fn parse_datetime(s: &str) -> Option<PrimitiveDateTime> {
    let t = s.trim().trim_matches('\'').trim();
    let (date_part, time_part) = match t.split_once('T') {
        Some((d, tm)) => (d, Some(tm)),
        None => (t, None),
    };

    let mut d = date_part.split('-');
    let year: i32 = d.next()?.parse().ok()?;
    let month: u8 = d.next()?.parse().ok()?;
    let day: u8 = d.next()?.parse().ok()?;
    if d.next().is_some() {
        return None;
    }
    let date = Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()?;

    let time = match time_part {
        None => Time::MIDNIGHT,
        Some(tp) => {
            let mut parts = tp.split(':');
            let hour: u8 = parts.next()?.parse().ok()?;
            let minute: u8 = parts.next()?.parse().ok()?;
            let (sec, nanos) = match parts.next() {
                None => (0u8, 0u32),
                Some(sec_field) => {
                    let (whole, frac) = match sec_field.split_once('.') {
                        Some((w, f)) => (w, Some(f)),
                        None => (sec_field, None),
                    };
                    let sec: u8 = whole.parse().ok()?;
                    let nanos = match frac {
                        None => 0,
                        Some(f) => {
                            let mut digits: String = f.chars().take(9).collect();
                            while digits.len() < 9 {
                                digits.push('0');
                            }
                            digits.parse::<u32>().ok()?
                        }
                    };
                    (sec, nanos)
                }
            };
            if parts.next().is_some() {
                return None;
            }
            Time::from_hms_nano(hour, minute, sec, nanos).ok()?
        }
    };

    Some(PrimitiveDateTime::new(date, time))
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
