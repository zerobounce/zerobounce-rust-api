use chrono::NaiveDateTime;

use serde::Deserialize;

use crate::utility::structures::custom_deserialize::deserialize_naive_date;


#[derive(Clone, Debug, Deserialize)]
pub struct ZBValidation {
    pub address: String,
    pub status: String,
    pub sub_status: String,
    pub free_email: bool,
    pub did_you_mean: Option<String>,
    pub account: Option<String>,
    pub domain: Option<String>,
    pub domain_age_days: Option<String>,
    pub smtp_provider: Option<String>,
    pub mx_record: Option<String>,
    pub mx_found: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub gender: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub zipcode: Option<String>,

    #[serde(deserialize_with="deserialize_naive_date")]
    pub processed_at: NaiveDateTime,
}


#[derive(Clone, Debug, Deserialize)]
pub struct ZBBatchError {
    pub error: String,
    pub email_address: String,
}


#[derive(Clone, Debug, Deserialize)]
pub struct ZBBatchValidation {
    pub email_batch: Vec<ZBValidation>,
    pub errors: Vec<ZBBatchError>,
}

#[cfg(test)]
mod test {
    use serde_json::{Result as SerdeResult, from_str};

    use super::*;
    use crate::utility::mock_constants::VALIDATION_RESPONSE_VALID;
    use crate::utility::mock_constants::VALIDATION_RESPONSE_INVALID;
    use crate::utility::mock_constants::VALIDATION_RESPONSE_NULL_FIELDS;
    use crate::utility::mock_constants::BATCH_VALIDATION_WITH_ERROR;
    use crate::utility::mock_constants::BATCH_VALIDATION_ERROR_ONLY;
    use crate::utility::mock_constants::BATCH_VALIDATION_NO_ERROR;

    #[test]
    fn test_validation_invalid_json() {
        let validation_res: SerdeResult<ZBValidation> = from_str("");
        assert!(validation_res.is_err());
    }

    #[test]
    fn test_validation_missing_expected_fields() {
        let validation_res: SerdeResult<ZBValidation> = from_str(VALIDATION_RESPONSE_NULL_FIELDS);
        assert!(validation_res.is_ok());

        let validation = validation_res.unwrap();
        assert_eq!(validation.did_you_mean, None);
        assert_eq!(validation.domain_age_days, Some("".to_string()));
    }

    #[test]
    fn test_validation_invalid_email_status() {
        let validation_res: SerdeResult<ZBValidation> = from_str(VALIDATION_RESPONSE_INVALID);
        assert!(validation_res.is_ok());

        let validation = validation_res.unwrap();
        assert_eq!(validation.status, "invalid".to_string());
        assert_eq!(validation.sub_status, "mailbox_not_found".to_string());
        assert_eq!(validation.did_you_mean, None);
        assert_eq!(validation.smtp_provider, Some("example".to_string()));
        assert_eq!(validation.free_email, false);

        let expected_date = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2023, 3, 23).unwrap(),
            chrono::NaiveTime::from_hms_milli_opt(12, 30, 28, 3).unwrap(),
        );
        assert_eq!(validation.processed_at, expected_date);
    }

    #[test]
    fn test_validation_valid_email_status() {
        let validation_res: SerdeResult<ZBValidation> = from_str(VALIDATION_RESPONSE_VALID);
        assert!(validation_res.is_ok());

        let validation = validation_res.unwrap();
        assert_eq!(validation.status, "valid".to_string());
        assert_eq!(validation.sub_status, "".to_string());
        assert_eq!(validation.did_you_mean, None);
        assert_eq!(validation.smtp_provider, Some("example".to_string()));
        assert_eq!(validation.free_email, false);

        let expected_date = NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2023, 3, 23).unwrap(),
            chrono::NaiveTime::from_hms_milli_opt(13, 30, 28, 105).unwrap(),
        );
        assert_eq!(validation.processed_at, expected_date);
    }

    #[test]
    fn test_batch_error_only_content() {
        let batch: SerdeResult<ZBBatchValidation> = from_str(BATCH_VALIDATION_ERROR_ONLY);
        assert!(batch.is_ok());

        let batch_object = batch.unwrap();
        assert_eq!(batch_object.email_batch.len(), 0);
        assert_eq!(batch_object.errors.len(), 1);
    }

    #[test]
    fn test_batch_validation_and_error_content() {
        let batch: SerdeResult<ZBBatchValidation> = from_str(BATCH_VALIDATION_WITH_ERROR);
        assert!(batch.is_ok());

        let batch_object = batch.unwrap();
        assert_eq!(batch_object.email_batch.len(), 1);
        assert_eq!(batch_object.errors.len(), 1);
    }

    #[test]
    fn test_batch_validation_only_content() {
        let batch: SerdeResult<ZBBatchValidation> = from_str(BATCH_VALIDATION_NO_ERROR);
        assert!(batch.is_ok());

        let batch_object = batch.unwrap();
        assert_eq!(batch_object.email_batch.len(), 1);
        assert_eq!(batch_object.errors.len(), 0);
    }

}
