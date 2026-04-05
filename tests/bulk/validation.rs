use zero_bounce::{ZBError, get_file_json_indicates_error};
use zero_bounce::utility::mock_constants;
use zero_bounce::utility::structures::bulk::{zb_download_type, ZBBulkResponse, ZBGetFileOptions};
use zero_bounce::utility::{CONTENT_TYPE_JSON, CONTENT_TYPE_STREAM};
use zero_bounce::utility::{ENDPOINT_FILE_SEND, ENDPOINT_FILE_STATUS, ENDPOINT_FILE_RESULT, ENDPOINT_FILE_DELETE};


use crate::common::{instantiate, endpoint_matcher, generate_zb_file};


#[test]
fn test_bulk_validate_submit_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_FILE_SEND))
        .with_status(200)
        .with_body("")
        .create();

    let zb_file = &generate_zb_file();
    let response = zb_instance.bulk_validation_file_submit(zb_file);
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_submit_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_FILE_SEND))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_SUBMIT_ERROR)
        .create();

    let zb_file = &generate_zb_file();
    let response = zb_instance.bulk_validation_file_submit(zb_file);
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_submit_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_FILE_SEND))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_SUBMIT_OK)
        .create();

    let zb_file = &generate_zb_file();
    let response = zb_instance.bulk_validation_file_submit(zb_file);
    mock.assert();
    assert!(response.is_ok());
}

#[test]
fn test_bulk_validate_status_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_STATUS))
        .with_status(200)
        .with_body("")
        .create();

    let response = zb_instance.bulk_validation_file_status_check("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_status_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_STATUS))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.bulk_validation_file_status_check("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_status_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_STATUS))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_STATUS_OK)
        .create();

    let response = zb_instance.bulk_validation_file_status_check("mock_file_id");
    mock.assert();
    assert!(response.is_ok());
}

#[test]
fn test_bulk_validate_result_no_content_type() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(200)
        .with_body("mock content")
        .create();

    let response = zb_instance.bulk_validation_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_result_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.bulk_validation_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_result_json_error_http_200() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_RESULT_DELETED)
        .create();

    let response = zb_instance.bulk_validation_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let ZBError::ExplicitError(msg) = response.unwrap_err() else {
        panic!("expected ExplicitError");
    };
    assert!(msg.contains("File deleted"), "{}", msg);
}

#[test]
fn test_get_file_json_indicates_error_export() {
    assert!(get_file_json_indicates_error(r#"{"success":false,"message":""}"#));
    assert!(!get_file_json_indicates_error(r#"{"file_id":"x"}"#));
}

#[test]
fn test_bulk_validate_result_fetch_with_options() {
    let (mut mock_server, zb_instance) = instantiate();

    let expected_content = "col1,col2\n";
    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_STREAM)
        .with_body(expected_content)
        .create();

    let mut opts = ZBGetFileOptions::default();
    opts.download_type = Some(zb_download_type::COMBINED.to_string());
    opts.activity_data = Some(true);

    let response = zb_instance.bulk_validation_result_fetch_with_options("mock_file_id", &opts);
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);
    let ZBBulkResponse::Content(b) = response.unwrap() else {
        panic!("expected Content");
    };
    assert_eq!(b.as_ref(), expected_content.as_bytes());
}

#[test]
fn test_bulk_validate_result_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let expected_content = "some raw content";
    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_STREAM)
        .with_body(expected_content)
        .create();

    let response = zb_instance.bulk_validation_result_fetch("mock_file_id");
    mock.assert();
    assert!(response.is_ok(), "{:#?}", response);

    let response_obj = response.unwrap();
    if let ZBBulkResponse::Content(content) = response_obj {
        let expected_bytes = bytes::Bytes::from(expected_content);
        assert_eq!(content, expected_bytes);
    } else {
        panic!("unexpected response type: {:#?}", response_obj)
    }
}


#[test]
fn test_bulk_validate_delete_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_DELETE))
        .with_status(200)
        .with_body("")
        .create();

    let response = zb_instance.bulk_validation_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_delete_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_DELETE))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .create();

    let response = zb_instance.bulk_validation_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_err());

    let zb_error = response.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_bulk_validate_delete_not_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_DELETE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_DELETE_NOT_FOUND)
        .create();

    let response = zb_instance.bulk_validation_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_ok());

    let response_obj = response.unwrap();
    assert_eq!(response_obj.success, false, "{:#?}", response_obj);
}

#[test]
fn test_bulk_validate_delete_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_DELETE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::BULK_VALIDATION_DELETE_OK)
        .create();

    let response = zb_instance.bulk_validation_result_delete("mock_file_id");
    mock.assert();
    assert!(response.is_ok());

    let response_obj = response.unwrap();
    assert_eq!(response_obj.success, true, "{:#?}", response_obj);
}



