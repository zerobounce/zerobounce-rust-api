//! `null` in optional response fields reads as the type's empty value, and a
//! file job's `message` may be a string or a list of strings (joined with
//! "\n"). Both are accepts, not API errors, on the real calls.

use zero_bounce::utility::{
    CONTENT_TYPE_JSON, ENDPOINT_BATCH_VALIDATE, ENDPOINT_FILE_SEND, ENDPOINT_FILE_STATUS,
    ENDPOINT_VALIDATE,
};
use zero_bounce::utility::structures::bulk::{ZBFileFeedback, ZBFile};

use crate::common::{endpoint_matcher, instantiate, generate_zb_file};

fn mock_json(server: &mut mockito::ServerGuard, method: &str, endpoint: &str, body: &str) -> mockito::Mock {
    server
        .mock(method, endpoint_matcher(endpoint))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create()
}

#[test]
fn null_file_id_and_upload_date_read_as_empty() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "GET",
        ENDPOINT_FILE_STATUS,
        r#"{"success":true,"file_id":null,"file_name":"f.csv","file_status":"Complete",
            "complete_percentage":"100%","return_url":null,"upload_date":null,"error_reason":null}"#,
    );
    let status = zb.bulk_validation_file_status_check("mock_file_id");
    mock.assert();
    let status = status.expect("null file_id and upload_date are accepted");
    assert_eq!(status.file_id, "");
    assert_eq!(status.upload_date.timestamp(), 0);
}

#[test]
fn null_batch_lists_read_as_empty() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "POST",
        ENDPOINT_BATCH_VALIDATE,
        r#"{"email_batch":null,"errors":null}"#,
    );
    let emails = vec![("valid@example.com".to_string(), "".to_string())];
    let batch = zb.batch_validate(emails);
    mock.assert();
    let batch = batch.expect("null email_batch and errors are accepted");
    assert!(batch.email_batch.is_empty());
    assert!(batch.errors.is_empty());
}

#[test]
#[allow(deprecated)]
fn null_other_domain_formats_read_as_empty_on_every_finder_call() {
    let body = r#"{"email":"a@b.c","domain":"b.c","format":"first","status":"","sub_status":"",
        "confidence":"high","did_you_mean":"","failure_reason":"","other_domain_formats":null,
        "email_confidence":"high","company_name":""}"#;

    let (mut server, zb) = instantiate();
    let mock = server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(body)
        .create();
    let v1 = zb.find_email("b.c", "a", "", "b").expect("find_email accepts null formats");
    assert!(v1.other_domain_formats.is_empty());
    let v1_domain = zb.domain_search("b.c").expect("domain_search accepts null formats");
    assert!(v1_domain.other_domain_formats.is_empty());
    let v2 = zb
        .domain_search_v2()
        .domain("b.c")
        .call()
        .expect("domain_search_v2 accepts null formats");
    assert!(v2.other_domain_formats.is_empty());
    mock.expect_at_least(3).assert();
}

#[test]
fn null_processed_at_reads_as_unset_date() {
    let (mut server, zb) = instantiate();
    let mock = mock_json(
        &mut server,
        "GET",
        ENDPOINT_VALIDATE,
        r#"{"address":"a@b.c","status":"valid","sub_status":"","free_email":false,"processed_at":null}"#,
    );
    let validation = zb.validate_email("a@b.c");
    mock.assert();
    let validation = validation.expect("null processed_at is accepted");
    assert_eq!(validation.status, "valid");
}

fn submit_with_message(message_json: &str) -> zero_bounce::utility::ZBResult<ZBFileFeedback> {
    let (mut server, zb) = instantiate();
    let body = format!(r#"{{"success":true,"message":{message_json},"file_name":"f.csv","file_id":"id"}}"#);
    let mock = mock_json(&mut server, "POST", ENDPOINT_FILE_SEND, &body);
    let file: ZBFile = generate_zb_file();
    let result = zb.bulk_validation_file_submit(&file);
    mock.assert();
    result
}

#[test]
fn sendfile_message_list_is_joined_with_newlines() {
    let feedback = submit_with_message(r#"["File Accepted","Second line"]"#)
        .expect("a list of messages is accepted");
    assert_eq!(feedback.message, "File Accepted\nSecond line");
}

#[test]
fn sendfile_null_message_reads_as_empty() {
    let feedback = submit_with_message("null").expect("null message is accepted");
    assert_eq!(feedback.message, "");
}

#[test]
fn sendfile_message_of_another_json_type_is_an_api_error_200() {
    for bad in ["5", "true", "{}", "[5]", r#"["a",5]"#] {
        let err = submit_with_message(bad).expect_err(bad);
        let text = format!("{err:?}");
        assert!(text.contains("200") || text.contains("Some(200)"), "{bad}: {text}");
    }
}
