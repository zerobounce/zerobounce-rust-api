//! Message handling in the answers. A finder or domain-search answer whose keys
//! are only message keys is an error with status 200, whatever the message's
//! value: a string, a blank, a null or a list (standard.md §3). Validate's
//! greylist `{"Message": ...}` is a result, so it is not covered here. A file
//! job's `message` (delete, and the file status) may be a list of strings.

use zero_bounce::utility::{
    CONTENT_TYPE_JSON, ENDPOINT_EMAIL_FINDER, ENDPOINT_FILE_DELETE, ENDPOINT_FILE_STATUS,
    ENDPOINT_SCORING_DELETE, ENDPOINT_SCORING_STATUS,
};
use zero_bounce::utility::ZBError;

use crate::common::{endpoint_matcher, instantiate};

/// Bodies that are nothing but a message, each with a different message value.
const MESSAGE_ONLY_BODIES: [&str; 7] = [
    r#"{"Message":"Invalid API key or your account ran out of credits"}"#,
    r#"{"Message":""}"#,
    r#"{"Message":"   "}"#,
    r#"{"Message":null}"#,
    r#"{"Message":["Invalid API key"]}"#,
    r#"{"Message":[]}"#,
    r#"{"error":""}"#,
];

#[test]
#[allow(deprecated)]
fn a_message_only_finder_or_domain_search_answer_is_an_api_error_200() {
    let mut failures = Vec::new();
    for call in ["find_email", "find_email_v2", "domain_search", "domain_search_v2"] {
        for body in MESSAGE_ONLY_BODIES {
            let (mut server, zb) = instantiate();
            let mock = server
                .mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
                .with_status(200)
                .with_header("content-type", CONTENT_TYPE_JSON)
                .with_body(body)
                .create();
            let outcome = match call {
                "find_email" => zb.find_email("example.com", "John", "", "Doe").map(|_| ()),
                "find_email_v2" => zb.find_email_v2().first_name("John").domain("example.com").call().map(|_| ()),
                "domain_search" => zb.domain_search("example.com").map(|_| ()),
                _ => zb.domain_search_v2().domain("example.com").call().map(|_| ()),
            };
            mock.assert();
            match outcome {
                Err(ZBError::ApiError { status: 200, .. }) => {}
                other => failures.push(format!("{call} {body}: {other:?}")),
            }
        }
    }
    assert!(failures.is_empty(), "not an ApiError 200:\n{}", failures.join("\n"));
}

fn json_mock(server: &mut mockito::ServerGuard, endpoint: &str, body: &str) -> mockito::Mock {
    server
        .mock("GET", endpoint_matcher(endpoint))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create()
}

#[test]
fn a_list_message_on_a_delete_is_joined_with_newlines() {
    let (mut server, zb) = instantiate();
    let mock = json_mock(&mut server, ENDPOINT_FILE_DELETE, r#"{"success":true,"message":["Deleted","Second line"]}"#);
    let feedback = zb.bulk_validation_result_delete("file_id");
    mock.assert();
    assert_eq!(feedback.expect("a list message is accepted").message, "Deleted\nSecond line");

    let (mut server, zb) = instantiate();
    let mock = json_mock(&mut server, ENDPOINT_SCORING_DELETE, r#"{"success":true,"message":["Deleted"]}"#);
    let feedback = zb.ai_scoring_result_delete("file_id");
    mock.assert();
    assert_eq!(feedback.expect("a list message is accepted").message, "Deleted");

    let (mut server, zb) = instantiate();
    let mock = json_mock(&mut server, ENDPOINT_FILE_DELETE, r#"{"success":true,"message":null}"#);
    let feedback = zb.bulk_validation_result_delete("file_id");
    mock.assert();
    assert_eq!(feedback.expect("a null message is accepted").message, "");
}

#[test]
fn a_list_message_on_a_file_status_is_accepted() {
    let body = r#"{"success":true,"file_id":"f","file_name":"f.csv","file_status":"Complete",
        "complete_percentage":"100%","return_url":null,"upload_date":"2023-04-26T17:52:23Z",
        "error_reason":null,"message":["File is complete","Second line"]}"#;

    let (mut server, zb) = instantiate();
    let mock = json_mock(&mut server, ENDPOINT_FILE_STATUS, body);
    let status = zb.bulk_validation_file_status_check("f");
    mock.assert();
    let status = status.expect("a list message on a file status is accepted");
    assert_eq!(status.file_status, "Complete");
    assert_eq!(status.complete_percentage, 100.0);

    let (mut server, zb) = instantiate();
    let mock = json_mock(&mut server, ENDPOINT_SCORING_STATUS, body);
    let status = zb.ai_scoring_file_status_check("f");
    mock.assert();
    assert_eq!(status.expect("a list message on a scoring status is accepted").file_status, "Complete");
}
