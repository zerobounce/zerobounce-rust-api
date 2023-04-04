use zero_bounce::{ZBUrlProvider, ZeroBounce};
use zero_bounce::utility::{ENDPOINT_CREDITS, ZBError, mock_constants};

use crate::common::{INVALID_URL, MOCK_API_KEY};
use crate::common::{instantiate, endpoint_matcher};

#[test]
fn test_credits_invalid_client_error() {
    // no mock server
    let zb_instance = ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: ZBUrlProvider {
            url: INVALID_URL.to_owned(),
            bulk_url: INVALID_URL.to_owned(),
        },
    };

    let credits = zb_instance.get_credits();
    assert!(credits.is_err());

    let credits_error = credits.unwrap_err();
    let ZBError::RequestError(_) = credits_error else {
        panic!("unexpected error: {:#?}", credits_error);
    };
}

#[test]
fn test_credits_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_CREDITS))
        .with_status(200)
        .with_body("")
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_err());
    mock.assert();

    let credits_error = credits.unwrap_err();
    let ZBError::JsonError(_) = credits_error else {
        panic!("unexpected error: {:#?}", credits_error);
    };
}

#[test]
fn test_credits_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_CREDITS))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_err());
    mock.assert();

    let credits_error = credits.unwrap_err();
    let ZBError::ExplicitError(_) = credits_error else {
        panic!("unexpected error: {:#?}", credits_error);
    };
}

#[test]
fn test_credits_negative_response() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_CREDITS))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::CREDITS_RESPONSE_NEGATIVE.clone())
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_ok());
    mock.assert();

    let amount = credits.unwrap();
    assert_eq!(amount, -1);
}

#[test]
fn test_credits_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_CREDITS))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::CREDITS_RESPONSE_OK.clone())
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_ok());
    mock.assert();

    let amount = credits.unwrap();
    assert_eq!(amount, 123456);
}
