use serde::de::Error as SerError;
use serde_json::Value;

// Why does this enum exist?
//
// Because the `message` field of the file validation endpoints
// (both for bulk validation and AI scoring) does not have
// a defined structure. For that reason, this generic implementation
// was chosen.
pub enum ZBValidationMessage {
    Single(String),
    Multiple(Vec<String>),
    Unknown(String),
}

impl<'de> serde::Deserialize<'de> for ZBValidationMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let base_error = String::from("[zb file message] could not de-serialize ");
        let json_value: Value = serde::Deserialize::deserialize(deserializer)?;

        // check if message is a single string message
        if json_value.is_string() {
            let string_value = json_value
                .as_str()
                .ok_or(SerError::custom(base_error + "string"))?;

            return Ok(ZBValidationMessage::Single(string_value.to_string()));
        }

        // check if message is a list of messages message
        if json_value.is_array() {
            let array_of_values = json_value
                .as_array()
                .ok_or(SerError::custom(base_error + "array of strings"))?
                .into_iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<String>>();

            return Ok(ZBValidationMessage::Multiple(array_of_values));
        }
    
        // fallback by returning it whole
        Ok(ZBValidationMessage::Unknown(json_value.to_string()))
    }

}


pub struct ZBFileValidation {
    pub success: bool,
    pub message: ZBValidationMessage,
    pub file_name: String,
    pub file_id: String,
}

#[cfg(test)]
mod test {

}