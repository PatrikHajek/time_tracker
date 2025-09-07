use chrono::{Datelike, Timelike};

// TODO: Use this in place of almost all chrono::DateTime.
#[derive(PartialEq, Debug)]
pub struct DateTime {
    pub date: chrono::DateTime<chrono::Local>,
}

// TODO: Implement From<chrono::DateTime> trait.
// TODO: Implement trait for comparison between DateTime and chrono::DateTime?
// TODO: Go through and make all functions that should be methods methods.
impl DateTime {
    pub fn now() -> DateTime {
        let now = chrono::Local::now();
        let date =
            chrono::NaiveDateTime::new(now.date_naive(), now.time().with_nanosecond(0).unwrap())
                .and_local_timezone(now.timezone())
                .unwrap();
        DateTime { date }
    }

    pub fn new(date: &chrono::DateTime<chrono::Local>) -> DateTime {
        DateTime { date: date.clone() }
    }

    fn plus_milli(&self, milli: i64) -> DateTime {
        let date = chrono::DateTime::from_timestamp_millis(self.date.timestamp_millis() + milli)
            .unwrap()
            .into();
        DateTime { date }
    }

    #[allow(dead_code)]
    pub fn plus_seconds(&self, seconds: i64) -> DateTime {
        self.plus_milli(seconds * 1000)
    }

    #[allow(dead_code)]
    pub fn plus_minutes(&self, minutes: i64) -> DateTime {
        self.plus_milli(minutes * 60 * 1000)
    }

    #[allow(dead_code)]
    pub fn plus_hours(&self, hours: i64) -> DateTime {
        self.plus_milli(hours * 60 * 60 * 1000)
    }

    #[allow(dead_code)]
    pub fn plus_days(&self, days: i64) -> DateTime {
        self.plus_milli(days * 24 * 60 * 60 * 1000)
    }

    pub fn format(date: &chrono::DateTime<chrono::Local>) -> String {
        date.format("%FT%T%:z").to_string()
    }

    pub fn to_formatted(&self) -> String {
        DateTime::format(&self.date)
    }

    pub fn to_formatted_pretty(&self) -> String {
        self.date.format("%F %T %:z").to_string()
    }

    pub fn to_formatted_time(&self) -> String {
        self.date.format("%T").to_string()
    }

    // TEST: that it works when the months change in the middle of the week.
    #[allow(dead_code)]
    pub fn get_start_of_week(
        date: &chrono::DateTime<chrono::Local>,
    ) -> chrono::DateTime<chrono::Local> {
        let days_since_monday: i64 = date.weekday().num_days_from_monday().into();
        let date: chrono::DateTime<chrono::Local> = chrono::DateTime::from_timestamp_millis(
            date.timestamp_millis() - days_since_monday * 24 * 60 * 60 * 1000,
        )
        .unwrap()
        .into();
        let date = date
            .with_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap();
        date
    }

    // TODO: move to it's own struct or combine with std::time::Duration?
    pub fn get_time(
        start: &chrono::DateTime<chrono::Local>,
        end: &chrono::DateTime<chrono::Local>,
    ) -> u64 {
        let time = end.timestamp_millis() - start.timestamp_millis();
        time.try_into()
            .expect("start date must always be smaller than end date")
    }

    pub fn get_time_hr_from_milli(milli: u64) -> String {
        let mut timestamp = milli / 1000;
        const UNIT: u64 = 60;
        let seconds = timestamp % UNIT;
        timestamp /= UNIT;
        let minutes = timestamp % UNIT;
        timestamp /= UNIT;
        let hours = timestamp;
        format!("{hours}h {minutes}m {seconds}s")
    }

    // TODO: Refactor.
    pub fn modify_by_relative_input(&self, text: &str) -> Result<Self, &'static str> {
        let mut text = text.trim();
        let mut sign = 1;
        if text.starts_with("-") {
            text = &text[1..];
            sign = -1;
        }

        if text.ends_with("h") || text.ends_with("m") || text.ends_with("s") {
            let time = text[0..text.len() - 1]
                .parse::<i64>()
                .map_err(|_e| "failed to parse provided text")?;
            let out = match text.chars().last().unwrap() {
                's' => self.plus_seconds(sign * time),
                'm' => self.plus_minutes(sign * time),
                'h' => self.plus_hours(sign * time),
                _ => panic!("no other option possible in this conditional"),
            };
            return Ok(out);
        }

        const SEPARATOR: &str = ":";
        let separator_count = text.matches(SEPARATOR).collect::<Vec<&str>>().len();
        if text.len() <= "23:59".len()
            && separator_count == 1
            && !text.starts_with(SEPARATOR)
            && !text.ends_with(SEPARATOR)
        {
            let colon_index = text
                .find(SEPARATOR)
                .expect("should contain exactly one colon");
            let hour = text[0..colon_index]
                .parse::<u32>()
                .map_err(|_e| ())
                .and_then(|v| if v < 24 { Ok(v) } else { Err(()) })
                .map_err(|_e| "failed to parse hour")?;
            let minute = text[colon_index + 1..text.len()]
                .parse::<u32>()
                .map_err(|_e| ())
                .and_then(|v| if v < 60 { Ok(v) } else { Err(()) })
                .map_err(|_e| "failed to parse minute")?;

            let date_parsed = self
                .date
                .with_hour(hour)
                .unwrap()
                .with_minute(minute)
                .unwrap();
            let difference = date_parsed.timestamp_millis() - self.date.timestamp_millis();
            let is_same_day = if difference == 0 {
                sign > 0
            } else {
                sign * difference > 0
            };
            if is_same_day {
                return Ok(DateTime { date: date_parsed });
            } else {
                let out = chrono::DateTime::from_timestamp_millis(
                    date_parsed.timestamp_millis() + sign * 24 * 60 * 60 * 1000,
                )
                .unwrap()
                .into();
                return Ok(DateTime { date: out });
            }
        }

        text.parse::<u32>()
            .map_err(|_e| ())
            .and_then(|v| if v < 60 { Ok(v) } else { Err(()) })
            .map(|v| {
                let date_parsed = self.date.with_minute(v).unwrap();
                let difference = date_parsed.timestamp_millis() - self.date.timestamp_millis();
                let is_same_hour = if difference == 0 {
                    sign > 0
                } else {
                    sign * difference > 0
                };
                if is_same_hour {
                    return date_parsed;
                } else {
                    return chrono::DateTime::from_timestamp_millis(
                        date_parsed.timestamp_millis() + sign * 60 * 60 * 1000,
                    )
                    .unwrap()
                    .into();
                }
            })
            .map(|v| DateTime { date: v })
            .map_err(|_e| "failed to parse provided text")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn date_time_now_works() {
        let dt = DateTime::now();
        let date = dt.date;
        let formatted = dt.to_formatted();
        let now = chrono::Local::now();

        assert_eq!(date.year(), now.year());
        assert_eq!(date.month(), now.month());
        assert_eq!(date.day(), now.day());
        assert_eq!(date.hour(), now.hour());
        assert_eq!(date.minute(), now.minute());
        assert_eq!(date.second(), now.second());
        assert_eq!(date.nanosecond(), 0);
        assert_eq!(date.offset(), now.offset());

        assert_eq!(formatted, DateTime::format(&date));
    }

    #[test]
    fn date_time_new_works() {
        let dt = DateTime {
            date: testing::date_default(),
        };
        assert_eq!(DateTime::new(&testing::date_default()), dt);
    }

    #[test]
    fn date_time_plus_milli_works() {
        fn date_default_plus_milli(milli: i64) -> chrono::DateTime<chrono::Local> {
            let date_default = testing::date_default();
            chrono::DateTime::from_timestamp_millis(date_default.timestamp_millis() + milli)
                .unwrap()
                .into()
        }

        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(500),
            DateTime::new(&date_default_plus_milli(500))
        );
        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(500_000_000_000),
            DateTime::new(&date_default_plus_milli(500_000_000_000))
        );
        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(0),
            DateTime::new(&date_default_plus_milli(0))
        );
        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(-0),
            DateTime::new(&date_default_plus_milli(-0))
        );
        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(-300),
            DateTime::new(&date_default_plus_milli(-300))
        );
        assert_eq!(
            DateTime::new(&testing::date_default()).plus_milli(-300_000_000),
            DateTime::new(&date_default_plus_milli(-300_000_000))
        );
    }

    #[test]
    fn date_time_plus_seconds_works() {
        let date_default = testing::date_default();
        assert_eq!(
            DateTime::new(&date_default).plus_seconds(15),
            DateTime::new(&date_default).plus_milli(15 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_seconds(1500),
            DateTime::new(&date_default).plus_milli(1500 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_seconds(-12),
            DateTime::new(&date_default).plus_milli(-12 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_seconds(-400),
            DateTime::new(&date_default).plus_milli(-400 * 1000)
        );
    }

    #[test]
    fn date_time_plus_minutes_works() {
        let date_default = testing::date_default();
        assert_eq!(
            DateTime::new(&date_default).plus_minutes(15),
            DateTime::new(&date_default).plus_milli(15 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_minutes(1500),
            DateTime::new(&date_default).plus_milli(1500 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_minutes(-12),
            DateTime::new(&date_default).plus_milli(-12 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_minutes(-400),
            DateTime::new(&date_default).plus_milli(-400 * 60 * 1000)
        );
    }

    #[test]
    fn date_time_plus_hours_works() {
        let date_default = testing::date_default();
        assert_eq!(
            DateTime::new(&date_default).plus_hours(15),
            DateTime::new(&date_default).plus_milli(15 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_hours(1500),
            DateTime::new(&date_default).plus_milli(1500 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_hours(-12),
            DateTime::new(&date_default).plus_milli(-12 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_hours(-400),
            DateTime::new(&date_default).plus_milli(-400 * 60 * 60 * 1000)
        );
    }

    #[test]
    fn date_time_plus_days_works() {
        let date_default = testing::date_default();
        assert_eq!(
            DateTime::new(&date_default).plus_days(15),
            DateTime::new(&date_default).plus_milli(15 * 24 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_days(1500),
            DateTime::new(&date_default).plus_milli(1500 * 24 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_days(-12),
            DateTime::new(&date_default).plus_milli(-12 * 24 * 60 * 60 * 1000)
        );
        assert_eq!(
            DateTime::new(&date_default).plus_days(-400),
            DateTime::new(&date_default).plus_milli(-400 * 24 * 60 * 60 * 1000)
        );
    }

    #[test]
    fn date_time_get_start_of_week_works() {
        let date = DateTime::get_start_of_week(&testing::date_default());
        assert_eq!(date.weekday(), chrono::Weekday::Mon);
        assert_eq!(
            date,
            // Will break if date_default changes.
            testing::date_default()
                .with_day(&testing::date_default().day() - 2)
                .unwrap()
                .with_hour(0)
                .unwrap()
        );
        let time = date.time();
        assert_eq!(time.hour(), 0);
        assert_eq!(time.minute(), 0);
        assert_eq!(time.second(), 0);
    }

    #[test]
    fn date_time_get_time_works() {
        let start = DateTime::now().date.with_minute(2).unwrap();
        let end = start.with_minute(5).unwrap();
        let time = DateTime::get_time(&start, &end);
        assert_eq!(time, 180_000);
    }

    #[test]
    fn date_time_get_time_hr_from_milli_works() {
        let start = DateTime::now()
            .date
            .with_year(2000)
            .unwrap()
            .with_month(1)
            .unwrap()
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let end = start
            // 7*31*24=5208h
            // 4*30*24=2880h
            // 1*29*24=0696h
            .with_year(2001)
            .unwrap()
            // +744h
            .with_month(2)
            .unwrap()
            // +24h
            .with_day(2)
            .unwrap()
            // +2h
            .with_hour(2)
            .unwrap()
            .with_minute(2)
            .unwrap()
            .with_second(2)
            .unwrap();
        let text = "9554h 2m 2s";
        let time = DateTime::get_time(&start, &end);
        assert_eq!(DateTime::get_time_hr_from_milli(time), text);
    }

    #[test]
    fn date_time_modify_by_relative_input_works() -> Result<(), &'static str> {
        let date = testing::date_default();

        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("2s")?,
            DateTime::new(&date).plus_seconds(2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-2s")?,
            DateTime::new(&date).plus_seconds(-2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("60s")?,
            DateTime::new(&date).plus_seconds(60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-60s")?,
            DateTime::new(&date).plus_seconds(-60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("--15s")?,
            DateTime::new(&date).plus_seconds(15)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("+15s")?,
            DateTime::new(&date).plus_seconds(15)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("2m")?,
            DateTime::new(&date).plus_minutes(2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-2m")?,
            DateTime::new(&date).plus_minutes(-2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("60m")?,
            DateTime::new(&date).plus_minutes(60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-60m")?,
            DateTime::new(&date).plus_minutes(-60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("--15m")?,
            DateTime::new(&date).plus_minutes(15)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("+15m")?,
            DateTime::new(&date).plus_minutes(15)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("2h")?,
            DateTime::new(&date).plus_hours(2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-2h")?,
            DateTime::new(&date).plus_hours(-2)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("60h")?,
            DateTime::new(&date).plus_hours(60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("-60h")?,
            DateTime::new(&date).plus_hours(-60)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("--15h")?,
            DateTime::new(&date).plus_hours(15)
        );
        assert_eq!(
            DateTime::new(&date).modify_by_relative_input("+15h")?,
            DateTime::new(&date).plus_hours(15)
        );

        assert_eq!(
            DateTime::new(&testing::date_default(),)
                .modify_by_relative_input("10")?
                .date,
            testing::date_default().with_minute(10).unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("-10")?
                .date,
            testing::date_default()
                .with_hour(11)
                .unwrap()
                .with_minute(10)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default().with_minute(30).unwrap())
                .modify_by_relative_input("10")?
                .date,
            testing::date_default()
                .with_hour(13)
                .unwrap()
                .with_minute(10)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default().with_minute(30).unwrap())
                .modify_by_relative_input("-10")?
                .date,
            testing::date_default().with_minute(10).unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default().with_minute(10).unwrap())
                .modify_by_relative_input("10")?
                .date,
            testing::date_default().with_minute(10).unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default().with_minute(10).unwrap())
                .modify_by_relative_input("-10")?
                .date,
            testing::date_default()
                .with_hour(11)
                .unwrap()
                .with_minute(10)
                .unwrap()
        );
        // TODO: Create utils for creating/working with dates. Function that takes hour, minute and
        // second as parameters and creates the date or even DateTime.
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("-12:00")?
                .date,
            testing::date_default()
                .with_day(testing::date_default().day() - 1)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("12:00")?
                .date,
            testing::date_default()
        );
        // Sets the time and keeps the day because the time already happened today.
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("-9:02")?
                .date,
            testing::date_default()
                .with_hour(9)
                .unwrap()
                .with_minute(2)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("-09:2")?
                .date,
            testing::date_default()
                .with_hour(9)
                .unwrap()
                .with_minute(2)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("13:15")?
                .date,
            testing::date_default()
                .with_hour(13)
                .unwrap()
                .with_minute(15)
                .unwrap()
        );
        // Sets the time and day because the time hasn't happened today yet.
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("9:02")?
                .date,
            testing::date_default()
                .with_day(testing::date_default().day() + 1)
                .unwrap()
                .with_hour(9)
                .unwrap()
                .with_minute(2)
                .unwrap()
        );
        assert_eq!(
            DateTime::new(&testing::date_default())
                .modify_by_relative_input("-13:15")?
                .date,
            testing::date_default()
                .with_day(testing::date_default().day() - 1)
                .unwrap()
                .with_hour(13)
                .unwrap()
                .with_minute(15)
                .unwrap()
        );

        let dt = DateTime::new(&date);
        assert!(dt.modify_by_relative_input("").is_err());
        assert!(dt.modify_by_relative_input("-").is_err());
        assert!(dt.modify_by_relative_input("--5").is_err());
        assert!(dt.modify_by_relative_input("60").is_err());
        assert!(dt.modify_by_relative_input("-60").is_err());
        assert!(dt.modify_by_relative_input("12:").is_err());
        assert!(dt.modify_by_relative_input(":12").is_err());
        assert!(dt.modify_by_relative_input("12:05:37").is_err());
        assert!(dt.modify_by_relative_input("24:05").is_err());
        assert!(dt.modify_by_relative_input("23:60").is_err());
        assert!(dt.modify_by_relative_input("--23:40").is_err());

        Ok(())
    }
}
