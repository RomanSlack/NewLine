use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::America::Denver;

use crate::task::NBSP;

/// Today's date in Mountain time (America/Denver, DST-aware), independent of the
/// machine's configured timezone.
pub fn today() -> NaiveDate {
    Utc::now().with_timezone(&Denver).date_naive()
}

/// The folder where daily journal entries live, kept separate from projects so the
/// .txt files don't mix. Created on demand.
pub fn journal_dir() -> PathBuf {
    let dir = dirs::document_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NextLine-Journal");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Path to a given day's entry file, e.g. `.../NextLine-Journal/2026-05-31.txt`.
pub fn path_for(date: NaiveDate) -> PathBuf {
    journal_dir().join(format!("{}.txt", date.format("%Y-%m-%d")))
}

/// Parse the `YYYY-MM-DD` file stem of a journal entry path.
pub fn parse_date(path: &Path) -> Option<NaiveDate> {
    let stem = path.file_stem()?.to_string_lossy();
    NaiveDate::parse_from_str(&stem, "%Y-%m-%d").ok()
}

/// Human-friendly title, e.g. "Friday, May 31, 2026".
pub fn display_title(date: NaiveDate) -> String {
    date.format("%A, %B %-d, %Y").to_string()
}

/// Heading shown inside a fresh entry, e.g. "Friday, May 31".
fn heading(date: NaiveDate) -> String {
    date.format("%A, %B %-d").to_string()
}

/// Contents of a freshly created day: a date heading and a single blank task line.
pub fn template_for(date: NaiveDate) -> String {
    format!("# {}\n\n-{}\n", heading(date), NBSP)
}

/// Ensure a given day's file exists (creating it from the template if missing),
/// returning its path.
pub fn ensure(date: NaiveDate) -> PathBuf {
    let path = path_for(date);
    if !path.exists() {
        let _ = std::fs::write(&path, template_for(date));
    }
    path
}

/// Ensure today's file exists, returning its path.
pub fn ensure_today() -> PathBuf {
    ensure(today())
}

/// Build a `NaiveDate` from year/month/day, where `month` is 1-12.
pub fn date_from_ymd(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Day-of-month for a date (1-31), convenience for calendar marking.
pub fn day_of_month(date: NaiveDate) -> u32 {
    date.day()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_uses_iso_date() {
        let d = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        assert!(path_for(d).ends_with("2026-05-31.txt"));
    }

    #[test]
    fn title_and_template_format() {
        let d = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        assert_eq!(display_title(d), "Sunday, May 31, 2026");
        assert_eq!(template_for(d), format!("# Sunday, May 31\n\n-{}\n", NBSP));
    }

    #[test]
    fn parse_roundtrips() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 9).unwrap();
        assert_eq!(parse_date(&path_for(d)), Some(d));
    }
}
