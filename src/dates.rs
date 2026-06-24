//! Date/timestamp handling. A post's `date` field may be a plain date or a
//! full timestamp; these helpers parse it leniently and format it for RSS.

use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Parse a post's `date` field. Accepts, in order:
///   - RFC 3339 / ISO 8601 with offset or `Z`  (`2026-06-24T14:30:00+02:00`)
///   - date + time without an offset (assumed UTC)  (`2026-06-24 14:30[:00]`)
///   - date only (midnight UTC)  (`2026-06-24`)
///
/// Returns a timezone-aware instant, or `None` if unrecognized.
pub fn parse(s: &str) -> Option<DateTime<FixedOffset>> {
    let s = s.trim();

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt);
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Utc.from_utc_datetime(&ndt).fixed_offset());
        }
    }
    if let Ok(nd) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0)?;
        return Some(Utc.from_utc_datetime(&ndt).fixed_offset());
    }
    None
}

/// RFC-2822 form for an RSS `<pubDate>`, or the input unchanged if it isn't a
/// recognizable date/timestamp (never panics).
pub fn rfc2822(date: &str) -> String {
    match parse(date) {
        Some(dt) => dt.to_rfc2822(),
        None => date.to_string(),
    }
}

/// Today's local date as `YYYY-MM-DD` (for the `new` scaffold's simple form).
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// The current local time as an RFC-3339 timestamp with offset
/// (`2026-06-24T15:30:00+02:00`), used to stamp new posts.
pub fn now_timestamp() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_supported_forms() {
        assert_eq!(
            parse("2026-06-24").unwrap().to_rfc3339(),
            "2026-06-24T00:00:00+00:00"
        );
        assert_eq!(
            parse("2026-06-24 09:15:00").unwrap().to_rfc3339(),
            "2026-06-24T09:15:00+00:00"
        );
        assert_eq!(
            parse("2026-06-24T14:30:00+02:00").unwrap().to_rfc3339(),
            "2026-06-24T14:30:00+02:00"
        );
    }

    #[test]
    fn parse_rejects_garbage_and_impossible_dates() {
        assert!(parse("not-a-date").is_none());
        assert!(parse("2026-13-99").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn rfc2822_includes_the_time_or_falls_back() {
        assert_eq!(rfc2822("2026-06-24"), "Wed, 24 Jun 2026 00:00:00 +0000");
        assert_eq!(
            rfc2822("2026-06-24T14:30:00+02:00"),
            "Wed, 24 Jun 2026 14:30:00 +0200"
        );
        assert_eq!(
            rfc2822("2026-06-24 09:15:00"),
            "Wed, 24 Jun 2026 09:15:00 +0000"
        );
        assert_eq!(rfc2822("2026-13-99"), "2026-13-99");
    }

    #[test]
    fn newer_timestamps_sort_after_older() {
        assert!(parse("2026-06-24T10:00:00Z") > parse("2026-06-24T09:00:00Z"));
        assert!(parse("2026-06-24") < parse("2026-06-24T00:00:01Z"));
    }

    #[test]
    fn now_helpers_are_well_formed() {
        assert_eq!(today().len(), 10);
        assert!(parse(&now_timestamp()).is_some());
    }
}
