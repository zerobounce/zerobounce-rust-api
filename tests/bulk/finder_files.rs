//! Bulk email finder and bulk domain search file APIs.
//!
//! These live on the bulk host *without* the `/v2` path the other bulk APIs use,
//! so every test here asserts the path as well as the parameters.

use mockito::Matcher;
use zero_bounce::utility::mock_constants;
use zero_bounce::utility::structures::bulk::{ZBBulkResponse, ZBEmailFinderFileOptions};
use zero_bounce::utility::{CONTENT_TYPE_JSON, CONTENT_TYPE_STREAM};
use zero_bounce::ZBError;

use crate::common::{endpoint_matcher, instantiate, MOCK_API_KEY};

const FINDER_SEND: &str = "/email-finder/sendfile";
const FINDER_RESULT: &str = "/email-finder/getfile";
const FINDER_DELETE: &str = "/email-finder/deletefile";
const DOMAIN_SEND: &str = "/domain-search/sendfile";
const DOMAIN_RESULT: &str = "/domain-search/getfile";
const DOMAIN_DELETE: &str = "/domain-search/deletefile";

/// The api_key and file_id both have to be on the query string, not just one.
fn file_id_query(file_id: &str) -> Matcher {
    Matcher::AllOf(vec![
        Matcher::UrlEncoded("api_key".to_string(), MOCK_API_KEY.to_string()),
        Matcher::UrlEncoded("file_id".to_string(), file_id.to_string()),
    ])
}

// ---------------------------------------------------------------- sendfile

#[test]
fn test_email_finder_submit_sends_every_name_column() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("POST", endpoint_matcher(FINDER_SEND))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_SUBMIT_OK)
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#"name="api_key""#.to_string()),
            Matcher::Regex(r#"name="domain_column"\r\n\r\n3"#.to_string()),
            Matcher::Regex(r#"name="first_name_column"\r\n\r\n1"#.to_string()),
            Matcher::Regex(r#"name="full_name_column"\r\n\r\n5"#.to_string()),
            Matcher::Regex(r#"name="middle_name_column"\r\n\r\n2"#.to_string()),
            Matcher::Regex(r#"name="last_name_column"\r\n\r\n4"#.to_string()),
            // Booleans go on the wire as the strings "true"/"false".
            Matcher::Regex(r#"name="has_header_row"\r\n\r\ntrue"#.to_string()),
        ]))
        .create();

    let options = ZBEmailFinderFileOptions {
        first_name_column: Some(1),
        full_name_column: Some(5),
        middle_name_column: Some(2),
        last_name_column: Some(4),
        has_header_row: Some(true),
    };
    let response =
        zb_instance.send_email_finder_file(3, &options, "leads.csv", &b"example.com\n"[..]);
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);
    assert_eq!(response.unwrap().message, "File Accepted");
}

#[test]
fn test_email_finder_submit_needs_a_name_column() {
    let (mut _mock_server, zb_instance) = instantiate();

    // The API cannot guess a person from a domain alone, so one of the two name
    // columns is required and the SDK says so before spending a request.
    let response = zb_instance.send_email_finder_file(
        1,
        &ZBEmailFinderFileOptions::default(),
        "leads.csv",
        &b"example.com\n"[..],
    );
    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("first_name_column"), "{msg}");
    assert!(msg.contains("full_name_column"), "{msg}");
}

#[test]
fn test_email_finder_submit_rejects_domain_column_zero() {
    let (mut _mock_server, zb_instance) = instantiate();

    let response = zb_instance.send_email_finder_file(
        0,
        &ZBEmailFinderFileOptions {
            first_name_column: Some(2),
            ..Default::default()
        },
        "leads.csv",
        &b"example.com\n"[..],
    );
    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("domain_column"), "{msg}");
}

#[test]
fn test_email_finder_submit_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("POST", endpoint_matcher(FINDER_SEND))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.send_email_finder_file(
        1,
        &ZBEmailFinderFileOptions {
            first_name_column: Some(2),
            ..Default::default()
        },
        "leads.csv",
        &b"example.com\n"[..],
    );
    mock.assert();

    let zb_error = response.unwrap_err();
    assert_eq!(zb_error.status_code(), Some(400));
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains(MOCK_API_KEY), "{message}");
}

#[test]
fn test_domain_search_submit_sends_has_header_row() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("POST", endpoint_matcher(DOMAIN_SEND))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_SUBMIT_OK)
        .match_body(Matcher::AllOf(vec![
            Matcher::Regex(r#"name="domain_column"\r\n\r\n2"#.to_string()),
            Matcher::Regex(r#"name="has_header_row"\r\n\r\nfalse"#.to_string()),
        ]))
        .create();

    let response =
        zb_instance.send_domain_search_file(2, Some(false), "domains.csv", &b"example.com\n"[..]);
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);
}

// ----------------------------------------------------------------- getfile

#[test]
fn test_email_finder_result_fetch_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let expected_content = "domain,email\nexample.com,ada@example.com\n";
    let mock = mock_server
        .mock("GET", endpoint_matcher(FINDER_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_STREAM)
        .with_body(expected_content)
        .match_query(file_id_query("mock_file_id"))
        .create();

    let response = zb_instance.email_finder_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);

    let ZBBulkResponse::Content(content) = response.unwrap() else {
        panic!("expected Content");
    };
    assert_eq!(content.as_ref(), expected_content.as_bytes());
}

#[test]
fn test_domain_search_result_fetch_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let expected_content = "domain,format\nexample.com,first.last\n";
    let mock = mock_server
        .mock("GET", endpoint_matcher(DOMAIN_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_STREAM)
        .with_body(expected_content)
        .match_query(file_id_query("mock_file_id"))
        .create();

    let response = zb_instance.domain_search_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);

    let ZBBulkResponse::Content(content) = response.unwrap() else {
        panic!("expected Content");
    };
    assert_eq!(content.as_ref(), expected_content.as_bytes());
}

#[test]
fn test_email_finder_result_fetch_json_error_http_200() {
    let (mut mock_server, zb_instance) = instantiate();

    // LIVE: {"success": false} on an HTTP 200 is a failure, not file bytes.
    let mock = mock_server
        .mock("GET", endpoint_matcher(FINDER_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_RESULT_DELETED)
        .create();

    let response = zb_instance.email_finder_result_fetch("mock_file_id");
    mock.assert();

    let ZBError::ExplicitError(msg) = response.unwrap_err() else {
        panic!("expected ExplicitError");
    };
    assert!(msg.contains("File deleted"), "{msg}");
}

#[test]
fn test_domain_search_result_fetch_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("GET", endpoint_matcher(DOMAIN_RESULT))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.domain_search_result_fetch("mock_file_id");
    mock.assert();

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains(MOCK_API_KEY), "{message}");
}

#[test]
fn test_result_fetch_rejects_empty_file_id() {
    let (mut _mock_server, zb_instance) = instantiate();

    for error in [
        zb_instance.email_finder_result_fetch("  ").unwrap_err(),
        zb_instance.domain_search_result_fetch("").unwrap_err(),
    ] {
        assert!(error.to_string().contains("file_id"), "{error}");
    }
}

// -------------------------------------------------------------- deletefile

#[test]
fn test_email_finder_result_delete_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("GET", endpoint_matcher(FINDER_DELETE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_DELETE_OK)
        .match_query(file_id_query("mock_file_id"))
        .create();

    let response = zb_instance.email_finder_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);

    let feedback = response.unwrap();
    assert!(feedback.success);
    assert_eq!(feedback.message, "File Deleted");
    assert_eq!(feedback.file_name.as_deref(), Some("emails2.txt"));
}

#[test]
fn test_domain_search_result_delete_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("GET", endpoint_matcher(DOMAIN_DELETE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_DELETE_OK)
        .match_query(file_id_query("mock_file_id"))
        .create();

    let response = zb_instance.domain_search_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);
    assert!(response.unwrap().success);
}

#[test]
fn test_email_finder_result_delete_not_found() {
    let (mut mock_server, zb_instance) = instantiate();

    // LIVE: an unknown file id comes back as {"success": false} with an HTTP 200.
    let mock = mock_server
        .mock("GET", endpoint_matcher(FINDER_DELETE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_DELETE_NOT_FOUND)
        .create();

    let response = zb_instance.email_finder_result_delete("mock_file_id");
    mock.assert();

    let zb_error = response.unwrap_err();
    assert_eq!(zb_error.status_code(), Some(200));
    let message = zb_error.to_string();
    assert!(message.contains("File cannot be found"), "{message}");
    assert!(message.len() < 500, "{message}");
}

#[test]
fn test_domain_search_result_delete_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("GET", endpoint_matcher(DOMAIN_DELETE))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.domain_search_result_delete("mock_file_id");
    mock.assert();

    let zb_error = response.unwrap_err();
    let ZBError::ApiError { .. } = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_email_finder_result_delete_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server
        .mock("GET", endpoint_matcher(FINDER_DELETE))
        .with_status(200)
        .with_body("")
        .create();

    let response = zb_instance.email_finder_result_delete("mock_file_id");
    mock.assert();

    let message = response.unwrap_err().to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains("expected"), "{message}");
}

#[test]
fn test_result_delete_rejects_empty_file_id() {
    let (mut _mock_server, zb_instance) = instantiate();

    for error in [
        zb_instance.email_finder_result_delete(" ").unwrap_err(),
        zb_instance.domain_search_result_delete("").unwrap_err(),
    ] {
        assert!(error.to_string().contains("file_id"), "{error}");
    }
}
