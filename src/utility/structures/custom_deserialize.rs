use chrono::{DateTime, FixedOffset, NaiveDateTime, NaiveDate};

use serde::de::Error as SerdeError;
use serde_json::Value;


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
    D: serde::Deserializer<'de>
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(string).map_err(SerdeError::custom)
}

// Expecting a
pub(crate) fn deserialize_percentage_float<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>
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


// More context for this function.
//
// The `message` field of the file validation endpoints
// (both for bulk validation and AI scoring) does not have a consistent
// structure. For that reason, this generic implementation was chosen.
pub(crate) fn deserialize_generic_message<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>
{
    let base_error = String::from("[zb message] could not de-serialize ");
    let json_value: Value = serde::Deserialize::deserialize(deserializer)?;

    // check if message is a single string message
    if json_value.is_string() {
        let string_value = json_value
            .as_str()
            .ok_or(SerdeError::custom(base_error + "string"))?;

        return Ok(string_value.to_string());
    }

    // check if message is a list of messages message
    if json_value.is_array() {
        let array_of_values = json_value
            .as_array()
            .ok_or(SerdeError::custom(base_error + "array of strings"))?
            .into_iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect::<Vec<String>>();

        return Ok(array_of_values.join("\n"));
    }

    // fallback by returning it whole
    Ok(json_value.to_string())
}