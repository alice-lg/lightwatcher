use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid date time string: '{0}'")]
    InvalidDateTimeString(String),
}

/// Parse date time string into a duration. A timezone can be
/// specified as a parameter. If no timezone is specified, UTC is
/// assumed. The result is the duration in seconds.
pub fn parse(s: &str) -> Result<DateTime<Utc>> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let now = Utc::now();
    let date = format!("{}", now.format("%Y-%m-%d"));
    let (date, time) = match parts.len() {
        1 => (date.as_ref(), parts[0]),
        2 => (parts[0], parts[1]),
        _ => return Err(Error::InvalidDateTimeString(s.to_string()).into()),
    };

    // Parse date time string
    let datetime = format!("{} {}", date, time);
    let datetime = Utc
        .datetime_from_str(datetime.as_ref(), "%Y-%m-%d %H:%M:%S")
        .map_err(|_| Error::InvalidDateTimeString(s.to_string()))?;

    Ok(datetime)
}

/// Parse date time string into a duration
pub fn parse_duration_sec(s: &str) -> Result<f64> {
    let datetime = parse(s)?;
    let now = Utc::now();
    let duration = datetime.signed_duration_since(now);
    let duration = duration.num_seconds();
    let duration = duration.abs() as f64;

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{Datelike, Duration, Timelike};

    #[test]
    fn test_parse() {
        let result = parse("2022-06-23 10:42:11").unwrap();
        assert_eq!(result.year(), 2022);
        assert_eq!(result.month(), 6);
        assert_eq!(result.day(), 23);
        assert_eq!(result.hour(), 10);
        assert_eq!(result.minute(), 42);
        assert_eq!(result.second(), 11);
    }

    #[test]
    fn test_parse_time() {
        let result = parse("10:42:11").unwrap();
        assert_eq!(result.hour(), 10);
        assert_eq!(result.minute(), 42);
        assert_eq!(result.second(), 11);
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse("2022-06-23 10:42:11 10:42:11");
        assert!(result.is_err());

        let result = parse("123:11");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duration_sec() {
        let fiveminutesago = Utc::now() - Duration::minutes(5);
        let result =
            parse_duration_sec(format!("{}", fiveminutesago.format("%Y-%m-%d %H:%M:%S")).as_ref())
                .unwrap();
        assert_eq!(result, 300.0);
    }
}
