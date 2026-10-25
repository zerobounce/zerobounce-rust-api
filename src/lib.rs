pub mod api;
pub mod utility;
pub mod warmup;

use std::collections::HashMap;
use std::time::Duration;

pub use crate::api::{DomainSearchV2Builder, FindEmailV2Builder};
pub use crate::utility::structures::bulk::{
    zb_download_type, ZBBulkResponse, ZBEmailFinderFileOptions, ZBFile, ZBFileFeedback,
    ZBFileStatus, ZBGetFileOptions,
};
pub use crate::utility::structures::validate_enums::{ZBValidateStatus, ZBValidateSubStatus};
pub use crate::utility::structures::{ActivityData, ApiUsage};
pub use crate::utility::{format_get_file_error_message, get_file_json_indicates_error};
pub use crate::utility::{ApiBaseUrl, ZBError, ZBResult};
pub use crate::warmup::{EmailContent, ManagedSending, OnboardingModel, ZeroBounceWarmup};

// Structure meant to generate the URLs to be accessed with the HTTP requests
// based on the base API URLs (for the base API and bulk API).
pub struct ZBUrlProvider {
    pub url: String,
    pub bulk_url: String,
    /// Overridable; the list evaluator path differs from the other bulk endpoints.
    pub list_evaluator_url: String,
    /// Overridable; the Inbox Placement Tester is not under the bulk /v2 path.
    pub inbox_placement_url: String,
}

impl ZBUrlProvider {
    pub fn url_of(&self, endpoint: &str) -> String {
        self.url.to_owned() + endpoint
    }
    pub fn bulk_url_of(&self, endpoint: &str) -> String {
        self.bulk_url.to_owned() + endpoint
    }
    pub fn list_evaluator_url_of(&self, endpoint: &str) -> String {
        self.list_evaluator_url.to_owned() + endpoint
    }
    pub fn inbox_placement_url_of(&self, endpoint: &str) -> String {
        self.inbox_placement_url.to_owned() + endpoint
    }
}

impl Default for ZBUrlProvider {
    fn default() -> Self {
        ZBUrlProvider {
            url: ApiBaseUrl::Default.as_str().to_string(),
            bulk_url: crate::utility::BULK_URI.to_string(),
            list_evaluator_url: crate::utility::LIST_EVALUATOR_URI.to_string(),
            inbox_placement_url: crate::utility::INBOX_PLACEMENT_URI.to_string(),
        }
    }
}

// Client offering methods for different API methods and functionalities
pub struct ZeroBounce {
    pub api_key: String,
    pub base_url: String,
    pub client: reqwest::blocking::Client,
    pub url_provider: ZBUrlProvider,
}

// More method implementations of this class can be found throughout
// the project.
impl ZeroBounce {
    /// Create a new ZeroBounce client instance with the default API URL.
    ///
    /// # Arguments
    /// * `api_key` - Your ZeroBounce API key
    ///
    /// # Example
    /// ```no_run
    /// use zero_bounce::ZeroBounce;
    ///
    /// let zb = ZeroBounce::new("your_api_key");
    /// ```
    pub fn new(api_key: &str) -> ZeroBounce {
        Self::with_base_url(api_key, ApiBaseUrl::Default)
    }

    /// Create a new ZeroBounce client instance with a custom base URL.
    ///
    /// # Arguments
    /// * `api_key` - Your ZeroBounce API key
    /// * `base_url` - Base URL. Can be:
    ///   - `ApiBaseUrl::Default` - Uses the default API URL
    ///   - `ApiBaseUrl::USA` - Uses the USA API URL
    ///   - `ApiBaseUrl::EU` - Uses the EU API URL
    ///   - `String` or `&str` - Uses a custom URL string
    ///
    /// # Example
    /// ```no_run
    /// use zero_bounce::{ZeroBounce, ApiBaseUrl};
    ///
    /// // Using enum
    /// let zb = ZeroBounce::with_base_url("your_api_key", ApiBaseUrl::USA);
    ///
    /// // Using custom string
    /// let zb = ZeroBounce::with_base_url("your_api_key", "https://custom-api.example.com/v2/");
    /// ```
    pub fn with_base_url<T>(api_key: &str, base_url: T) -> ZeroBounce
    where
        T: Into<String>,
    {
        let base_url_string = require_https_url(base_url.into());

        let url_provider = ZBUrlProvider {
            url: base_url_string.clone(),
            ..ZBUrlProvider::default()
        };

        ZeroBounce {
            api_key: api_key.to_string(),
            base_url: base_url_string,
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::default()),
            url_provider,
        }
    }

    fn require_key(&self) -> ZBResult<()> {
        if self.api_key.trim().is_empty() {
            return Err(ZBError::explicit("api_key is required"));
        }
        Ok(())
    }

    fn read_api_response(&self, response: reqwest::blocking::Response) -> ZBResult<String> {
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = utility::read_limited(
            response,
            utility::MAX_JSON_RESPONSE_BYTES,
            "response body exceeds size limit",
        )?;
        let response_content = String::from_utf8_lossy(&body).into_owned();
        utility::http_error::ensure_json_api_body(status, &content_type, &response_content)?;
        Ok(response_content)
    }

    fn generic_get_request<'a>(
        &'a self,
        url: String,
        mut query_args: HashMap<&'a str, &'a str>,
    ) -> ZBResult<String> {
        self.require_key()?;
        query_args.insert("api_key", self.api_key.as_str());

        let response = self
            .client
            .get(parse_request_url(&url)?)
            .query(&query_args)
            .send()?;
        self.read_api_response(response)
    }

    fn generic_form_post_request<'a>(
        &'a self,
        url: String,
        mut form_args: HashMap<&'a str, &'a str>,
    ) -> ZBResult<String> {
        self.require_key()?;
        form_args.insert("api_key", self.api_key.as_str());

        let response = self
            .client
            .post(parse_request_url(&url)?)
            .form(&form_args)
            .send()?;
        self.read_api_response(response)
    }
}

pub(crate) fn parse_request_url(url: &str) -> ZBResult<reqwest::Url> {
    let parsed = reqwest::Url::parse(url).map_err(|e| ZBError::ExplicitError(e.to_string()))?;
    if parsed.scheme() == "https" {
        return Ok(parsed);
    }
    if parsed.scheme() == "http" {
        match parsed.host_str() {
            Some("127.0.0.1") | Some("localhost") | Some("[::1]") => return Ok(parsed),
            _ => {}
        }
    }
    Err(ZBError::ExplicitError(
        "request URL must use https://".into(),
    ))
}

fn require_https_url(url: String) -> String {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return url;
    }
    let is_loopback_http = reqwest::Url::parse(&url)
        .map(|parsed| {
            parsed.scheme() == "http"
                && matches!(
                    parsed.host_str(),
                    Some("127.0.0.1") | Some("localhost") | Some("[::1]")
                )
        })
        .unwrap_or(false);
    if is_loopback_http {
        return url;
    }
    panic!("base_url must be an https:// URL");
}

#[cfg(test)]
mod security_tests {
    use super::require_https_url;

    #[test]
    fn accepts_https() {
        assert_eq!(
            require_https_url("https://api.zerobounce.net/v2/".to_string()),
            "https://api.zerobounce.net/v2/"
        );
    }

    #[test]
    fn accepts_loopback_http() {
        assert_eq!(
            require_https_url("http://127.0.0.1:1234/".to_string()),
            "http://127.0.0.1:1234/"
        );
    }

    #[test]
    #[should_panic(expected = "https://")]
    fn rejects_remote_http() {
        require_https_url("http://evil.example/v2/".to_string());
    }

    #[test]
    #[should_panic(expected = "https://")]
    fn rejects_loopback_lookalike_host() {
        require_https_url("http://localhost.evil.example/v2/".to_string());
    }

    #[test]
    #[should_panic(expected = "https://")]
    fn rejects_loopback_userinfo_trick() {
        require_https_url("http://127.0.0.1@evil.example/v2/".to_string());
    }
}
