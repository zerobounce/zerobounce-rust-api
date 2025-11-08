pub mod api;
pub mod utility;

use std::collections::HashMap;

pub use crate::utility::{ZBError, ZBResult};
pub use crate::utility::structures::{ActivityData, ApiUsage};
pub use crate::utility::structures::bulk::{ZBFile, ZBFileFeedback, ZBFileStatus};
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
            url: crate::utility::URI.to_string(),
            bulk_url: crate::utility::BULK_URI.to_string(),
        }
    }
}

// Client offering methods for different API methods and functionalities
pub struct ZeroBounce {
    pub api_key: String,
    pub client: reqwest::blocking::Client,
    pub url_provider: ZBUrlProvider,
}

// More method implementations of this class can be found throughout
// the project.
impl ZeroBounce {
    pub fn new(api_key: &str) -> ZeroBounce {
        ZeroBounce {
            api_key: api_key.to_string().clone(),
            client: reqwest::blocking::Client::default(),
            url_provider: ZBUrlProvider::default(),
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
