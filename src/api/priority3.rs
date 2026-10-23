use std::collections::HashMap;
use std::io::Read;

use reqwest::blocking::multipart::{Form, Part};
use serde_json::{from_str, Value};

use crate::utility::structures::bulk::ZBFileFeedback;
use crate::utility::{ZBError, ZBResult};
use crate::ZeroBounce;

fn bulk_api_root(bulk_url: &str) -> String {
    let trimmed = bulk_url.trim_end_matches('/');
    trimmed.strip_suffix("/v2").unwrap_or(trimmed).to_string()
}

fn post_bulk_root_multipart(
    client: &ZeroBounce,
    url: String,
    file_name: &str,
    file_bytes: Vec<u8>,
    fields: HashMap<String, String>,
) -> ZBResult<ZBFileFeedback> {
    client.require_key()?;
    let mut form = Form::new()
        .part(
            "file",
            Part::bytes(file_bytes)
                .file_name(file_name.to_string())
                .mime_str("text/csv")?,
        )
        .text("api_key", client.api_key.clone());
    for (key, value) in fields {
        form = form.text(key, value);
    }
    let response = client.client.post(url).multipart(form).send()?;
    let body = client.read_api_response(response)?;
    crate::utility::http_error::parse_json_body(&body)
}

fn bulk_root_file_status(client: &ZeroBounce, base: &str, file_id: &str) -> ZBResult<String> {
    let url = format!(
        "{}/{}/filestatus",
        bulk_api_root(&client.url_provider.bulk_url),
        base
    );
    client.generic_get_request(url, HashMap::from([("file_id", file_id)]))
}

impl ZeroBounce {
    pub fn send_email_finder_file(
        &self,
        domain_column: u32,
        file_name: &str,
        mut reader: impl Read,
    ) -> ZBResult<ZBFileFeedback> {
        if domain_column == 0 {
            return Err(ZBError::explicit("empty parameter: domain_column"));
        }
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let url = format!(
            "{}/email-finder/sendfile",
            bulk_api_root(&self.url_provider.bulk_url)
        );
        post_bulk_root_multipart(
            self,
            url,
            file_name,
            bytes,
            HashMap::from([("domain_column".to_string(), domain_column.to_string())]),
        )
    }

    pub fn email_finder_file_status(&self, file_id: &str) -> ZBResult<String> {
        bulk_root_file_status(self, "email-finder", file_id)
    }

    pub fn domain_search_file_status(&self, file_id: &str) -> ZBResult<String> {
        bulk_root_file_status(self, "domain-search", file_id)
    }

    pub fn send_evaluated_file(
        &self,
        file_name: &str,
        mut reader: impl Read,
    ) -> ZBResult<ZBFileFeedback> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let url = format!(
            "{}/v2/send-evaluated-file/",
            bulk_api_root(&self.url_provider.bulk_url)
        );
        post_bulk_root_multipart(self, url, file_name, bytes, HashMap::new())
    }

    pub fn send_domain_search_file(
        &self,
        domain_column: u32,
        file_name: &str,
        mut reader: impl Read,
    ) -> ZBResult<ZBFileFeedback> {
        if domain_column == 0 {
            return Err(ZBError::explicit("empty parameter: domain_column"));
        }
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let url = format!(
            "{}/domain-search/sendfile",
            bulk_api_root(&self.url_provider.bulk_url)
        );
        post_bulk_root_multipart(
            self,
            url,
            file_name,
            bytes,
            HashMap::from([("domain_column".to_string(), domain_column.to_string())]),
        )
    }

    pub fn send_list_evaluator_file(
        &self,
        email_address_column: u32,
        file_name: &str,
        mut reader: impl Read,
    ) -> ZBResult<ZBFileFeedback> {
        if email_address_column == 0 {
            return Err(ZBError::explicit("empty parameter: email_address_column"));
        }
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let url = format!(
            "{}/",
            self.url_provider.list_evaluator_url
        );
        post_bulk_root_multipart(
            self,
            url,
            file_name,
            bytes,
            HashMap::from([(
                "email_address_column".to_string(),
                email_address_column.to_string(),
            )]),
        )
    }

    pub fn get_list_evaluator_result(&self, file_id: &str) -> ZBResult<Value> {
        if file_id.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: file_id"));
        }
        let url = format!(
            "{}/{}/",
            self.url_provider.list_evaluator_url,
            crate::utility::encode_path_segment(file_id)
        );
        let body = self.generic_get_request(url, HashMap::new())?;
        crate::utility::http_error::parse_json_body(&body)
    }

    pub fn add_filter(&self, rule: &str, target: &str, value: &str) -> ZBResult<Value> {
        let body = self.generic_form_post_request(
            self.url_provider.url_of("/filters/add"),
            HashMap::from([("rule", rule), ("target", target), ("value", value)]),
        )?;
        Ok(from_str(&body)?)
    }

    pub fn list_filters(&self, use_post: bool) -> ZBResult<Value> {
        let body = if use_post {
            self.generic_form_post_request(
                self.url_provider.url_of("/filters/list"),
                HashMap::new(),
            )?
        } else {
            self.generic_get_request(self.url_provider.url_of("/filters/list"), HashMap::new())?
        };
        Ok(from_str(&body)?)
    }

    pub fn delete_filter(&self, rule: &str, target: &str, value: &str) -> ZBResult<Value> {
        let body = self.generic_form_post_request(
            self.url_provider.url_of("/filters/delete"),
            HashMap::from([("rule", rule), ("target", target), ("value", value)]),
        )?;
        Ok(from_str(&body)?)
    }

}
