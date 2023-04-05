use mockito::{Matcher, ServerGuard, Server};
use zero_bounce::{ZBUrlProvider, ZeroBounce};

pub(crate) const EMAIL: &str = "example@example.com";
pub(crate) const MOCK_API_KEY: &str = "mock_api_key";
pub(crate) const INVALID_URL: &str = "http://255.255.255.255";
pub(crate) const SANDBOX_IP: &str = "99.110.204.1";

pub(crate) fn instantiate<'s>() -> (ServerGuard, ZeroBounce) {

    let mock_server = Server::new();
    let mock_url: String = mock_server.url().to_owned();

    let mock_url_provider = ZBUrlProvider {
        url: mock_url.clone(), bulk_url: mock_url.clone()
    };
    let zb_instance = ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: mock_url_provider,
    };
    (mock_server, zb_instance)
}

pub(crate) fn endpoint_matcher(endpoint: &str) -> Matcher {
    Matcher::Regex(endpoint.to_owned() + r#"(\?.*)?"#)
}

pub(crate) fn invalid_url_zb_instance() -> ZeroBounce {
    ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: ZBUrlProvider {
            url: INVALID_URL.to_owned(),
            bulk_url: INVALID_URL.to_owned(),
        },
    }
}