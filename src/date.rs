use anyhow::{Result, anyhow};
use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Utc};

pub fn parse_input_date(s: &str) -> Result<i64> {
    Ok(NaiveDate::parse_from_str(s, "%d.%m.%Y")?
        .and_time(NaiveTime::default())
        .and_local_timezone(Local)
        .earliest()
        .ok_or_else(|| anyhow!("Failed to convert to local timezone"))?
        .timestamp())
}

pub fn timestamp_to_local_str(timestamp: i64) -> Result<String> {
    Ok(Utc
        .timestamp_opt(timestamp, 0)
        .earliest()
        .ok_or_else(|| anyhow!("Invalid timestamp"))?
        .with_timezone(&Local)
        .format("%Y-%m-%d")
        .to_string())
}

#[test]
fn test() {
    let timestamp = parse_input_date("2.9.2025").unwrap();
    let str = timestamp_to_local_str(timestamp).unwrap();
    assert_eq!(&str, "2025-09-02")
}
