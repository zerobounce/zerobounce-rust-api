use zero_bounce::{ZBUrlProvider, ZeroBounce};
use zero_bounce::utility::{ENDPOINT_ACTIVITY_DATA, ZBError, mock_constants};

use crate::common::{INVALID_URL, MOCK_API_KEY};
use crate::common::{instantiate, endpoint_matcher};

const EMAIL: &str = "example@example.com";

#[test]
fn test_activity_data_client_error() {
    // no mock server
    let zb_instance = ZeroBounce {
        api_key: MOCK_API_KEY.to_string().clone(),
        client: reqwest::blocking::Client::default(),
        url_provider: ZBUrlProvider {
            url: INVALID_URL.to_owned(),
            bulk_url: INVALID_URL.to_owned(),
        },
    };

    let activity_data = zb_instance.get_activity_data(EMAIL);
    assert!(activity_data.is_err());

    let zb_error = activity_data.unwrap_err();
    let ZBError::RequestError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_activity_data_invalid_json() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_ACTIVITY_DATA))
        .with_status(200)
        .with_body("")
        .create();

    let activity_data = zb_instance.get_activity_data(EMAIL);
    assert!(activity_data.is_err());
    mock.assert();

    let zb_error = activity_data.unwrap_err();
    let ZBError::JsonError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_activity_data_bad_request() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_ACTIVITY_DATA))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::INVALID_API_RESPONSE.clone())
        .create();

    let activity_data = zb_instance.get_activity_data(EMAIL);
    assert!(activity_data.is_err());
    mock.assert();

    let zb_error = activity_data.unwrap_err();
    let ZBError::ExplicitError(_) = zb_error else {
        panic!("unexpected error: {:#?}", zb_error);
    };
}

#[test]
fn test_activity_data_ok_inactive() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_ACTIVITY_DATA))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::ACTIVITY_DATA_RESPONSE_INACTIVE.clone())
        .create();

    // start_date, end_date will not match the response, as the API should
    let activity_data = zb_instance.get_activity_data(EMAIL);
    assert!(activity_data.is_ok());
    mock.assert();

    let a_data = activity_data.unwrap();
    assert_eq!(a_data.found, false);
    assert_eq!(a_data.active_in_days, None);
}


#[test]
fn test_activity_data_ok_active() {
    let (mut mock_server, zb_instance) = instantiate();

    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_ACTIVITY_DATA))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_constants::ACTIVITY_DATA_RESPONSE_ACTIVE.clone())
        .create();

    // start_date, end_date will not match the response, as the API should
    let activity_data = zb_instance.get_activity_data(EMAIL);
    assert!(activity_data.is_ok());
    mock.assert();

    let a_data = activity_data.unwrap();
    assert_eq!(a_data.found, true);
    assert_eq!(a_data.active_in_days, Some(180));
}
