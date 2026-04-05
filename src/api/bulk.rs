use std::collections::HashMap;

use serde_json::from_str;

use crate::ZeroBounce;
use crate::utility::{ZBResult, ZBError};
use crate::utility::{ENDPOINT_FILE_SEND, ENDPOINT_FILE_STATUS, ENDPOINT_FILE_RESULT, ENDPOINT_FILE_DELETE};
use crate::utility::{ENDPOINT_SCORING_DELETE, ENDPOINT_SCORING_STATUS, ENDPOINT_SCORING_RESULT, ENDPOINT_SCORING_SEND};
use crate::utility::structures::bulk::{ZBBulkResponse, ZBFile, ZBFileFeedback, ZBFileStatus, ZBGetFileOptions};
use crate::utility::bulk_get_file::should_treat_get_file_body_as_error;
use crate::utility::format_get_file_error_message;


impl ZeroBounce {

    fn generic_file_submit(&self, endpoint: &str, zb_file: &ZBFile) -> ZBResult<ZBFileFeedback> {
        let mut multi_part_form = zb_file.generate_multipart()?
            .text("api_key", self.api_key.clone());

        if endpoint == ENDPOINT_FILE_SEND {
            if let Some(v) = zb_file.allow_phase_2 {
                multi_part_form = multi_part_form.text("allow_phase_2", v.to_string());
            }
        }

        let url = self.url_provider.bulk_url_of(endpoint);
        let response = self.client.post(url)
            .multipart(multi_part_form)
            .send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        // Debug: Print raw response to examine structure in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("Raw API response: {}", response_content);
        }

        if !response_ok {
            return Err(ZBError::ExplicitError(response_content));
        }

        let feedback_object = from_str::<ZBFileFeedback>(&response_content)?;
        Ok(feedback_object)
    }

    fn generic_file_status_check(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileStatus> {
        let query_args = HashMap::from([
            ("file_id", file_id),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.bulk_url_of(endpoint), query_args
        )?;

        let file_status = from_str::<ZBFileStatus>(&response_content)?;
        Ok(file_status)
    }

    fn generic_result_fetch(
        &self,
        endpoint: &str,
        file_id: &str,
        options: Option<&ZBGetFileOptions>,
        is_scoring: bool,
    ) -> ZBResult<ZBBulkResponse> {
        let url = self.url_provider.bulk_url_of(endpoint);
        let mut query: Vec<(&str, String)> = vec![
            ("api_key", self.api_key.clone()),
            ("file_id", file_id.to_string()),
        ];
        if let Some(opts) = options {
            if let Some(dt) = &opts.download_type {
                query.push(("download_type", dt.clone()));
            }
            if !is_scoring {
                if let Some(ad) = opts.activity_data {
                    query.push(("activity_data", if ad { "true".into() } else { "false".into() }));
                }
            }
        }

        let response = self.client.get(&url).query(&query).send()?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .ok_or_else(|| ZBError::explicit("content type not specified in response"))?
            .to_str()
            .map_err(|e| ZBError::ExplicitError(e.to_string()))?
            .to_string();

        let body_bytes = response.bytes()?;
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

        #[cfg(debug_assertions)]
        if !status.is_success() || should_treat_get_file_body_as_error(&body_str, &content_type) {
            eprintln!("Raw API response: {}", body_str);
        }

        if !status.is_success() {
            let msg = if body_str.trim_start().starts_with('{') {
                format_get_file_error_message(body_str.trim())
            } else if body_str.is_empty() {
                format!("HTTP {}", status.as_u16())
            } else {
                body_str
            };
            return Err(ZBError::ExplicitError(msg));
        }

        if should_treat_get_file_body_as_error(&body_str, &content_type) {
            return Err(ZBError::ExplicitError(format_get_file_error_message(body_str.trim())));
        }

        Ok(ZBBulkResponse::Content(body_bytes))
    }

    fn generic_result_delete(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileFeedback>{
        let query_args = HashMap::from([
            ("file_id", file_id),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.bulk_url_of(endpoint), query_args
        )?;

        let file_status = from_str::<ZBFileFeedback>(&response_content)?;
        Ok(file_status)
    }

    pub fn bulk_validation_file_submit(&self, zb_file: &ZBFile) -> ZBResult<ZBFileFeedback> {
        self.generic_file_submit(ENDPOINT_FILE_SEND, zb_file)
    }

    pub fn bulk_validation_file_status_check(&self, file_id: &str) -> ZBResult<ZBFileStatus> {
        self.generic_file_status_check(ENDPOINT_FILE_STATUS, file_id)
    }

    pub fn bulk_validation_result_fetch(&self, file_id: &str) -> ZBResult<ZBBulkResponse> {
        self.generic_result_fetch(ENDPOINT_FILE_RESULT, file_id, None, false)
    }

    /// Bulk validation getfile with optional v2 query parameters (`download_type`, `activity_data`).
    pub fn bulk_validation_result_fetch_with_options(
        &self,
        file_id: &str,
        options: &ZBGetFileOptions,
    ) -> ZBResult<ZBBulkResponse> {
        self.generic_result_fetch(ENDPOINT_FILE_RESULT, file_id, Some(options), false)
    }

    pub fn bulk_validation_result_delete(&self, file_id: &str) -> ZBResult<ZBFileFeedback> {
        self.generic_result_delete(ENDPOINT_FILE_DELETE, file_id)
    }

    pub fn ai_scoring_file_submit(&self, zb_file: &ZBFile) -> ZBResult<ZBFileFeedback> {
        self.generic_file_submit(ENDPOINT_SCORING_SEND, zb_file)
    }

    pub fn ai_scoring_file_status_check(&self, file_id: &str) -> ZBResult<ZBFileStatus> {
        self.generic_file_status_check(ENDPOINT_SCORING_STATUS, file_id)
    }

    pub fn ai_scoring_result_fetch(&self, file_id: &str) -> ZBResult<ZBBulkResponse> {
        self.generic_result_fetch(ENDPOINT_SCORING_RESULT, file_id, None, true)
    }

    /// AI scoring getfile with optional `download_type` (`activity_data` is not sent).
    pub fn ai_scoring_result_fetch_with_options(
        &self,
        file_id: &str,
        options: &ZBGetFileOptions,
    ) -> ZBResult<ZBBulkResponse> {
        self.generic_result_fetch(ENDPOINT_SCORING_RESULT, file_id, Some(options), true)
    }

    pub fn ai_scoring_result_delete(&self, file_id: &str) -> ZBResult<ZBFileFeedback> {
        self.generic_result_delete(ENDPOINT_SCORING_DELETE, file_id)
    }


}
