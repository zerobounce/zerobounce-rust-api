use zero_bounce::{ZBError, get_file_json_indicates_error};
use crate::mock_constants;
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
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains("expected"), "{message}");
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
    let ZBError::ApiError { .. } = zb_error else {
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
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains("expected"), "{message}");
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
    let ZBError::ApiError { .. } = zb_error else {
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
    // A response arrived, so the error carries its status.
    let ZBError::ApiError { status: 200, .. } = zb_error else {
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
    let ZBError::ApiError { status: 400, .. } = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert_eq!(zb_error.status_code(), Some(400));
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

    let zb_error = response.unwrap_err();
    assert_eq!(zb_error.status_code(), Some(200));
    let ZBError::ApiError { message: msg, .. } = zb_error else {
        panic!("expected ApiError");
    };
    assert!(msg.contains("File deleted"), "{}", msg);
}

#[test]
fn test_get_file_json_indicates_error_export() {
    assert!(get_file_json_indicates_error(r#"{"success":false,"message":""}"#));
    // Any JSON object or array is an error, not a result file.
    assert!(get_file_json_indicates_error(r#"{"file_id":"x"}"#));
    assert!(get_file_json_indicates_error(r#"["x"]"#));
    assert!(get_file_json_indicates_error("{not json"));
    assert!(get_file_json_indicates_error("\u{FEFF}\u{FEFF}{not json"));
    assert!(!get_file_json_indicates_error("email,status\n"));
}

/// What a download served with `content_type` and `body` comes back as.
fn fetch_download(
    status: usize,
    content_type: Option<&str>,
    body: &str,
    headers: &[(&str, &str)],
) -> Result<ZBBulkResponse, ZBError> {
    let (mut mock_server, mut zb_instance) = instantiate();
    // Never follow a redirect: the test mock has no second page to go to.
    zb_instance.client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let mut mock = mock_server
        .mock("GET", endpoint_matcher(ENDPOINT_FILE_RESULT))
        .with_status(status)
        .with_body(body);
    if let Some(content_type) = content_type {
        mock = mock.with_header("content-type", content_type);
    }
    for &(name, value) in headers {
        mock = mock.with_header(name, value);
    }
    let mock = mock.create();
    let response = zb_instance.bulk_validation_result_fetch("mock_file_id");
    mock.assert();
    response
}

#[test]
fn test_bulk_validate_result_rejects_a_bom_before_json() {
    let cases = [
        ("one BOM", "\u{FEFF}{\"file_id\":\"x\"}"),
        ("two BOMs", "\u{FEFF}\u{FEFF}{\"message\":\"No file.\"}"),
        ("three BOMs and a list", "\u{FEFF}\u{FEFF}\u{FEFF}[\"x\"]"),
        ("not json", "{not json"),
        ("BOMs and not json", "\u{FEFF}\u{FEFF}{not json"),
        ("plain object", "{\"file_id\":\"x\"}"),
    ];
    for (name, body) in cases {
        let response = fetch_download(200, Some(CONTENT_TYPE_STREAM), body, &[]);
        match response {
            Err(ZBError::ApiError { status: 200, .. }) => {}
            other => panic!("{name}: saved or wrong error: {other:?}"),
        }
    }
}

#[test]
fn test_bulk_validate_result_bom_error_keeps_its_message() {
    let response = fetch_download(
        200,
        Some(CONTENT_TYPE_STREAM),
        "\u{FEFF}\u{FEFF}{\"message\":\"File deleted.\"}",
        &[],
    );
    let Err(ZBError::ApiError { message, .. }) = response else {
        panic!("expected ApiError, got {response:?}");
    };
    assert_eq!(message, "File deleted.");
}

#[test]
fn test_bulk_validate_result_bom_only_body_is_empty() {
    for body in ["\u{FEFF}", "\u{FEFF}\u{FEFF}\u{FEFF}", "\u{FEFF} \n\u{FEFF}"] {
        let response = fetch_download(200, Some(CONTENT_TYPE_STREAM), body, &[]);
        match response {
            Err(ZBError::ApiError { status: 200, message }) => {
                assert!(message.contains("Empty response"), "{message}");
            }
            other => panic!("{body:?}: saved or wrong error: {other:?}"),
        }
    }
}

#[test]
fn test_bulk_validate_result_rejects_json_content_types() {
    for content_type in [
        "application/json",
        "application/problem+json",
        "application/vnd.api+JSON; charset=utf-8",
        "Application/JSON",
    ] {
        // Whatever the body: even one that looks like a CSV.
        let response = fetch_download(200, Some(content_type), "email,status\na@b.c,valid\n", &[]);
        match response {
            Err(ZBError::ApiError { status: 200, .. }) => {}
            other => panic!("{content_type}: saved or wrong error: {other:?}"),
        }
    }
}

#[test]
fn test_bulk_validate_result_plain_csv_with_or_without_bom_is_ok() {
    for body in ["email,status\na@b.c,valid\n", "\u{FEFF}email,status\na@b.c,valid\n"] {
        let response = fetch_download(200, Some("text/csv"), body, &[]);
        let Ok(ZBBulkResponse::Content(content)) = response else {
            panic!("{body:?}: expected Content, got {response:?}");
        };
        assert_eq!(content.as_ref(), body.as_bytes());
    }
}

#[test]
fn test_bulk_validate_result_rejects_an_encoded_body() {
    for encoding in ["gzip", "br", "deflate", "zstd", "GZIP", "identity, gzip"] {
        let response = fetch_download(
            200,
            Some("text/csv"),
            "not really compressed",
            &[("content-encoding", encoding)],
        );
        match response {
            Err(ZBError::ApiError { status: 200, .. }) => {}
            other => panic!("{encoding}: saved or wrong error: {other:?}"),
        }
    }
    // `identity` and an empty value are the plain bytes.
    for encoding in ["identity", "Identity", ""] {
        let response = fetch_download(
            200,
            Some("text/csv"),
            "email,status\n",
            &[("content-encoding", encoding)],
        );
        assert!(response.is_ok(), "{encoding:?}: {response:?}");
    }
}

#[test]
fn test_bulk_validate_result_redirect_is_refused_without_a_content_type() {
    for status in [301, 302, 303, 307, 308] {
        // No Content-Type header, and a Location that must not be followed.
        let response = fetch_download(
            status,
            None,
            "",
            &[("location", "http://127.0.0.1:1/elsewhere")],
        );
        let Err(ZBError::ApiError { status: seen, message }) = response else {
            panic!("{status}: expected ApiError, got {response:?}");
        };
        assert_eq!(seen, status as u16);
        assert_eq!(
            message,
            "ZeroBounce returned a redirect, which this client does not follow."
        );
    }
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

    let opts = ZBGetFileOptions {
        download_type: Some(zb_download_type::COMBINED.to_string()),
        activity_data: Some(true),
    };

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
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(!message.contains("expected"), "{message}");
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
    let ZBError::ApiError { .. } = zb_error else {
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

    // LIVE: {"success": false} on an HTTP 200 is a failure, not a result. The
    // API answers an unknown file id that way, with a 200.
    let response = zb_instance.bulk_validation_result_delete("mock_file_id");
    mock.assert();

    let zb_error = response.unwrap_err();
    assert_eq!(zb_error.status_code(), Some(200));
    let message = zb_error.to_string();
    assert!(message.len() < 500, "{message}");
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
    assert!(response_obj.success, "{:#?}", response_obj);
}



