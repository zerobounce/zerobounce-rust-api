pub mod api;
pub mod utility;

use std::collections::HashMap;

use crate::utility::{ZBError, ZBResult};

// Structure meant to generate the URLs to be accessed with the HTTP requests
// based on the base API URLs (for the base API and bulk API).
pub struct ZBUrlProvider {
    pub url: String,
    pub bulk_url: String,
}

impl ZBUrlProvider {
    pub fn url_of(&self, endpoint: &str) -> String {
        return self.url.to_owned() + endpoint;
    }
    pub fn bulk_url_of(&self, endpoint: &str) -> String {
        return self.bulk_url.to_owned() + endpoint;
    }
}

impl Default for ZBUrlProvider {
    fn default() -> Self {
        ZBUrlProvider {
            url: crate::utility::URI.clone().to_string(),
            bulk_url: crate::utility::BULK_URI.clone().to_string(),
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

    fn generic_get_request(&self,url: String,query_args: HashMap<&str, &str>,) -> ZBResult<String> {
        let response = self.client.get(url).query(&query_args).send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        if !response_ok {
            return Err(ZBError::ExplicitError(response_content));
        }

        Ok(response_content)
    }
}
