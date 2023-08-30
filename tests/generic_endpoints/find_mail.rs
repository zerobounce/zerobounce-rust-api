use zero_bounce::utility::{ENDPOINT_EMAIL_FINDER, CONTENT_TYPE_JSON, ZBError, mock_constants};

use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};


#[test]
fn test_find_mail_client_error() {
    // no mock server
    let zb_instance = invalid_url_zb_instance();
    let find_mail_res = zb_instance.find_email(
        "example.com", "John", "", "Doe"
    );
    assert!(find_mail_res.is_err());

    let zb_error = find_mail_res.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_mail_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_body("")
        .create();

    let find_mail_res = zb_instance.find_email(
        "example.com", "John", "", "Doe"
    );
    assert!(find_mail_res.is_err());
    mock.assert();

    let zb_error = find_mail_res.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let find_mail_res = zb_instance.find_email(
        "example.com", "John", "", "Doe"
    );
    assert!(find_mail_res.is_err());
    mock.assert();

    let zb_error = find_mail_res.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_invalid_status_payload() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_MAIL_INVALID.clone())
        .create();

    let find_mail_res = zb_instance.find_email(
        "example.com", "John", "", "Doe"
    );
    assert!(find_mail_res.is_ok());
    mock.assert();

    let find_mail_object = find_mail_res.unwrap();
    assert_eq!(find_mail_object.status, "invalid");
    assert_eq!(find_mail_object.email, "");
    assert_eq!(find_mail_object.confidence, "undetermined");
    assert_eq!(find_mail_object.other_domain_formats.len(), 0);
}

#[test]
fn test_find_valid_status_payload() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_MAIL_VALID.clone())
        .create();

    let find_mail_res = zb_instance.find_email(
        "example.com", "John", "", "Doe"
    );
    assert!(find_mail_res.is_ok());
    mock.assert();

    let find_mail_object = find_mail_res.unwrap();
    assert_eq!(find_mail_object.status, "valid");
    assert_eq!(find_mail_object.email, "john.doe@example.com");
    assert_eq!(find_mail_object.confidence, "high");
    assert_eq!(find_mail_object.other_domain_formats.len(), 2);
    assert_eq!(find_mail_object.other_domain_formats[0].confidence, "high");
    assert_eq!(find_mail_object.other_domain_formats[1].confidence, "medium");
}


#[test]
fn test_domain_search_client_error() {
    // no mock server
    let zb_instance = invalid_url_zb_instance();
    let domain_search_res = zb_instance.domain_search("example.com");
    assert!(domain_search_res.is_err());

    let zb_error = domain_search_res.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_body("")
        .create();

    let domain_search_res = zb_instance.domain_search("example.com");
    assert!(domain_search_res.is_err());
    mock.assert();

    let zb_error = domain_search_res.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_bad_request() {

    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let domain_search_res = zb_instance.domain_search("example.com");
    assert!(domain_search_res.is_err());
    mock.assert();

    let zb_error = domain_search_res.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_invalid_status_payload() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_MAIL_INVALID.clone())
        .create();

    let domain_search_res = zb_instance.domain_search("example.com");
    assert!(domain_search_res.is_ok());
    mock.assert();

    let domain_search_object = domain_search_res.unwrap();
    assert_eq!(domain_search_object.status, "invalid");
    assert_eq!(domain_search_object.email, "");
    assert_eq!(domain_search_object.confidence, "undetermined");
    assert_eq!(domain_search_object.other_domain_formats.len(), 0);
}

#[test]
fn test_domain_search_valid_status_payload() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_MAIL_VALID.clone())
        .create();

    let domain_search_obj = zb_instance.domain_search("example.com");
    assert!(domain_search_obj.is_ok());
    mock.assert();

    let domain_search_object = domain_search_obj.unwrap();
    assert_eq!(domain_search_object.status, "valid");
    assert_eq!(domain_search_object.email, "john.doe@example.com");
    assert_eq!(domain_search_object.confidence, "high");
    assert_eq!(domain_search_object.other_domain_formats.len(), 2);
    assert_eq!(domain_search_object.other_domain_formats[0].confidence, "high");
    assert_eq!(domain_search_object.other_domain_formats[1].confidence, "medium");
}

