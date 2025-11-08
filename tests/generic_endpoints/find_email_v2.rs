use zero_bounce::utility::{ENDPOINT_EMAIL_FINDER, CONTENT_TYPE_JSON, ZBError, mock_constants};

use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};

#[test]
fn test_find_email_v2_empty_first_name() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.find_email_v2("", "example.com", None, None, None);
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("first_name"));
}

#[test]
fn test_find_email_v2_both_domain_and_company() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.find_email_v2("John", "example.com", "Example Inc", None, None);
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("exactly one"));
}

#[test]
fn test_find_email_v2_neither_domain_nor_company() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.find_email_v2("John", None, None, None, None);
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("either domain or company_name"));
}

#[test]
fn test_find_email_v2_empty_domain() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.find_email_v2("John", "", None, None, None);
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("domain"));
}

#[test]
fn test_find_email_v2_empty_company_name() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.find_email_v2("John", None, "", None, None);
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("company_name"));
}

#[test]
fn test_find_email_v2_client_error() {
    let zb_instance = invalid_url_zb_instance();
    let result = zb_instance.find_email_v2("John", "example.com", None, None, "Doe");
    assert!(result.is_err());

    let zb_error = result.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_email_v2_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_body("")
        .create();

    let result = zb_instance.find_email_v2("John", "example.com", None, None, "Doe");
    assert!(result.is_err());
    mock.assert();

    let zb_error = result.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_email_v2_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("first_name".to_string(), "John".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("last_name".to_string(), "Doe".to_string()))
        .create();

    let result = zb_instance.find_email_v2("John", "example.com", None, None, "Doe");
    assert!(result.is_err());
    mock.assert();

    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_find_email_v2_with_domain_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_EMAIL_V2_DOMAIN_VALID)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("first_name".to_string(), "John".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("last_name".to_string(), "Doe".to_string()))
        .create();

    let result = zb_instance.find_email_v2("John", "example.com", None, None, "Doe");
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.email, "john.doe@example.com");
    assert_eq!(response.confidence, "high");
    assert_eq!(response.company_name, "Internet Assigned Numbers Authority");
}

#[test]
fn test_find_email_v2_with_company_name_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_EMAIL_V2_COMPANY_VALID)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("first_name".to_string(), "John".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("company_name".to_string(), "Example Inc".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("last_name".to_string(), "Doe".to_string()))
        .create();

    let result = zb_instance.find_email_v2("John", None, "Example Inc", None, "Doe");
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.email, "john.doe@example.com");
    assert_eq!(response.confidence, "high");
    assert_eq!(response.company_name, "Example Inc");
    assert_eq!(response.domain, "betheexample.org");
}

#[test]
fn test_find_email_v2_with_middle_name() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_EMAIL_V2_DOMAIN_VALID)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("first_name".to_string(), "John".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("middle_name".to_string(), "Middle".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("last_name".to_string(), "Doe".to_string()))
        .create();

    let result = zb_instance.find_email_v2("John", "example.com", None, "Middle", "Doe");
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.email, "john.doe@example.com");
    assert_eq!(response.confidence, "high");
}

#[test]
fn test_find_email_v2_invalid_response() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_FIND_EMAIL_V2_INVALID)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("first_name".to_string(), "John".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("company_name".to_string(), "Example Inc".to_string()))
        .create();

    let result = zb_instance.find_email_v2("John", None, "Example Inc", None, None);
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.email, "");
    assert_eq!(response.confidence, "undetermined");
}

