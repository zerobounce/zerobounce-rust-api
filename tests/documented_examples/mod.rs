//! Examples copied verbatim from zerobounce.net's API reference. Each must parse
//! through the real call (ACCEPT), so a documented field the SDK cannot read fails here.

use chrono::NaiveDate;

use zero_bounce::utility::{CONTENT_TYPE_JSON, ENDPOINT_API_USAGE, ENDPOINT_BATCH_VALIDATE, ENDPOINT_FILE_STATUS};

use crate::common::{endpoint_matcher, instantiate};

const BATCH_VALIDATE_SUCCESS: &str = r##"
{
      "email_batch": [
          {
              "address": "john.doe@zerobounce.net",
              "status": "valid",
              "sub_status": "",
              "free_email": false,
              "catchall_domain": true,
              "did_you_mean": null,
              "account": "johndoe",
              "domain": "zerobounce.net",
              "domain_age_days": "3431",
              "active_in_days": "30",
              "active_first_seen": "2026-02-25",
              "smtp_provider": "g-suite",
              "mx_found": "true",
              "mx_record": "aspmx.l.google.com",
              "firstname": "dan",
              "lastname": "cotet",
              "gender": "male",
              "country": null,
              "region": null,
              "city": null,
              "zipcode": null,
              "processed_at": "2026-06-18 07:05:00.033"
          },
          {
              "address": "invalid@example.com",
              "status": "invalid",
              "sub_status": "mailbox_not_found",
              "free_email": false,
              "did_you_mean": null,
              "account": null,
              "domain": null,
              "domain_age_days": "9692",
              "active_in_days": "365",
              "active_first_seen": "2026-02-25",
              "smtp_provider": "example",
              "mx_found": "true",
              "mx_record": "mx.example.com",
              "catchall_domain": false,
              "firstname": "zero",
              "lastname": "bounce",
              "gender": "male",
              "country": null,
              "region": null,
              "city": null,
              "zipcode": null,
              "processed_at": "2020-09-17 17:43:11.830"
          },
          {
              "address": "disposable@example.com",
              "status": "do_not_mail",
              "sub_status": "disposable",
              "free_email": false,
              "did_you_mean": null,
              "account": null,
              "domain": null,
              "domain_age_days": "9692",
              "active_in_days": "365",
              "active_first_seen": "2026-02-25",
              "smtp_provider": "example",
              "mx_found": "true",
              "mx_record": "mx.example.com",
              "catchall_domain": false,
              "firstname": "zero",
              "lastname": "bounce",
              "gender": "male",
              "country": null,
              "region": null,
              "city": null,
              "zipcode": null,
              "processed_at": "2020-09-17 17:43:11.830"
          }
      ],
      "errors": []
  }
"##;

const API_USAGE_SUCCESS: &str = r##"
{
      "total": 120,
      "status_valid": 17,
      "status_invalid": 69,
      "status_catch_all": 9,
      "status_do_not_mail": 16,
      "status_spamtrap": 0,
      "status_unknown": 6,
      "sub_status_toxic": 0,
      "sub_status_disposable": 1,
      "sub_status_role_based": 1,
      "sub_status_possible_trap": 0,
      "sub_status_global_suppression": 14,
      "sub_status_timeout_exceeded": 0,
      "sub_status_mail_server_temporary_error": 0,
      "sub_status_mail_server_did_not_respond": 0,
      "sub_status_greylisted": 6,
      "sub_status_antispam_system": 0,
      "sub_status_does_not_accept_mail": 27,
      "sub_status_exception_occurred": 0,
      "sub_status_failed_syntax_check": 20,
      "sub_status_mailbox_not_found": 12,
      "sub_status_unroutable_ip_address": 0,
      "sub_status_possible_typo": 0,
      "sub_status_no_dns_entries": 10,
      "sub_status_role_based_catch_all": 0,
      "sub_status_mailbox_quota_exceeded": 0,
      "sub_status_forcible_disconnect": 0,
      "sub_status_failed_smtp_connection": 0,
      "sub_status_leading_period_removed": 0,
      "sub_status_alias_address": 2,
      "sub_status_mx_forward": 0,
      "sub_status_alternate": 0,
      "sub_status_blocked": 0,
      "sub_status_allowed": 0,
      "start_date": "1/1/2023",
      "end_date": "1/30/2024"
    }
"##;

const FILE_STATUS_PHASE_2_NA: &str = r##"
{
    "success": true,
    "file_id": "aaaaaaaa-zzzz-xxxx-yyyy-5003727fffff",
    "file_name": "Your file name.csv",
    "upload_date": "2023-04-28T15:25:41Z",
    "file_status": "Complete",
    "file_phase_2_status": "N/A",
    "complete_percentage": "100%",
    "return_url": "Your return URL if provided when calling sendfile API"
  }
"##;

#[test]
fn zerobounce_net_batch_validate_success_example_parses() {
    let (mut mock_server, zb_instance) = instantiate();
    let mock = mock_server.mock("POST", endpoint_matcher(ENDPOINT_BATCH_VALIDATE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(BATCH_VALIDATE_SUCCESS)
        .create();

    let emails = vec![("john.doe@zerobounce.net".to_string(), String::new())];
    let batch = zb_instance.batch_validate(emails);
    mock.assert();
    let batch = batch.expect("the documented batch-validate answer parses");
    assert_eq!(batch.email_batch.len(), 3);
    assert_eq!(batch.email_batch[2].status, "do_not_mail");
    assert!(batch.errors.is_empty());
}

#[test]
fn zerobounce_net_get_api_usage_success_example_parses() {
    let (mut mock_server, zb_instance) = instantiate();
    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_API_USAGE))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(API_USAGE_SUCCESS)
        .create();

    let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 30).unwrap();
    let usage = zb_instance.get_api_usage(start, end);
    mock.assert();
    let usage = usage.expect("the documented get-api-usage answer parses");
    assert_eq!(usage.total, 120);
    assert_eq!(usage.sub_status_alias_address, 2);
    assert_eq!(usage.sub_status_leading_period_removed, 0);
}

#[test]
fn zerobounce_net_file_status_with_phase_2_na_example_parses() {
    let (mut mock_server, zb_instance) = instantiate();
    let mock = mock_server.mock("GET", endpoint_matcher(ENDPOINT_FILE_STATUS))
        .with_status(200)
        .with_header("content-type", CONTENT_TYPE_JSON)
        .with_body(FILE_STATUS_PHASE_2_NA)
        .create();

    let status = zb_instance.bulk_validation_file_status_check("aaaaaaaa-zzzz-xxxx-yyyy-5003727fffff");
    mock.assert();
    let status = status.expect("the documented file-status answer with N/A parses");
    assert_eq!(status.file_phase_2_status.as_deref(), Some("N/A"));
    assert_eq!(status.complete_percentage, 100.0);
}
