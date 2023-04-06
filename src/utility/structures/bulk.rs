use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, de::Error as SerdeError};
use serde_json::Value;

use crate::utility::structures::custom_deserialize::deserialize_date_rfc;
use crate::utility::structures::custom_deserialize::deserialize_percentage_float;

#[derive(Clone, Debug, PartialEq)]
pub enum ZBFeedbackMessage {
    Message(String),
    MultipleMessages(Vec<String>),
    Unexpected(String),
}

impl ZBFeedbackMessage {
    pub fn is_message(&self) -> bool {
        matches!(&self, ZBFeedbackMessage::Message(_))
    }

    pub fn are_multiple_messages(&self) -> bool {
        matches!(&self, ZBFeedbackMessage::MultipleMessages(_))
    }

    pub fn is_unexpected(&self) -> bool {
        matches!(&self, ZBFeedbackMessage::Unexpected(_))
    }
}

// Why does `ZBValidationMessage` enum exist?
//
// Because the `message` field of the file validation endpoints
// (both for bulk validation and AI scoring) does not have a consistent
// structure. For that reason, this generic implementation was chosen.
impl<'de> serde::Deserialize<'de> for ZBFeedbackMessage {
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
                .ok_or(SerdeError::custom(base_error + "string"))?;

            return Ok(ZBFeedbackMessage::Message(string_value.to_string()));
        }

        // check if message is a list of messages message
        if json_value.is_array() {
            let array_of_values = json_value
                .as_array()
                .ok_or(SerdeError::custom(base_error + "array of strings"))?
                .into_iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<String>>();

            return Ok(ZBFeedbackMessage::MultipleMessages(array_of_values));
        }

        // fallback by returning it whole
        Ok(ZBFeedbackMessage::Unexpected(json_value.to_string()))
    }

}

#[derive(Clone, Debug, Deserialize)]
pub struct ZBFileFeedback {
    pub success: bool,
    pub message: ZBFeedbackMessage,
    pub file_name: Option<String>,
    pub file_id: Option<String>,
}


#[derive(Clone, Debug, Deserialize)]
pub struct ZBFileStatus {
    pub success: bool,
    pub file_id: String,
    pub file_name: String,
    pub file_status: String,
    pub error_reason: Option<String>,
    pub return_url: Option<String>,

    #[serde(deserialize_with="deserialize_date_rfc")]
    pub upload_date: DateTime<FixedOffset>,

    #[serde(deserialize_with="deserialize_percentage_float")]
    pub complete_percentage: f32,
}

#[cfg(test)]

mod test {
    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use serde_json::{Result as SerdeResult, from_str};

    use super::*;
    use crate::utility::mock_constants::BULK_VALIDATION_SUBMIT_OK;
    use crate::utility::mock_constants::BULK_VALIDATION_STATUS_OK;
    use crate::utility::mock_constants::BULK_VALIDATION_STATUS_DELETED;
    use crate::utility::mock_constants::BULK_VALIDATION_RESULT_DELETED;
    use crate::utility::mock_constants::BULK_VALIDATION_DELETE_OK;
    use crate::utility::mock_constants::FILE_FEEDBACK_SUPPORTED_VARIANT_1;
    use crate::utility::mock_constants::FILE_FEEDBACK_SUPPORTED_VARIANT_2;


    #[test]
    fn test_parsing_file_submit_response_ok() {
        let validation: SerdeResult<ZBFileFeedback> = from_str(BULK_VALIDATION_SUBMIT_OK);
        assert!(validation.is_ok());

        let validation_obj = validation.unwrap();
        assert_eq!(validation_obj.success, true);
        assert!(validation_obj.file_id.is_some());
        assert!(validation_obj.file_name.is_some());
        assert!(validation_obj.message.is_message(), "{:#?}", validation_obj.message);
    }

    #[test]
    fn test_parse_file_status_response_ok() {
        let file_status: SerdeResult<ZBFileStatus> = from_str(BULK_VALIDATION_STATUS_OK);
        assert!(file_status.is_ok());

        let expected_date_time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 4, 26).unwrap(),
            NaiveTime::from_hms_opt(17, 52, 23).unwrap(),
        );

        let file_status_obj = file_status.unwrap();
        assert_eq!(file_status_obj.success, true);
        assert_eq!(file_status_obj.complete_percentage, 100.);
        assert_eq!(file_status_obj.upload_date.naive_utc(), expected_date_time);
        assert!(file_status_obj.return_url.is_some());
        assert!(file_status_obj.error_reason.is_none());
    }

    #[test]
    fn test_parse_file_status_response_deleted() {
        let file_status: SerdeResult<ZBFileStatus> = from_str(BULK_VALIDATION_STATUS_DELETED);
        assert!(file_status.is_ok());

        let expected_date_time = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 4, 26).unwrap(),
            NaiveTime::from_hms_opt(17, 52, 23).unwrap(),
        );

        let file_status_obj = file_status.unwrap();
        assert_eq!(file_status_obj.success, true);
        assert_eq!(file_status_obj.complete_percentage, 0.);
        assert_eq!(file_status_obj.upload_date.naive_utc(), expected_date_time);
        assert!(file_status_obj.return_url.is_none());
        assert!(file_status_obj.error_reason.is_some());

    }

    #[test]
    fn test_parse_file_result_deleted() {
        let feedback: SerdeResult<ZBFileFeedback> = from_str(BULK_VALIDATION_RESULT_DELETED);
        assert!(feedback.is_ok());

        let feedback_obj = feedback.unwrap();
        assert_eq!(feedback_obj.success, false);
        assert!(feedback_obj.file_id.is_none());
        assert!(feedback_obj.file_name.is_none());
    }

    #[test]
    fn test_parse_file_delete_ok() {
        let feedback: SerdeResult<ZBFileFeedback> = from_str(BULK_VALIDATION_DELETE_OK);
        assert!(feedback.is_ok());

        let feedback_obj = feedback.unwrap();
        assert_eq!(feedback_obj.success, true);
        assert!(feedback_obj.message.is_message(), "{:#?}", feedback_obj.message);
        assert!(feedback_obj.file_id.is_some());
        assert!(feedback_obj.file_name.is_some());
    }

    #[test]
    fn test_parse_file_feedback_multiple_error_message() {
        let feedback: SerdeResult<ZBFileFeedback> = from_str(FILE_FEEDBACK_SUPPORTED_VARIANT_1);
        assert!(feedback.is_ok());

        let feedback_obj = feedback.unwrap();
        assert_eq!(feedback_obj.success, false);
        assert!(feedback_obj.message.are_multiple_messages(), "{:#?}", feedback_obj.message);

        if let ZBFeedbackMessage::MultipleMessages(messages) = feedback_obj.message {
            assert_eq!(messages.len(), 2);
        }
    }

    #[test]
    fn test_parse_file_feedback_unexpected_format() {
        let feedback: SerdeResult<ZBFileFeedback> = from_str(FILE_FEEDBACK_SUPPORTED_VARIANT_2);
        assert!(feedback.is_ok());

        let feedback_obj = feedback.unwrap();
        assert_eq!(feedback_obj.success, false);
        assert!(feedback_obj.message.is_unexpected(), "{:#?}", feedback_obj.message);
    }

}
