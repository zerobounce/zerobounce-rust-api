use std::fmt::Debug;

use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use reqwest::blocking::multipart::{Form, Part};

use serde::Deserialize;

use crate::utility::{ZBResult, ZBError};
use crate::utility::structures::custom_deserialize::deserialize_date_rfc;
use crate::utility::structures::custom_deserialize::deserialize_percentage_float;


#[derive(Clone, Debug, Deserialize)]
pub struct ZBFileFeedback {
    pub success: bool,
    pub message: String,
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

pub enum ZBBulkResponse {
    Content(Bytes),
    Feedback(ZBFileFeedback),
}

impl Debug for ZBBulkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Content(cnt) => {
                write!(f, "<ZBBulkResponse::Content | size {}>", cnt.len())
            },
            Self::Feedback(feedback) => {
                write!(f, "<ZBBulkResponse::Feedback | {:#?}>", feedback)
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ZBFileContentType {
    FilePath(String),
    RawContent(Vec<u8>),
    Empty,
}

impl ZBFileContentType {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::RawContent(vec) => vec.len() == 0,
            _ => false,
        }
    }
}

pub struct ZBFile {
    content_type: ZBFileContentType,
    has_header_row: bool,
    remove_duplicate: bool,
    email_address_column: u32,
    first_name_column: Option<u32>,
    last_name_column: Option<u32>,
    gender_column: Option<u32>,
    ip_address_column: Option<u32>,
}

impl Default for ZBFile {
    fn default() -> Self {
        ZBFile {
            content_type: ZBFileContentType::Empty,
            has_header_row: true,
            remove_duplicate: false,
            email_address_column: 1,
            first_name_column: None,
            last_name_column: None,
            gender_column: None,
            ip_address_column: None,
        }
    }
}

impl ZBFile {

    pub fn from_path(path_to_file: String) -> ZBFile {
        let mut file = ZBFile::default();
        file.content_type = ZBFileContentType::FilePath(path_to_file);
        file
    }

    pub fn from_content(content: Vec<u8>) -> ZBFile {
        let mut file = ZBFile::default();
        file.content_type = ZBFileContentType::RawContent(content);
        file
    }

    fn file_content_multipart(&self) -> ZBResult<Part> {
        match self.content_type.clone() {
            ZBFileContentType::Empty => Err(ZBError::explicit("bulk content cannot be empty")),
            ZBFileContentType::FilePath(file_path) => Ok(
                Part::file(file_path.clone())?
            ),
            ZBFileContentType::RawContent(value) => Ok(
                Part::bytes(value.clone())
                    .file_name("file.csv")
                    .mime_str("text/csv")?
            ),
        }
    }

    pub fn generate_multipart(&self) -> ZBResult<Form> {
        let content_part = self.file_content_multipart()?;
        let mut multipart_form = Form::new()
            .part("file", content_part)
            .text("has_header_row", self.has_header_row.to_string())
            .text("remove_duplicate", self.remove_duplicate.to_string())
            .text("email_address_column", self.email_address_column.to_string());

        if let Some(amount) = self.first_name_column {
            multipart_form = multipart_form.text("first_name_column", amount.to_string());
        }
        if let Some(amount) = self.last_name_column {
            multipart_form = multipart_form.text("last_name_column", amount.to_string());
        }
        if let Some(amount) = self.gender_column {
            multipart_form = multipart_form.text("gender_column", amount.to_string());
        }
        if let Some(amount) = self.ip_address_column {
            multipart_form = multipart_form.text("ip_address_column", amount.to_string());
        }

        Ok(multipart_form)
    }

    pub fn set_has_header_row(mut self, has_header_row: bool) -> Self {
        self.has_header_row = has_header_row;
        self
    }

    pub fn set_remove_duplicate(mut self, remove_duplicate: bool) -> Self {
        self.remove_duplicate = remove_duplicate;
        self
    }

    pub fn set_email_address_column(mut self, email_address_column: u32) -> Self {
        self.email_address_column = email_address_column;
        self
    }

    pub fn set_first_name_column(mut self, first_name_column: Option<u32>) -> Self {
        self.first_name_column = first_name_column;
        self
    }

    pub fn set_last_name_column(mut self, last_name_column: Option<u32>) -> Self {
        self.last_name_column = last_name_column;
        self
    }

    pub fn set_gender_column(mut self, gender_column: Option<u32>) -> Self {
        self.gender_column = gender_column;
        self
    }

    pub fn set_ip_address_column(mut self, ip_address_column: Option<u32>) -> Self {
        self.ip_address_column = ip_address_column;
        self
    }
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


    #[test]
    fn test_parsing_file_submit_response_ok() {
        let validation: SerdeResult<ZBFileFeedback> = from_str(BULK_VALIDATION_SUBMIT_OK);
        assert!(validation.is_ok());

        let validation_obj = validation.unwrap();
        assert_eq!(validation_obj.success, true);
        assert!(validation_obj.file_id.is_some());
        assert!(validation_obj.file_name.is_some());
        assert_eq!(validation_obj.message, "File Accepted");
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
        assert!(feedback_obj.file_id.is_some());
        assert!(feedback_obj.file_name.is_some());
        assert_eq!(feedback_obj.message, "File Deleted");
    }

}
