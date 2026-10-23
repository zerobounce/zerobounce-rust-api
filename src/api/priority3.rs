use std::collections::HashMap;
use std::io::Read;

use reqwest::blocking::multipart::{Form, Part};
use serde_json::{from_str, Value};

use serde::Deserialize;

use crate::utility::structures::bulk::ZBFileFeedback;
use crate::utility::{ZBError, ZBResult};
use crate::ZeroBounce;

fn bulk_api_root(bulk_url: &str) -> String {
    let trimmed = bulk_url.trim_end_matches('/');
    trimmed.strip_suffix("/v2").unwrap_or(trimmed).to_string()
}

fn post_bulk_root_form(
    client: &ZeroBounce,
    path: &str,
    fields: HashMap<String, String>,
) -> ZBResult<ZBFileFeedback> {
    client.require_key()?;
    let url = format!(
        "{}/{}",
        bulk_api_root(&client.url_provider.bulk_url),
        path.trim_start_matches('/')
    );
    let mut form: HashMap<&str, String> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    form.insert("api_key", client.api_key.clone());
    let response = client.client.post(url).form(&form).send()?;
    let body = client.read_api_response(response)?;
    crate::utility::http_error::parse_json_body(&body)
}

/// POST one of the documented S3 / SFTP file jobs on the bulkapi root.
///
/// `file_name` is optional only for `validateSFTPfile`; every other job requires it.
fn bulk_root_file_job(
    client: &ZeroBounce,
    path: &str,
    file_name: Option<&str>,
    has_header_row: bool,
    remove_duplicates: Option<bool>,
    return_url: Option<&str>,
    file_name_required: bool,
) -> ZBResult<ZBFileFeedback> {
    match file_name {
        None if file_name_required => {
            return Err(ZBError::explicit("empty parameter: file_name"));
        }
        Some(name) if name.trim().is_empty() => {
            return Err(ZBError::explicit("empty parameter: file_name"));
        }
        _ => {}
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    fields.insert("has_header_row".to_string(), has_header_row.to_string());
    if let Some(name) = file_name {
        fields.insert("file_name".to_string(), name.to_string());
    }
    if let Some(remove_duplicates) = remove_duplicates {
        fields.insert(
            "remove_duplicates".to_string(),
            remove_duplicates.to_string(),
        );
    }
    if let Some(return_url) = return_url {
        fields.insert("return_url".to_string(), return_url.to_string());
    }
    post_bulk_root_form(client, path, fields)
}

#[derive(Deserialize, Debug)]
pub struct S3ListFileEntry {
    pub name: String,
    pub size: i64,
    pub last_modified: String,
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

    pub fn list_s3_files(
        &self,
        bucket: Option<&str>,
        search: Option<&str>,
    ) -> ZBResult<Vec<S3ListFileEntry>> {
        let url = format!("{}/listS3files", bulk_api_root(&self.url_provider.bulk_url));
        let mut params: HashMap<&str, &str> = HashMap::new();
        if let Some(bucket) = bucket {
            if !bucket.trim().is_empty() {
                params.insert("bucket", bucket);
            }
        }
        if let Some(search) = search {
            if !search.trim().is_empty() {
                params.insert("search", search);
            }
        }
        let body = self.generic_get_request(url, params)?;
        Ok(from_str(&body)?)
    }

    pub fn validate_s3_file(&self, file_name: &str) -> ZBResult<ZBFileFeedback> {
        if file_name.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: file_name"));
        }
        post_bulk_root_form(
            self,
            "validateS3file",
            HashMap::from([("file_name".to_string(), file_name.to_string())]),
        )
    }

    pub fn score_s3_file(&self, file_name: &str) -> ZBResult<ZBFileFeedback> {
        if file_name.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: file_name"));
        }
        post_bulk_root_form(
            self,
            "scoreS3file",
            HashMap::from([("file_name".to_string(), file_name.to_string())]),
        )
    }

    pub fn export_s3_file(&self, file_id: &str, export_bucket: &str) -> ZBResult<ZBFileFeedback> {
        if file_id.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: file_id"));
        }
        if export_bucket.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: export_bucket"));
        }
        post_bulk_root_form(
            self,
            "exportS3file",
            HashMap::from([
                ("file_id".to_string(), file_id.to_string()),
                ("export_bucket".to_string(), export_bucket.to_string()),
            ]),
        )
    }

    /// Run the email finder on a file in your Amazon S3 bucket (POST /emailFinderS3file).
    ///
    /// `file_name` is the full path as returned by [`list_s3_files`](Self::list_s3_files).
    pub fn email_finder_s3_file(
        &self,
        file_name: &str,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "emailFinderS3file",
            Some(file_name),
            has_header_row,
            remove_duplicates,
            return_url,
            true,
        )
    }

    /// Run domain search on a file in your Amazon S3 bucket (POST /domainSearchS3file).
    pub fn domain_search_s3_file(
        &self,
        file_name: &str,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "domainSearchS3file",
            Some(file_name),
            has_header_row,
            remove_duplicates,
            return_url,
            true,
        )
    }

    // SFTP / FTP bulk. The server, path and protocol come from your ZeroBounce settings.

    /// Validate a file from your SFTP/FTP server (POST /validateSFTPfile).
    ///
    /// The docs list `file_name` as optional here alone; pass `None` to use the input
    /// bucket configured in your ZeroBounce settings.
    pub fn validate_sftp_file(
        &self,
        file_name: Option<&str>,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "validateSFTPfile",
            file_name,
            has_header_row,
            remove_duplicates,
            return_url,
            false,
        )
    }

    /// Score a file from your SFTP/FTP server (POST /scoreSFTPfile).
    ///
    /// `file_name` is a plain name such as `my_file.txt`.
    pub fn score_sftp_file(
        &self,
        file_name: &str,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "scoreSFTPfile",
            Some(file_name),
            has_header_row,
            remove_duplicates,
            return_url,
            true,
        )
    }

    /// Run the email finder on a file from your SFTP/FTP server (POST /emailFinderSFTPfile).
    pub fn email_finder_sftp_file(
        &self,
        file_name: &str,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "emailFinderSFTPfile",
            Some(file_name),
            has_header_row,
            remove_duplicates,
            return_url,
            true,
        )
    }

    /// Run domain search on a file from your SFTP/FTP server (POST /domainSearchSFTPfile).
    pub fn domain_search_sftp_file(
        &self,
        file_name: &str,
        has_header_row: bool,
        remove_duplicates: Option<bool>,
        return_url: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        bulk_root_file_job(
            self,
            "domainSearchSFTPfile",
            Some(file_name),
            has_header_row,
            remove_duplicates,
            return_url,
            true,
        )
    }

    /// Export a validated or scored file back to your SFTP/FTP server (POST /exportSFTPfile).
    ///
    /// Note the field name: this endpoint takes `export_path`, not the `export_bucket`
    /// that [`export_s3_file`](Self::export_s3_file) uses. Pass `None` to use the export
    /// path from your ZeroBounce settings.
    pub fn export_sftp_file(
        &self,
        file_id: &str,
        export_path: Option<&str>,
    ) -> ZBResult<ZBFileFeedback> {
        if file_id.trim().is_empty() {
            return Err(ZBError::explicit("empty parameter: file_id"));
        }
        let mut fields = HashMap::from([("file_id".to_string(), file_id.to_string())]);
        if let Some(export_path) = export_path {
            if export_path.trim().is_empty() {
                return Err(ZBError::explicit("empty parameter: export_path"));
            }
            fields.insert("export_path".to_string(), export_path.to_string());
        }
        post_bulk_root_form(self, "exportSFTPfile", fields)
    }
}
