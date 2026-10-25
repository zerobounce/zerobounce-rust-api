use std::collections::HashMap;

use crate::utility::bulk_get_file::should_treat_get_file_body_as_error;
use crate::utility::format_get_file_error_message;
use crate::utility::structures::bulk::{
    ZBBulkResponse, ZBFile, ZBFileFeedback, ZBFileStatus, ZBGetFileOptions,
};
use crate::utility::{ZBError, ZBResult};
use crate::utility::{
    ENDPOINT_FILE_DELETE, ENDPOINT_FILE_RESULT, ENDPOINT_FILE_SEND, ENDPOINT_FILE_STATUS,
};
use crate::utility::{
    ENDPOINT_SCORING_DELETE, ENDPOINT_SCORING_RESULT, ENDPOINT_SCORING_SEND,
    ENDPOINT_SCORING_STATUS,
};
use crate::ZeroBounce;

impl ZeroBounce {
    fn generic_file_submit(&self, endpoint: &str, zb_file: &ZBFile) -> ZBResult<ZBFileFeedback> {
        self.require_key()?;
        let include_person_columns = endpoint == ENDPOINT_FILE_SEND;
        let mut multi_part_form = zb_file
            .generate_multipart_with(include_person_columns)?
            .text("api_key", self.api_key.clone());

        if endpoint == ENDPOINT_FILE_SEND {
            if let Some(v) = zb_file.allow_phase_2 {
                multi_part_form = multi_part_form.text("allow_phase_2", v.to_string());
            }
        }

        let url = self.url_provider.bulk_url_of(endpoint);
        let response = self.client.post(url).multipart(multi_part_form).send()?;

        let response_content = self.read_api_response(response)?;
        crate::utility::http_error::parse_json_body(&response_content)
    }

    fn generic_file_status_check(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileStatus> {
        let query_args = HashMap::from([("file_id", file_id)]);

        let response_content =
            self.generic_get_request(self.url_provider.bulk_url_of(endpoint), query_args)?;

        crate::utility::http_error::parse_json_body(&response_content)
    }

    /// Bulk `getfile` against a full URL.
    ///
    /// The bulk email finder and domain search live on the bulk host without the
    /// `/v2` path, so they build their own URL and share this body.
    pub(crate) fn result_fetch_at(
        &self,
        url: String,
        file_id: &str,
        options: Option<&ZBGetFileOptions>,
        is_scoring: bool,
    ) -> ZBResult<ZBBulkResponse> {
        self.require_key()?;
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
                    query.push((
                        "activity_data",
                        if ad { "true".into() } else { "false".into() },
                    ));
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

        let body_bytes = crate::utility::read_limited(
            response,
            crate::utility::MAX_GETFILE_BYTES,
            "getfile response exceeds size limit",
        )?;
        let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

        if crate::utility::http_error::body_looks_like_html(&body_str)
            || crate::utility::http_error::content_type_is_html(&content_type)
            || !status.is_success()
        {
            return Err(ZBError::ExplicitError(
                crate::utility::http_error::message_from_body(
                    status.as_u16(),
                    &content_type,
                    &body_str,
                ),
            ));
        }

        if should_treat_get_file_body_as_error(&body_str, &content_type) {
            let message = crate::utility::http_error::strip_error_prefix(
                &format_get_file_error_message(body_str.trim()),
            );
            if message.is_empty()
                || message.len() > 500
                || crate::utility::http_error::body_looks_like_html(&message)
            {
                return Err(ZBError::ExplicitError(
                    crate::utility::http_error::message_from_body(
                        status.as_u16(),
                        &content_type,
                        &body_str,
                    ),
                ));
            }
            return Err(ZBError::ExplicitError(message));
        }

        Ok(ZBBulkResponse::Content(body_bytes.into()))
    }

    fn generic_result_fetch(
        &self,
        endpoint: &str,
        file_id: &str,
        options: Option<&ZBGetFileOptions>,
        is_scoring: bool,
    ) -> ZBResult<ZBBulkResponse> {
        self.result_fetch_at(
            self.url_provider.bulk_url_of(endpoint),
            file_id,
            options,
            is_scoring,
        )
    }

    /// Bulk `deletefile` against a full URL. See [`result_fetch_at`](Self::result_fetch_at).
    pub(crate) fn result_delete_at(&self, url: String, file_id: &str) -> ZBResult<ZBFileFeedback> {
        let query_args = HashMap::from([("file_id", file_id)]);

        let response_content = self.generic_get_request(url, query_args)?;

        crate::utility::http_error::parse_json_body(&response_content)
    }

    fn generic_result_delete(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileFeedback> {
        self.result_delete_at(self.url_provider.bulk_url_of(endpoint), file_id)
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
