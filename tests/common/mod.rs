use mockito::{Matcher, ServerGuard, Server};
use zero_bounce::{ZBUrlProvider, ZeroBounce, ZBFile};

pub(crate) const EMAIL: &str = "example@example.com";
pub(crate) const MOCK_API_KEY: &str = "mock_api_key";
pub(crate) const INVALID_URL: &str = "https://255.255.255.255";
pub(crate) const SANDBOX_IP: &str = "99.110.204.1";

pub(crate) fn instantiate() -> (ServerGuard, ZeroBounce) {

    let mock_server = Server::new();
    let mock_url: String = mock_server.url().to_owned();

    // ZBUrlProvider is #[non_exhaustive]: start from the default and override.
    let mut mock_url_provider = ZBUrlProvider::default();
    mock_url_provider.url = mock_url.clone();
    mock_url_provider.bulk_url = mock_url.clone();
    mock_url_provider.list_evaluator_url = mock_url.clone();
    mock_url_provider.inbox_placement_url = mock_url.clone();
    let zb_instance = ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        base_url: mock_url.clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: mock_url_provider,
    };
    (mock_server, zb_instance)
}

pub(crate) fn endpoint_matcher(endpoint: &str) -> Matcher {
    Matcher::Regex(endpoint.to_owned() + r#"(\?.*)?"#)
}

pub(crate) fn invalid_url_zb_instance() -> ZeroBounce {
    let mut url_provider = ZBUrlProvider::default();
    url_provider.url = INVALID_URL.to_owned();
    url_provider.bulk_url = INVALID_URL.to_owned();
    url_provider.list_evaluator_url = INVALID_URL.to_owned();
    url_provider.inbox_placement_url = INVALID_URL.to_owned();
    ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        base_url: INVALID_URL.to_owned(),
        client: reqwest::blocking::Client::default(),
        url_provider,
    }
}

pub(crate) fn generate_zb_file() -> ZBFile {
    let file_content = String::from("")
        + "invalid@example.com\n"
        + "valid@example.com\n"
        + "toxic@example.com\n"
        + "donotmail@example.com\n";

    ZBFile::from_content(Vec::from(file_content))
        .set_has_header_row(false)
}