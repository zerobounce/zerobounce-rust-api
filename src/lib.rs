pub mod api;
pub mod utility;

use std::collections::HashMap;
use std::time::Duration;

pub use crate::utility::{ZBError, ZBResult, ApiBaseUrl};
pub use crate::utility::structures::{ActivityData, ApiUsage};
pub use crate::utility::structures::bulk::{
    zb_download_type, ZBFile, ZBFileFeedback, ZBFileStatus, ZBBulkResponse, ZBGetFileOptions,
};
pub use crate::utility::{format_get_file_error_message, get_file_json_indicates_error};
pub use crate::utility::structures::validate_enums::{ZBValidateStatus, ZBValidateSubStatus};
pub use crate::api::{FindEmailV2Builder, DomainSearchV2Builder};

// Structure meant to generate the URLs to be accessed with the HTTP requests
// based on the base API URLs (for the base API and bulk API).
pub struct ZBUrlProvider {
    pub url: String,
    pub bulk_url: String,
}

impl ZBUrlProvider {
    pub fn url_of(&self, endpoint: &str) -> String {
        self.url.to_owned() + endpoint
    }
    pub fn bulk_url_of(&self, endpoint: &str) -> String {
        self.bulk_url.to_owned() + endpoint
    }
}

impl Default for ZBUrlProvider {
    fn default() -> Self {
        ZBUrlProvider {
            url: ApiBaseUrl::Default.as_str().to_string(),
            bulk_url: crate::utility::BULK_URI.to_string(),
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
        
        let mut url_provider = ZBUrlProvider::default();
        url_provider.url = base_url_string.clone();
        
        ZeroBounce {
            api_key: api_key.to_string(),
            base_url: base_url_string,
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::default()),
            url_provider,
        }
    }

    fn generic_get_request<'a>(&'a self, url: String, mut query_args: HashMap<&'a str, &'a str>) -> ZBResult<String> {
        // Automatically add api_key to query arguments
        query_args.insert("api_key", self.api_key.as_str());
        
        let response = self.client.get(url).query(&query_args).send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        if !response_ok {
            return Err(ZBError::ExplicitError(response_content));
        }

        Ok(response_content)
    }
}

fn require_https_url(url: String) -> String {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") {
        return url;
    }
    // mockito and local tests use loopback HTTP
    if lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
    {
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
}
