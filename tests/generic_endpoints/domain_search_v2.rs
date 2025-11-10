use zero_bounce::utility::{ENDPOINT_EMAIL_FINDER, CONTENT_TYPE_JSON, ZBError, mock_constants};

use crate::common::{instantiate, invalid_url_zb_instance, endpoint_matcher};

#[test]
fn test_domain_search_v2_both_domain_and_company() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .company_name("Example Inc")
        .call();
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("exactly one"));
}

#[test]
fn test_domain_search_v2_neither_domain_nor_company() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.domain_search_v2()
        .call();
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("either domain or company_name"));
}

#[test]
fn test_domain_search_v2_empty_domain() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.domain_search_v2()
        .domain("")
        .call();
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("domain"));
}

#[test]
fn test_domain_search_v2_empty_company_name() {
    let (mut _mock_server, zb_instance) = instantiate();
    
    let result = zb_instance.domain_search_v2()
        .company_name("")
        .call();
    assert!(result.is_err());
    
    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(msg) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
    assert!(msg.contains("company_name"));
}

#[test]
fn test_domain_search_v2_client_error() {
    let zb_instance = invalid_url_zb_instance();
    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .call();
    assert!(result.is_err());

    let zb_error = result.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_v2_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_body("")
        .create();

    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .call();
    assert!(result.is_err());
    mock.assert();

    let zb_error = result.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_v2_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(400)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::INVALID_API_RESPONSE)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .create();

    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .call();
    assert!(result.is_err());
    mock.assert();

    let zb_error = result.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_domain_search_v2_with_domain_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_DOMAIN_SEARCH_V2_DOMAIN)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .create();

    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .call();
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.domain, "example.com");
    assert_eq!(response.company_name, "");
    assert_eq!(response.format, "unknown");
    assert_eq!(response.confidence, "undetermined");
    assert_eq!(response.other_domain_formats.len(), 0);
}

#[test]
fn test_domain_search_v2_with_company_name_ok() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_DOMAIN_SEARCH_V2_COMPANY)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("company_name".to_string(), "Example Inc".to_string()))
        .create();

    let result = zb_instance.domain_search_v2()
        .company_name("Example Inc")
        .call();
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.domain, "betheexample.org");
    assert_eq!(response.company_name, "Example Inc");
    assert_eq!(response.format, "unknown");
    assert_eq!(response.confidence, "undetermined");
    assert_eq!(response.other_domain_formats.len(), 0);
}

#[test]
fn test_domain_search_v2_with_formats() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_EMAIL_FINDER))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(mock_constants::MOCK_DOMAIN_SEARCH_V2_WITH_FORMATS)
        .match_query(mockito::Matcher::UrlEncoded("api_key".to_string(), "mock_api_key".to_string()))
        .match_query(mockito::Matcher::UrlEncoded("domain".to_string(), "example.com".to_string()))
        .create();

    let result = zb_instance.domain_search_v2()
        .domain("example.com")
        .call();
    assert!(result.is_ok());
    mock.assert();

    let response = result.unwrap();
    assert_eq!(response.domain, "example.com");
    assert_eq!(response.format, "first.last");
    assert_eq!(response.confidence, "high");
    assert_eq!(response.other_domain_formats.len(), 2);
    assert_eq!(response.other_domain_formats[0].format, "first_last");
    assert_eq!(response.other_domain_formats[0].confidence, "high");
    assert_eq!(response.other_domain_formats[1].format, "first");
    assert_eq!(response.other_domain_formats[1].confidence, "medium");
}

