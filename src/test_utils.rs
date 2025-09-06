use chrono::{Datelike, Timelike};
use crate::date_time::DateTime;

/// Returns `2002:05:05T12:00:00` with your local timezone. The day is a Wednesday.
pub fn date_default() -> chrono::DateTime<chrono::Local> {
    DateTime::now()
        .date
        .with_year(2002)
        .unwrap()
        .with_month(5)
        .unwrap()
        .with_day(8)
        .unwrap()
        .with_hour(12)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
}

pub fn now_plus_secs(secs: i64) -> chrono::DateTime<chrono::Local> {
    let date = DateTime::now().date;
    chrono::DateTime::from_timestamp_millis(date.timestamp_millis() + secs * 1000)
        .unwrap()
        .into()
}
