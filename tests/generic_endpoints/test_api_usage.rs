use chrono::NaiveDate;

use zero_bounce::{ZBUrlProvider, ZeroBounce};
use zero_bounce::utility::{ENDPOINT_API_USAGE, ZBError, mock_constants};

use crate::common::{INVALID_URL, MOCK_API_KEY};
use crate::common::{instantiate, endpoint_matcher};

#[test]
fn test_api_usage_client_error() {
    // no mock server
    let zb_instance = ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: ZBUrlProvider {
            url: INVALID_URL.to_owned(),
            bulk_url: INVALID_URL.to_owned(),
        },
    };

    let api_usage = zb_instance.get_api_usage_overall();
    assert!(api_usage.is_err());

    let zb_error = api_usage.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_api_usage_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_API_USAGE))
        .with_status(200)
        .with_body("")
        .create();

    let api_usage = zb_instance.get_api_usage_overall();
    assert!(api_usage.is_err());
    mock.assert();

    let zb_error = api_usage.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_api_usage_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_API_USAGE))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let api_usage = zb_instance.get_api_usage_overall();
    assert!(api_usage.is_err());
    mock.assert();

    let zb_error = api_usage.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_api_usage_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_API_USAGE))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::API_USAGE_RESPONSE.clone())
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
