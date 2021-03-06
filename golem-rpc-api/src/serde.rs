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

pub mod opt_ts_seconds {
    use serde::{de, ser};
    use std::fmt;

    use chrono::{DateTime, Utc};

    pub fn serialize<S>(dt: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match dt {
            None => serializer.serialize_none(),
            Some(dt) => super::ts_seconds::serialize(dt, serializer),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Ok(d.deserialize_option(OptSecondsTimestampVisitor)?)
    }

    struct OptSecondsTimestampVisitor;

    impl<'de> de::Visitor<'de> for OptSecondsTimestampVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a unix timestamp in seconds or none")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            super::ts_seconds::deserialize(deserializer).map(|v| Some(v))
        }
    }
}

pub mod duration {
    use serde::{de, ser};
    use std::fmt;
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
        Ok(d.deserialize_str(DurationVisitor)?)
    }

    struct DurationVisitor;

    impl<'de> de::Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("missing duration spec")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_secs(value as u64))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_secs(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_secs(value as u64))
        }

        // TODO: Better error message. Using serde::de::Unexpected::Str
        fn visit_str<E>(self, value: &str) -> Result<Duration, E>
        where
            E: de::Error,
        {
            let mut it = value.split(":").fuse();
            match (it.next(), it.next(), it.next(), it.next()) {
                (Some(h), Some(m), Some(s), None) => {
                    (|| -> Result<Duration, std::num::ParseIntError> {
                        let (h, m, s): (u64, u64, u64) = (h.parse()?, m.parse()?, s.parse()?);

                        Ok(Duration::from_secs(h * 3600 + m * 60 + s))
                    })()
                    .map_err(|e| de::Error::custom(e))
                }
                _ => Err(de::Error::custom("invalid duration format")),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Serialize, Deserialize)]
    struct A {
        #[serde(with = "duration")]
        timeout: Duration,
    }

    #[test]
    fn test_duration_serialize() {
        assert_eq!(
            r#"{"timeout":"1:00:00"}"#,
            serde_json::to_string(&A {
                timeout: Duration::from_secs(3600)
            })
            .unwrap()
        );
        assert_eq!(
            r#"{"timeout":"1:00:01"}"#,
            serde_json::to_string(&A {
                timeout: Duration::from_secs(3601)
            })
            .unwrap()
        );

        let a: A = serde_json::from_str(r#"{"timeout":"1:00:02"}"#).unwrap();

        assert_eq!(a.timeout, Duration::from_secs(3602))
    }
}
