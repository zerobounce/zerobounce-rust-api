use zero_bounce::utility::{ENDPOINT_VALIDATE, ZBError, mock_constants};

use crate::common::{EMAIL, SANDBOX_IP};
use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};

#[test]
fn test_simple_validation_client_error() {
    let zb_instance = invalid_url_zb_instance();
    let validation = zb_instance.validate_email(EMAIL);
    assert!(validation.is_err());

    let zb_error = validation.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_simple_validation_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(200)
        .with_body("")
        .create();

    let validation = zb_instance.validate_email(EMAIL);
    assert!(validation.is_err());
    mock.assert();

    let zb_error = validation.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_simple_validation_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let validation = zb_instance.validate_email(EMAIL);
    assert!(validation.is_err());
    mock.assert();

    let zb_error = validation.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_simple_validation_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::VALIDATION_RESPONSE_VALID.clone())
        .match_query(mockito::Matcher::UrlEncoded("email".to_string(), EMAIL.to_string()))
        .create();

    // start_date, end_date will not match the response, as the API should
    let validation_response = zb_instance.validate_email(EMAIL);
    assert!(validation_response.is_ok());
    mock.assert();
}

#[test]
fn test_simple_validation_with_ip_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::VALIDATION_RESPONSE_VALID.clone())
        .match_query(mockito::Matcher::UrlEncoded("email".to_string(), EMAIL.to_string()))
        .match_query(mockito::Matcher::UrlEncoded("ip_address".to_string(), SANDBOX_IP.to_string()))
        .create();

    // start_date, end_date will not match the response, as the API should
    let validation_response = zb_instance.validate_email_and_ip(EMAIL, SANDBOX_IP);
    assert!(validation_response.is_ok());
    mock.assert();
}
