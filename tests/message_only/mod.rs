//! Message handling in the answers. A finder or domain-search answer whose keys
//! are only message keys is an error with status 200, whatever the message's
//! value: a string, a blank, a null or a list (standard.md §3). Validate's
//! greylist `{"Message": ...}` is a result, so it is not covered here. A file
//! job's `message` (delete, and the file status) may be a list of strings.
//! An empty object (`{}`, `{ }`, `{\n}`, `{\t}`) is the same failure on those
//! calls, and a wrong-shape answer carries the response's own status.

use zero_bounce::utility::http_error::UNEXPECTED_JSON_SHAPE_MESSAGE;
use zero_bounce::utility::{
    CONTENT_TYPE_JSON, ENDPOINT_ACTIVITY_DATA, ENDPOINT_API_USAGE, ENDPOINT_CREDITS,
    ENDPOINT_EMAIL_FINDER, ENDPOINT_FILE_DELETE, ENDPOINT_FILE_STATUS, ENDPOINT_SCORING,
    ENDPOINT_SCORING_DELETE, ENDPOINT_SCORING_STATUS, ENDPOINT_VALIDATE,
};
use zero_bounce::utility::ZBError;

use crate::common::{endpoint_matcher, instantiate};

/// Bodies that are nothing but a message, each with a different message value.
const MESSAGE_ONLY_BODIES: [&str; 11] = [
    "{}",
    "{ }",
    "{\n}",
    "{\t}",
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

fn mock_status(server: &mut mockito::ServerGuard, endpoint: &str, status: usize, body: &str) -> mockito::Mock {
    server
        .mock("GET", endpoint_matcher(endpoint))
        .with_status(status)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create()
}

/// `{}` and a message-only body on a file status are the API's error, with the
/// response status. This is the test that fails when the empty-object check in
/// `reject_message_only_body` is removed: a bare `{}` is a JSON object, so
/// nothing else on this path refuses it.
#[test]
fn an_empty_or_message_only_file_status_is_an_api_error_with_the_response_status() {
    let bodies = [
        "{}", "{ }", "{\n}", "{\t}",
        r#"{"message":"m"}"#, r#"{"Message":null}"#, r#"{"error":""}"#,
        "[]", r#"[{"file_status":"Complete"}]"#, "0", r#""x""#, "null", "true",
    ];
    let mut failures = Vec::new();
    for (base, call) in [("email-finder", "email_finder"), ("domain-search", "domain_search")] {
        for body in bodies {
            let (mut server, zb) = instantiate();
            let mock = mock_status(&mut server, &format!("/{base}/filestatus"), 202, body);
            let outcome = match call {
                "email_finder" => zb.email_finder_file_status("f"),
                _ => zb.domain_search_file_status("f"),
            };
            mock.assert();
            match outcome {
                Err(error) if error.status_code() == Some(202) => {}
                other => failures.push(format!("{call} {body:?}: {other:?}")),
            }
        }
    }
    assert!(failures.is_empty(), "not an ApiError 202:\n{}", failures.join("\n"));
}

#[test]
fn a_real_file_status_is_still_returned_as_the_raw_string() {
    let bodies = [
        r#"{"success":true,"file_id":"f","file_status":"Complete","message":"done"}"#,
        r#"{"file_status":"Processing"}"#,
    ];
    for body in bodies {
        let (mut server, zb) = instantiate();
        let mock = mock_status(&mut server, "/email-finder/filestatus", 200, body);
        assert_eq!(zb.email_finder_file_status("f").expect("a real status"), body);
        mock.assert();

        let (mut server, zb) = instantiate();
        let mock = mock_status(&mut server, "/domain-search/filestatus", 200, body);
        assert_eq!(zb.domain_search_file_status("f").expect("a real status"), body);
        mock.assert();
    }
}

#[test]
#[allow(deprecated)]
fn an_empty_or_unrelated_domain_search_v2_object_is_an_api_error_with_the_response_status() {
    let bodies = [
        "{}", "{ }", "{\n}", "{\t}",
        r#"{"message":"m","file_id":null}"#,
        r#"{"file_id":"x"}"#,
        r#"{"success":true}"#,
    ];
    for body in bodies {
        let (mut server, zb) = instantiate();
        let mock = mock_status(&mut server, ENDPOINT_EMAIL_FINDER, 202, body);
        let outcome = zb.domain_search_v2().domain("example.com").call();
        mock.assert();
        match outcome {
            Err(error) => {
                assert_eq!(error.status_code(), Some(202), "{body:?}");
                assert_eq!(error.to_string(), UNEXPECTED_JSON_SHAPE_MESSAGE, "{body:?}");
            }
            Ok(found) => panic!("{body:?} was accepted: {found:?}"),
        }
    }
}

/// Fields the live API omits stay optional, and `null` is still empty.
#[test]
fn a_domain_search_v2_object_with_any_field_of_its_own_is_still_a_result() {
    for body in [
        r#"{"domain":null}"#,
        r#"{"domain":"example.com"}"#,
        r#"{"other_domain_formats":[]}"#,
        r#"{"company_name":"Example","message":"m"}"#,
        r#"{"domain":"example.com","format":"first.last","confidence":"high","did_you_mean":"","failure_reason":"","other_domain_formats":[{"format":"f","confidence":"low"}]}"#,
    ] {
        let (mut server, zb) = instantiate();
        let mock = mock_status(&mut server, ENDPOINT_EMAIL_FINDER, 200, body);
        let outcome = zb.domain_search_v2().domain("example.com").call();
        mock.assert();
        outcome.unwrap_or_else(|error| panic!("{body}: {error:?}"));
    }
}

/// A wrong-shape 2xx answer carries the response's status on every call, not
/// a hard-coded 200.
#[test]
#[allow(deprecated)]
fn a_wrong_shape_answer_carries_the_response_status() {
    type Call = fn(&zero_bounce::ZeroBounce) -> zero_bounce::utility::ZBResult<()>;
    let calls: Vec<(&str, &str, &str, Call)> = vec![
        ("GET", ENDPOINT_CREDITS, r#"{"Credits":"abc"}"#, |zb| zb.get_credits().map(|_| ())),
        ("GET", ENDPOINT_CREDITS, r#"[]"#, |zb| zb.get_credits().map(|_| ())),
        ("GET", ENDPOINT_API_USAGE, r#"{"total":"x"}"#, |zb| zb.get_api_usage_overall().map(|_| ())),
        ("GET", ENDPOINT_ACTIVITY_DATA, r#"{"found":"x"}"#, |zb| zb.get_activity_data("a@b.com").map(|_| ())),
        ("GET", ENDPOINT_SCORING, r#"{"email":1}"#, |zb| zb.score_email("a@b.com").map(|_| ())),
        ("POST", ENDPOINT_SCORING, r#"{"email":1}"#, |zb| zb.score_email_post("a@b.com").map(|_| ())),
        ("GET", ENDPOINT_VALIDATE, r#"{"address":1}"#, |zb| zb.validate_email("a@b.com").map(|_| ())),
        ("GET", ENDPOINT_VALIDATE, r#"{"foo":"bar"}"#, |zb| zb.validate_email("a@b.com").map(|_| ())),
        ("GET", ENDPOINT_EMAIL_FINDER, r#"{"email":1}"#, |zb| zb.find_email("example.com", "J", "", "D").map(|_| ())),
        ("GET", ENDPOINT_EMAIL_FINDER, r#"{"email":1}"#, |zb| {
            zb.find_email_v2().first_name("J").domain("example.com").call().map(|_| ())
        }),
        ("GET", "/listS3files", r#"{"name":"x"}"#, |zb| zb.list_s3_files(None, None).map(|_| ())),
    ];
    let mut failures = Vec::new();
    for (method, endpoint, body, call) in calls {
        for status in [201usize, 299] {
            let (mut server, zb) = instantiate();
            let mock = server
                .mock(method, endpoint_matcher(endpoint))
                .with_status(status)
                .with_header("content-type", CONTENT_TYPE_JSON)
                .with_body(body)
                .create();
            let outcome = call(&zb);
            mock.assert();
            match outcome {
                Err(error)
                    if error.status_code() == Some(status as u16)
                        && error.to_string() == UNEXPECTED_JSON_SHAPE_MESSAGE => {}
                other => failures.push(format!("{method} {endpoint} {body} {status}: {other:?}")),
            }
        }
    }
    assert!(failures.is_empty(), "wrong status or message:\n{}", failures.join("\n"));
}

/// An empty object is not a failure where a message-only body is not one
/// either: the filters, the list evaluator's result and a validate greylist
/// answer.
#[test]
fn an_empty_object_is_still_a_result_where_message_only_is_not_a_failure() {
    let (mut server, zb) = instantiate();
    let mock = mock_status(&mut server, "/filters/list", 200, "{}");
    zb.list_filters(false).expect("an empty object is a filter list");
    mock.assert();

    let (mut server, zb) = instantiate();
    let mock = mock_status(&mut server, "/file_id_1/", 200, "{ }");
    zb.get_list_evaluator_result("file_id_1").expect("an empty object is an evaluator result");
    mock.assert();

    let (mut server, zb) = instantiate();
    let mock = mock_status(
        &mut server,
        ENDPOINT_VALIDATE,
        200,
        r#"{"address":"a@b.com","status":"unknown","sub_status":"greylisted","Message":"Greylisted"}"#,
    );
    zb.validate_email("a@b.com").expect("a greylist answer is a result");
    mock.assert();
}
