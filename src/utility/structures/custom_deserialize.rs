use chrono::{DateTime, FixedOffset, NaiveDateTime, NaiveDate};

use serde::de::Error as SerdeError;


pub(crate) fn deserialize_naive_date<'de, D>(
    deserializer: D,
) -> Result<NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    let format = "%Y-%m-%d %H:%M:%S.%3f";
    NaiveDateTime::parse_from_str(string, format)
        .map_err(SerdeError::custom)
}


pub(crate) fn deserialize_only_date<'de, D>(
    deserializer: D,
) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    let format = "%m/%d/%Y";
    NaiveDate::parse_from_str(string, format)
        .map_err(SerdeError::custom)
}

pub(crate) fn deserialize_stringified_uint<'de, D>(
    deserializer: D,
) -> Result<Option<u128>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let optional_string: Option<&str> = serde::Deserialize::deserialize(deserializer)?;

    if optional_string == None {
        return Ok(None);
    }

    let value: u128 = optional_string
        .unwrap()
        .parse::<u128>()
        .map_err(SerdeError::custom)?;

    Ok(Some(value))
}


pub(crate) fn deserialize_date_rfc<'de, D>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(string).map_err(SerdeError::custom)
}

// Expecting a
pub(crate) fn deserialize_percentage_float<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;

    let raw_error = SerdeError::custom("invalid percentage amount");
    let first_part = string.split('%').next().ok_or(raw_error)?;
    let amount = first_part
        .parse::<f32>()
        .ok()
        .unwrap_or(-1.);

    Ok(amount)
}
