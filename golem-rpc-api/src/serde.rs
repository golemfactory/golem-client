// Modified code from
// https://github.com/chronotope/chrono/blob/master/src/datetime.rs#L1261
//
pub mod ts_seconds {
    use serde::{de, ser};
    use std::fmt;

    use chrono::offset::TimeZone;
    use chrono::{DateTime, LocalResult, Utc};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let secs = dt.timestamp() as f64;
        let milis = dt.timestamp_millis() as f64 * 0.001;
        serializer.serialize_f64(secs + milis)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Ok(d.deserialize_i64(SecondsTimestampVisitor)?)
    }

    struct SecondsTimestampVisitor;

    fn serde_from<T, E, V>(me: LocalResult<T>, ts: &V) -> Result<T, E>
    where
        E: de::Error,
        V: fmt::Display,
        T: fmt::Display,
    {
        match me {
            LocalResult::None => Err(E::custom(format!("value is not a legal timestamp: {}", ts))),
            LocalResult::Ambiguous(min, max) => Err(E::custom(format!(
                "value is an ambiguous timestamp: {}, could be either of {}, {}",
                ts, min, max
            ))),
            LocalResult::Single(val) => Ok(val),
        }
    }

    impl<'de> de::Visitor<'de> for SecondsTimestampVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a unix timestamp in seconds")
        }

        /// Deserialize a timestamp in seconds since the epoch
        fn visit_i64<E>(self, value: i64) -> Result<DateTime<Utc>, E>
        where
            E: de::Error,
        {
            serde_from(Utc.timestamp_opt(value, 0), &value)
        }

        /// Deserialize a timestamp in seconds since the epoch
        fn visit_u64<E>(self, value: u64) -> Result<DateTime<Utc>, E>
        where
            E: de::Error,
        {
            serde_from(Utc.timestamp_opt(value as i64, 0), &value)
        }

        fn visit_f64<E>(self, value: f64) -> Result<DateTime<Utc>, E>
        where
            E: de::Error,
        {
            let secs = value as i64;
            let nanos = (value.fract() * 1_000_000_000f64) as u32;
            serde_from(Utc.timestamp_opt(secs, nanos), &value)
        }
    }
}

pub mod duration {
    use serde::{ser, de};
    use std::time::Duration;


    pub fn serialize<S>(d: &Duration, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
    {
        let secs = d.as_secs();
        let minutes = secs / 60;
        let hours = minutes / 60;
        serializer.serialize_str(&format!("{}:{:02}:{:02}", hours, minutes % 60, secs % 60))
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
        where
            D: de::Deserializer<'de>,
    {
        unimplemented!()
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use serde::Serialize;

    #[derive(Serialize)]
    struct A {
        #[serde(with = "duration")]
        timeout : Duration
    }


    #[test]
    fn test_duration_serialize() {
        assert_eq!(
            r#"{"timeout":"1:00:00"}"#,
            serde_json::to_string(&A { timeout: Duration::from_secs(3600) }).unwrap());
        assert_eq!(
            r#"{"timeout":"1:00:00"}"#,
            serde_json::to_string(&A { timeout: Duration::from_secs(3601) }).unwrap())

    }
}