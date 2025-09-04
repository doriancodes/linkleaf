use anyhow::Result;
use time::{Date, macros::format_description};

pub fn parse_date(s: &str) -> Result<Date, String> {
    // Accept strictly "YYYY-MM-DD"
    let fmt = format_description!("[year]-[month]-[day]");
    Date::parse(s.trim(), &fmt).map_err(|e| e.to_string())
}

pub fn parse_tags(raw: &str) -> Result<Vec<String>, String> {
    let tags = raw
        .split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Date;

    // ---------- parse_date ----------

    #[test]
    fn parse_date_accepts_strict_iso() {
        let d = parse_date("2025-09-02").expect("valid date");
        assert_eq!(
            d,
            Date::from_calendar_date(2025, time::Month::September, 2).unwrap()
        );
    }

    #[test]
    fn parse_date_trims_whitespace() {
        let d = parse_date("  2024-02-29 \t").expect("valid leap day with whitespace");
        assert_eq!(
            d,
            Date::from_calendar_date(2024, time::Month::February, 29).unwrap()
        );
    }

    #[test]
    fn parse_date_rejects_datetime() {
        // Must be exactly YYYY-MM-DD; datetime strings should fail.
        assert!(parse_date("2025-09-02 12:34:56").is_err());
    }

    #[test]
    fn parse_date_rejects_wrong_separator_or_format() {
        assert!(parse_date("2025/09/02").is_err());
        assert!(parse_date("02-09-2025").is_err());
        assert!(parse_date("2025-9-2").is_err()); // no zero-padding → should fail
    }

    #[test]
    fn parse_date_rejects_invalid_calendar_dates() {
        assert!(parse_date("2025-02-30").is_err());
        assert!(parse_date("2023-02-29").is_err()); // not a leap year
        assert!(parse_date("2025-13-01").is_err());
        assert!(parse_date("2025-00-10").is_err());
        assert!(parse_date("2025-01-00").is_err());
    }

    // ---------- parse_tags ----------

    #[test]
    fn parse_tags_empty_string_yields_empty_vec() {
        let tags = parse_tags("").expect("ok");
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tags_trims_and_skips_empties() {
        let tags = parse_tags(" a, b ,  ,c , , ").expect("ok");
        assert_eq!(tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_tags_single_value() {
        let tags = parse_tags("rust").expect("ok");
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn parse_tags_handles_tabs_and_newlines() {
        let tags = parse_tags("\trust,\n async ,tokio\t").expect("ok");
        assert_eq!(tags, vec!["rust", "async", "tokio"]);
    }

    #[test]
    fn parse_tags_keeps_case_and_order() {
        let tags = parse_tags("Rust,Async,Tokio").expect("ok");
        assert_eq!(tags, vec!["Rust", "Async", "Tokio"]);
    }

    #[test]
    fn parse_tags_all_commas_or_spaces_is_empty() {
        let tags = parse_tags(" , ,  , ").expect("ok");
        assert!(tags.is_empty());
    }
}
