use zero_bounce::utility::{ENDPOINT_BATCH_VALIDATE, ZBError, mock_constants};

use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};

fn emails_and_ips() -> Vec<(String, String)> {
    vec![
        ("valid@example.com".to_string(), "123.123.123.123".to_string()),
        ("invalid@example.com".to_string(), "".to_string()),
    ]
}

#[test]
fn test_batch_validation_client_error() {
    let zb_instance = invalid_url_zb_instance();
    let emails_and_ip_addresses = emails_and_ips();
    let validation = zb_instance.batch_validate(emails_and_ip_addresses);
    assert!(validation.is_err());

    let zb_error = validation.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_batch_validation_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_BATCH_VALIDATE))
        .with_status(200)
        .with_body("")
        .create();

    let emails_and_ip_addresses = emails_and_ips();
    let validation = zb_instance.batch_validate(emails_and_ip_addresses);
    assert!(validation.is_err());
    mock.assert();

    let zb_error = validation.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_batch_validation_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_BATCH_VALIDATE))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let emails_and_ip_addresses = emails_and_ips();
    let validation = zb_instance.batch_validate(emails_and_ip_addresses);
    assert!(validation.is_err());
    mock.assert();

    let zb_error = validation.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_batch_validation_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_BATCH_VALIDATE))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::BATCH_VALIDATION_NO_ERROR.clone())
        .create();

    // start_date, end_date will not match the response, as the API should
    let emails_and_ip_addresses = emails_and_ips();
    let validation = zb_instance.batch_validate(emails_and_ip_addresses);
    assert!(validation.is_ok());
    mock.assert();
}
