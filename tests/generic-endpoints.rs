use chrono::NaiveDate;
use zero_bounce::api::{ZBUrlProvider, ZeroBounce};

use zero_bounce::utility::{ENDPOINT_CREDITS, ENDPOINT_API_USAGE};
use zero_bounce::utility::mock_constants::{API_USAGE_RESPONSE, CREDITS_RESPONSE};

use mockito::{Matcher, ServerGuard, Server};

const MOCK_API_KEY: &str = "mock_api_key";

fn instantiate<'s>() -> (ServerGuard, ZeroBounce) {

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

fn endpoint_matcher(endpoint: &str) -> Matcher {
    Matcher::Regex(endpoint.to_owned() + r#"(\?.*)?"#)
}

#[test]
fn test_credits() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_CREDITS))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(CREDITS_RESPONSE.clone())
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_ok());

    let amount = credits.unwrap();
    assert_eq!(amount, 123456);
    mock.assert();
}

#[test]
fn test_api_usage() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_API_USAGE))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(API_USAGE_RESPONSE.clone())
        .create();

    // start_date, end_date will not match the response, as the API should
    let api_usage_response = zb_instance.get_api_usage_overall();
    assert!(api_usage_response.is_ok());
    mock.assert();

    let api_usage = api_usage_response.unwrap();
    let expected_start_date = NaiveDate::from_ymd_opt(2010, 1, 12).unwrap();
    let expected_end_date = NaiveDate::from_ymd_opt(2030, 12, 1).unwrap();

    assert_eq!(api_usage.total, 10);
    assert_eq!(api_usage.status_valid, 10);
    assert_eq!(api_usage.status_invalid, 0);
    assert_eq!(api_usage.start_date, expected_start_date);
    assert_eq!(api_usage.end_date, expected_end_date);
}
