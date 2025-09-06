use chrono::{DateTime, Utc};

pub type Time = i64;

pub fn parse_from_rfc3339(str: &str) -> Result<DateTime<chrono::prelude::FixedOffset>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(str)
}

pub fn to_rfc3339(date_time: DateTime<Utc>) -> String {
    date_time.to_rfc3339()
}

pub fn now() -> Time {
    Utc::now().timestamp()
}
