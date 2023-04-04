pub mod api;
pub mod utility;


// Structure meant to generate the URLs to be accessed with the HTTP requests
// based on the base API URLs (for the base API and bulk API).
pub struct ZBUrlProvider{
    pub url: String,
    pub bulk_url: String,
}

impl ZBUrlProvider {
    pub fn url_of(&self, endpoint: &str) -> String {
        return self.url.to_owned() + endpoint;
    }
}

impl Default for ZBUrlProvider {
    fn default() -> Self {
        ZBUrlProvider{
            url: crate::utility::URI.clone().to_string(),
            bulk_url: crate::utility::BULK_URI.clone().to_string(),
        }
    }
}

pub struct ZeroBounce {
    pub api_key: String,
    pub client: reqwest::blocking::Client,
    pub url_provider: ZBUrlProvider,
}
