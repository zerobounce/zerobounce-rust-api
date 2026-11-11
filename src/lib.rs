pub mod api;
pub mod utility;
pub mod warmup;

use std::collections::HashMap;

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

/// The base URLs requests are built from.
///
/// Start from [`ZBUrlProvider::default()`] (the live hosts) and assign the fields
/// you want to change. The struct is `#[non_exhaustive]`, so it cannot be built
/// with a struct literal outside this crate and a base URL added later is not a
/// breaking change.
///
/// Every request checks its final URL: it must be `https://`, or `http://` with
/// the exact host `127.0.0.1`, `localhost` or `[::1]` (for local tests). Any
/// other URL fails with an error before a connection is opened, so the API key
/// is never sent in cleartext.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ZBUrlProvider {
    pub url: String,
    pub bulk_url: String,
    /// Overridable; the list evaluator path differs from the other bulk endpoints.
    pub list_evaluator_url: String,
    /// Overridable; the Inbox Placement Tester is not under the bulk /v2 path.
    pub inbox_placement_url: String,
}

/// `base` and `endpoint` joined by exactly one slash. The built-in base URLs
/// end in `/v2/` and every endpoint starts with `/`, which used to give
/// `/v2//validate`; trailing slashes on any base URL are now dropped.
fn join_url(base: &str, endpoint: &str) -> String {
    let base = base.trim_end_matches('/');
    if endpoint.is_empty() || endpoint.starts_with('/') {
        format!("{base}{endpoint}")
    } else {
        format!("{base}/{endpoint}")
    }
}

impl ZBUrlProvider {
    pub fn url_of(&self, endpoint: &str) -> String {
        join_url(&self.url, endpoint)
    }
    pub fn bulk_url_of(&self, endpoint: &str) -> String {
        join_url(&self.bulk_url, endpoint)
    }
    pub fn list_evaluator_url_of(&self, endpoint: &str) -> String {
        join_url(&self.list_evaluator_url, endpoint)
    }
    pub fn inbox_placement_url_of(&self, endpoint: &str) -> String {
        join_url(&self.inbox_placement_url, endpoint)
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

/// Names the type and every field, with the API key shown as `***`.
impl std::fmt::Debug for ZeroBounce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZeroBounce")
            .field("api_key", &utility::redacted_key(&self.api_key))
            .field("base_url", &self.base_url)
            .field("client", &self.client)
            .field("url_provider", &self.url_provider)
            .finish()
    }
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
    ///
    /// # Panics
    /// When the HTTP client cannot be built (for example, the TLS backend
    /// fails to initialise), as `reqwest::blocking::Client::new()` does.
    /// [`with_base_url`](Self::with_base_url) returns that as an error instead.
    pub fn new(api_key: &str) -> ZeroBounce {
        Self::build(api_key, ApiBaseUrl::Default.as_str().to_string())
            .unwrap_or_else(|error| panic!("ZeroBounce::new: {error}"))
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
    /// # Errors
    /// Returns [`ZBError::ExplicitError`] when `base_url` is not an `https://`
    /// URL. `http://` is accepted only for the exact hosts `127.0.0.1`,
    /// `localhost` and `[::1]`, for local tests. Also when the HTTP client
    /// cannot be built.
    ///
    /// # Example
    /// ```no_run
    /// use zero_bounce::{ZeroBounce, ApiBaseUrl};
    ///
    /// # fn main() -> Result<(), zero_bounce::ZBError> {
    /// // Using enum
    /// let zb = ZeroBounce::with_base_url("your_api_key", ApiBaseUrl::USA)?;
    ///
    /// // Using custom string
    /// let zb = ZeroBounce::with_base_url("your_api_key", "https://custom-api.example.com/v2/")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_base_url<T>(api_key: &str, base_url: T) -> ZBResult<ZeroBounce>
    where
        T: Into<String>,
    {
        let base_url_string = base_url.into();
        parse_request_url(&base_url_string)
            .map_err(|_| ZBError::explicit("base_url must be an https:// URL"))?;
        Self::build(api_key, base_url_string)
    }

    /// The client both constructors share. Only [`with_base_url`](Self::with_base_url)
    /// takes a caller's URL, and it checks it first.
    fn build(api_key: &str, base_url_string: String) -> ZBResult<ZeroBounce> {
        let url_provider = ZBUrlProvider {
            url: base_url_string.clone(),
            ..ZBUrlProvider::default()
        };

        Ok(ZeroBounce {
            api_key: api_key.to_string(),
            base_url: base_url_string,
            client: utility::http_client()?,
            url_provider,
        })
    }

    fn require_key(&self) -> ZBResult<()> {
        if self.api_key.trim().is_empty() {
            return Err(ZBError::explicit("api_key is required"));
        }
        utility::check_key_text(&self.api_key)
    }

    fn read_api_response(&self, response: reqwest::blocking::Response) -> ZBResult<String> {
        self.read_api_response_checked(response, false)
    }

    fn read_api_response_checked(
        &self,
        response: reqwest::blocking::Response,
        message_only_is_failure: bool,
    ) -> ZBResult<String> {
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
        if message_only_is_failure {
            utility::http_error::reject_message_only_body(status, &content_type, &response_content)?;
        }
        Ok(response_content)
    }

    fn generic_get_request<'a>(
        &'a self,
        url: String,
        query_args: HashMap<&'a str, &'a str>,
    ) -> ZBResult<String> {
        self.get_request_checked(url, query_args, false)
    }

    /// `generic_get_request` for a call whose result is never a bare message
    /// (the email finder and domain search), so a message-only body is an error.
    fn finder_get_request<'a>(
        &'a self,
        url: String,
        query_args: HashMap<&'a str, &'a str>,
    ) -> ZBResult<String> {
        self.get_request_checked(url, query_args, true)
    }

    fn get_request_checked<'a>(
        &'a self,
        url: String,
        mut query_args: HashMap<&'a str, &'a str>,
        message_only_is_failure: bool,
    ) -> ZBResult<String> {
        self.require_key()?;
        query_args.insert("api_key", self.api_key.as_str());

        let response = self
            .client
            .get(parse_request_url(&url)?)
            .query(&query_args)
            .send()?;
        self.read_api_response_checked(response, message_only_is_failure)
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

/// Parse a request URL and enforce the transport rule every request follows:
/// `https://`, or `http://` only for the exact loopback hosts `127.0.0.1`,
/// `localhost` and `[::1]`. Every request path calls this on its final URL,
/// so an `http://` override on [`ZBUrlProvider`] can never put the API key on
/// the wire in cleartext.
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
        "request URL must use https:// (http:// is allowed only for 127.0.0.1, localhost and [::1])"
            .into(),
    ))
}

#[cfg(test)]
mod security_tests {
    use super::{parse_request_url, ZeroBounce};

    #[test]
    fn accepts_https() {
        let zb = ZeroBounce::with_base_url("key", "https://api.zerobounce.net/v2/").unwrap();
        assert_eq!(zb.base_url, "https://api.zerobounce.net/v2/");
        assert_eq!(zb.url_provider.url, "https://api.zerobounce.net/v2/");
    }

    #[test]
    fn accepts_loopback_http() {
        for url in [
            "http://127.0.0.1:1234/",
            "http://localhost:1234/",
            "http://[::1]:1234/",
        ] {
            let zb = ZeroBounce::with_base_url("key", url).unwrap();
            assert_eq!(zb.base_url, url);
        }
    }

    fn rejected(url: &str) -> String {
        match ZeroBounce::with_base_url("key", url) {
            Ok(_) => panic!("{url} was accepted"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn rejects_remote_http() {
        assert!(rejected("http://evil.example/v2/").contains("https://"));
    }

    #[test]
    fn rejects_loopback_lookalike_host() {
        assert!(rejected("http://localhost.evil.example/v2/").contains("https://"));
    }

    #[test]
    fn rejects_loopback_userinfo_trick() {
        assert!(rejected("http://127.0.0.1@evil.example/v2/").contains("https://"));
    }

    #[test]
    fn rejects_other_loopback_spellings_and_garbage() {
        for url in ["http://127.0.0.2/v2/", "http://0.0.0.0/v2/", "ftp://127.0.0.1/", "not a url"] {
            assert!(rejected(url).contains("https://"), "{url}");
        }
    }

    #[test]
    fn request_urls_follow_the_same_rule() {
        assert!(parse_request_url("https://bulkapi.zerobounce.net/v2/sendfile").is_ok());
        assert!(parse_request_url("http://localhost:8080/v2/getfile").is_ok());
        let error = parse_request_url("http://bulkapi.zerobounce.net/v2/sendfile").unwrap_err();
        assert!(error.to_string().contains("https://"), "{error}");
    }
}
