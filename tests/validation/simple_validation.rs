use zero_bounce::utility::{ENDPOINT_VALIDATE, CONTENT_TYPE_JSON, ZBError, mock_constants};
use zero_bounce::utility::structures::validate_enums::ZBValidateSubStatus;

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
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains("expected"), "{message}");
}

#[test]
fn test_simple_validation_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let validation = zb_instance.validate_email(EMAIL);
    assert!(validation.is_err());
    mock.assert();

    let zb_error = validation.unwrap_err();
    let ZBError::ApiError { .. } = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_simple_validation_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::VALIDATION_RESPONSE_VALID)
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
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::VALIDATION_RESPONSE_VALID)
        .match_query(mockito::Matcher::UrlEncoded("email".to_string(), EMAIL.to_string()))
        .match_query(mockito::Matcher::UrlEncoded(
            "ip_address".to_string(), SANDBOX_IP.to_string()
        ))
        .create();

    // start_date, end_date will not match the response, as the API should
    let validation_response = zb_instance.validate_email_and_ip(EMAIL, SANDBOX_IP);
    assert!(validation_response.is_ok());
    mock.assert();
}


/// Runs one validate call against a body carrying the given status/sub_status
/// and hands back the parsed sub-status enum.
#[cfg(test)]
fn sub_status_for(status: &str, sub_status: &str) -> ZBValidateSubStatus {
    let (mut mock_server, zb_instance) = instantiate();
    // Start from the full valid-response fixture so every required field is
    // present, then swap only the two values under test.
    let body = mock_constants::VALIDATION_RESPONSE_VALID
        .replace(r#""status": "valid""#, &format!(r#""status": "{}""#, status))
        .replace(r#""sub_status": """#, &format!(r#""sub_status": "{}""#, sub_status));

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_VALIDATE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create();

    let validation = zb_instance.validate_email(EMAIL);
    mock.assert();
    validation.expect("validation should succeed").sub_status_enum()
}

#[test]
fn test_sub_status_ai_agent_mailbox_valid() {
    assert_eq!(
        sub_status_for("valid", "ai_agent_mailbox"),
        ZBValidateSubStatus::AiAgentMailbox
    );
}

#[test]
fn test_sub_status_ai_agent_mailbox_catch_all() {
    assert_eq!(
        sub_status_for("catch-all", "ai_agent_mailbox"),
        ZBValidateSubStatus::AiAgentMailbox
    );
}

#[test]
fn test_unknown_sub_status_is_preserved() {
    // from_str has a catch-all arm, so a sub_status added after this SDK
    // shipped round-trips through UnknownValue instead of being lost.
    assert_eq!(
        sub_status_for("valid", "some_future_sub_status"),
        ZBValidateSubStatus::UnknownValue("some_future_sub_status".to_string())
    );
}
