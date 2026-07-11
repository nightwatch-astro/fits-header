//! Sexagesimal right-ascension / declination conversion (behind the `coords` feature).

/// Split a sexagesimal string into up to three numeric fields on spaces or colons.
fn fields(s: &str) -> Vec<f64> {
    s.split([' ', ':', '\t'])
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<f64>().ok())
        .collect()
}

/// Parse a sexagesimal right ascension (`H M S`, space- or colon-separated, optional fractional
/// seconds) to degrees (`hours × 15`).
pub fn sexagesimal_ra_to_deg(s: &str) -> Option<f64> {
    let f = fields(s.trim());
    let h = *f.first()?;
    let m = f.get(1).copied().unwrap_or(0.0);
    let sec = f.get(2).copied().unwrap_or(0.0);
    Some((h + m / 60.0 + sec / 3600.0) * 15.0)
}

/// Parse a sexagesimal declination (`±D M S`) to degrees. The sign is taken from the leading
/// token, so it is preserved even when the degrees field is `0` (`-00 30 00` → `-0.5`).
pub fn sexagesimal_dec_to_deg(s: &str) -> Option<f64> {
    let t = s.trim();
    let sign = if t.starts_with('-') { -1.0 } else { 1.0 };
    let f = fields(t.trim_start_matches(['+', '-']));
    let d = *f.first()?;
    let m = f.get(1).copied().unwrap_or(0.0);
    let sec = f.get(2).copied().unwrap_or(0.0);
    Some(sign * (d + m / 60.0 + sec / 3600.0))
}

/// Format degrees as a sexagesimal right ascension `HH MM SS.sss` (re-parses to the input within
/// millisecond-of-arc precision).
pub fn deg_to_sexagesimal_ra(deg: f64) -> String {
    let hours = deg.rem_euclid(360.0) / 15.0;
    let (h, m, s) = to_hms(hours);
    format!("{h:02} {m:02} {s:06.3}")
}

/// Format degrees as a sexagesimal declination `±DD MM SS.ss` (sign preserved).
pub fn deg_to_sexagesimal_dec(deg: f64) -> String {
    let sign = if deg.is_sign_negative() { '-' } else { '+' };
    let (d, m, s) = to_hms(deg.abs());
    format!("{sign}{d:02} {m:02} {s:05.2}")
}

/// Split a non-negative value in hours-or-degrees into whole units, minutes, and seconds.
fn to_hms(value: f64) -> (u32, u32, f64) {
    let total_seconds = value * 3600.0;
    let whole = (total_seconds / 3600.0).floor();
    let rem = total_seconds - whole * 3600.0;
    let minutes = (rem / 60.0).floor();
    let seconds = rem - minutes * 60.0;
    (whole as u32, minutes as u32, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_fields_default_to_zero() {
        assert_eq!(sexagesimal_ra_to_deg("10"), Some(150.0));
        assert_eq!(sexagesimal_ra_to_deg("10 30"), Some(157.5));
        assert_eq!(sexagesimal_dec_to_deg("45"), Some(45.0));
        assert_eq!(sexagesimal_ra_to_deg(""), None);
    }

    #[test]
    fn separators_and_fractions() {
        assert_eq!(sexagesimal_ra_to_deg("10:00:00"), Some(150.0));
        assert_eq!(sexagesimal_ra_to_deg("10 00 00"), Some(150.0));
        let d = sexagesimal_ra_to_deg("10 00 30.5").unwrap();
        assert!((d - (10.0 + 30.5 / 3600.0) * 15.0).abs() < 1e-9);
    }

    #[test]
    fn dec_sign_survives_zero_degrees() {
        assert_eq!(sexagesimal_dec_to_deg("-00 30 00"), Some(-0.5));
        assert_eq!(sexagesimal_dec_to_deg("+00 30 00"), Some(0.5));
        let s = deg_to_sexagesimal_dec(-0.5);
        assert!(s.starts_with("-00 30"));
    }

    #[test]
    fn ra_formatting_wraps_and_zero_pads() {
        assert_eq!(deg_to_sexagesimal_ra(0.0), "00 00 00.000");
        // Negative degrees wrap into [0, 360).
        let s = deg_to_sexagesimal_ra(-15.0);
        assert!(s.starts_with("23 00"));
        assert_eq!(deg_to_sexagesimal_ra(150.0), "10 00 00.000");
    }

    #[test]
    fn dec_formatting_signs_and_pads() {
        assert_eq!(deg_to_sexagesimal_dec(45.0), "+45 00 00.00");
        assert!(deg_to_sexagesimal_dec(-0.0).starts_with('-'));
    }
}
