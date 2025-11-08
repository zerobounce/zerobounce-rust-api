use zero_bounce::utility::{ENDPOINT_CREDITS, CONTENT_TYPE_JSON, ZBError, mock_constants};

use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};

#[test]
fn test_credits_invalid_client_error() {
    // no mock server
    let zb_instance = invalid_url_zb_instance();
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
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
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
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::CREDITS_RESPONSE_NEGATIVE)
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
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::CREDITS_RESPONSE_OK)
        .create();

    let credits = zb_instance.get_credits();
    assert!(credits.is_ok());
    mock.assert();

    let amount = credits.unwrap();
    assert_eq!(amount, 123456);
}
