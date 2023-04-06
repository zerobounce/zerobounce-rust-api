use std::collections::HashMap;

use serde_json::{Result as SerdeResult, from_str};

use crate::ZeroBounce;
use crate::utility::{ZBResult, ZBError};
use crate::utility::structures::bulk::{ZBBulkResponse, ZBFile, ZBFileFeedback, ZBFileStatus};




impl ZeroBounce {

    // TODO: have generic functions not be public
    pub fn generic_file_submit(&self, endpoint: &str, zb_file: &ZBFile) -> ZBResult<ZBFileFeedback> {
        let multi_part_form = zb_file.generate_multipart()?;
        let url = self.url_provider.url_of(endpoint);
        let response = self.client.post(url)
            .multipart(multi_part_form)
            .send()?;

        let response_content = response.text()?;

        let feedback_result: SerdeResult<ZBFileFeedback> = from_str(&response_content);
        if feedback_result.is_err() {
            return Err(ZBError::ExplicitError(response_content));
        }

        let feedback_object = feedback_result.unwrap();
        if !feedback_object.success {
            return Err(ZBError::ExplicitError(feedback_object.message.as_str()));
        }

        Ok(feedback_object)
    }

    pub fn generic_file_status_check(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileStatus> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.clone()),
            ("file_id", file_id.to_string()),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.bulk_url_of(endpoint), query_args
        )?;

        let file_status = from_str::<ZBFileStatus>(&response_content)?;
        Ok(file_status)
    }

    pub fn generic_result_fetch<'c>(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBBulkResponse> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.clone()),
            ("file_id", file_id.to_string()),
        ]);

        let url = self.url_provider.bulk_url_of(endpoint);
        let response = self.client
            .get(url)
            .query(&query_args)
            .send()?;

        // check if the response is a file, based on the content type
        let content_type = response
            .headers()
            .get("Content-Type")
            .ok_or(ZBError::explicit("content type not specified in response"))?
            .to_str()
            .map_err(|error| ZBError::ExplicitError(error.to_string()))?;

        // return content type
        if response.status().is_success() && content_type == "application/octet-stream" {
            let content = response.bytes()?;
            return Ok(ZBBulkResponse::Content(content));
        }

        // error, either json format (file feedback) or unknown
        let response_content = response.text()?;
        let feedback = from_str::<ZBFileFeedback>(&response_content);

        if feedback.is_ok() {
            return Ok(ZBBulkResponse::Feedback(feedback.unwrap()));
        }

        Err(ZBError::ExplicitError(response_content))
    }

    pub fn generic_result_delete(&self, endpoint: &str, file_id: &str) -> ZBResult<ZBFileFeedback>{
        let query_args = HashMap::from([
            ("api_key", self.api_key.clone()),
            ("file_id", file_id.to_string()),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.bulk_url_of(endpoint), query_args
        )?;

        let file_status = from_str::<ZBFileFeedback>(&response_content)?;
        Ok(file_status)
    }

}