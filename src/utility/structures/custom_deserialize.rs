use chrono::{DateTime, FixedOffset, NaiveDateTime, NaiveDate};

use serde::de::Error as SerdeError;


/// `null` reads as the type's default (the Unix epoch), as an absent field does.
pub(crate) fn deserialize_naive_date<'de, D>(
    deserializer: D,
) -> Result<NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    let Some(string) = string else {
        return Ok(NaiveDateTime::default());
    };
    let format = "%Y-%m-%d %H:%M:%S.%3f";
    NaiveDateTime::parse_from_str(&string, format)
        .map_err(SerdeError::custom)
}

/// Field-level `deserialize_with` for an optional field whose public type is not
/// nullable: `null` reads as the type's empty value (`""`, an empty list).
pub(crate) fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Default,
{
    let value: Option<T> = serde::Deserialize::deserialize(deserializer)?;
    Ok(value.unwrap_or_default())
}

/// A file job's `message` is a string or a list of strings; the public type is one
/// string, so a list is joined with newlines. `null` reads as "". Any other JSON
/// type is an error.
pub(crate) fn deserialize_message_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum MessageText {
        Text(String),
        Lines(Vec<String>),
    }

    let value: Option<MessageText> = serde::Deserialize::deserialize(deserializer)?;
    Ok(match value {
        None => String::new(),
        Some(MessageText::Text(text)) => text,
        Some(MessageText::Lines(lines)) => lines.join("\n"),
    })
}


pub(crate) fn deserialize_only_date<'de, D>(
    deserializer: D,
) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: String = serde::Deserialize::deserialize(deserializer)?;
    let format = "%m/%d/%Y";
    NaiveDate::parse_from_str(&string, format)
        .map_err(SerdeError::custom)
}

pub(crate) fn deserialize_stringified_uint<'de, D>(
    deserializer: D,
) -> Result<Option<u128>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let optional_string: Option<String> = serde::Deserialize::deserialize(deserializer)?;

    if optional_string.is_none() {
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
    let string: Option<String> = serde::Deserialize::deserialize(deserializer)?;
    let Some(string) = string else {
        return Ok(DateTime::default());
    };
    DateTime::parse_from_rfc3339(&string).map_err(SerdeError::custom)
}

// Expecting a
pub(crate) fn deserialize_percentage_float<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>
{
    let string: String = serde::Deserialize::deserialize(deserializer)?;

    let raw_error = SerdeError::custom("invalid percentage amount");
    let first_part = string.split('%').next().ok_or(raw_error)?;
    let amount = first_part
        .parse::<f32>()
        .ok()
        .unwrap_or(-1.);

    Ok(amount)
}
