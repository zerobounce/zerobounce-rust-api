pub const API_USAGE_RESPONSE: &str = r#"
{
    "total": 10,
    "status_valid": 10,
    "status_invalid": 0,
    "status_catch_all": 0,
    "status_do_not_mail": 0,
    "status_spamtrap": 0,
    "status_unknown": 0,
    "sub_status_toxic": 0,
    "sub_status_disposable": 0,
    "sub_status_role_based": 0,
    "sub_status_possible_trap": 0,
    "sub_status_global_suppression": 0,
    "sub_status_timeout_exceeded": 0,
    "sub_status_mail_server_temporary_error": 0,
    "sub_status_mail_server_did_not_respond": 0,
    "sub_status_greylisted": 0,
    "sub_status_antispam_system": 0,
    "sub_status_does_not_accept_mail": 0,
    "sub_status_exception_occurred": 0,
    "sub_status_failed_syntax_check": 0,
    "sub_status_mailbox_not_found": 0,
    "sub_status_unroutable_ip_address": 0,
    "sub_status_possible_typo": 0,
    "sub_status_no_dns_entries": 0,
    "sub_status_role_based_catch_all": 0,
    "sub_status_mailbox_quota_exceeded": 0,
    "sub_status_forcible_disconnect": 0,
    "sub_status_failed_smtp_connection": 0,
    "sub_status_mx_forward": 0,
    "sub_status_alternate": 0,
    "sub_status_blocked": 0,
    "sub_status_allowed": 0,
    "start_date": "1/12/2010",
    "end_date": "12/1/2030"
}
"#;

pub const CREDITS_RESPONSE_OK: &str = r#"
{
    "Credits": "123456"
}
"#;

pub const CREDITS_RESPONSE_NEGATIVE: &str = r#"
{
    "Credits": "-1"
}
"#;

pub const INVALID_API_RESPONSE: &str = r#"
{
    "error": "Missing parameter: api_key."
}
"#;

pub const ACTIVITY_DATA_RESPONSE_ACTIVE: &str = r#"
{
    "found": true,
    "active_in_days": "180"
}
"#;

pub const ACTIVITY_DATA_RESPONSE_INACTIVE: &str = r#"
{
    "found": false,
    "active_in_days": null
}
"#;

pub const VALIDATION_RESPONSE_VALID: &str =  r#"
{
    "address": "valid@example.com",
    "status": "valid",
    "sub_status": "",
    "free_email": false,
    "did_you_mean": null,
    "account": null,
    "domain": null,
    "domain_age_days": "9692",
    "smtp_provider": "example",
    "mx_found": "true",
    "mx_record": "mx.example.com",
    "firstname": "zero",
    "lastname": "bounce",
    "gender": "male",
    "country": "United States",
    "region": "Florida",
    "city": "West Palm Beach",
    "zipcode": "33401",
    "processed_at": "2023-03-23 13:30:28.105"
}
"#;

pub const VALIDATION_RESPONSE_INVALID: &str = r#"
{
    "address": "invalid@example.com",
    "status": "invalid",
    "sub_status": "mailbox_not_found",
    "free_email": false,
    "did_you_mean": null,
    "account": null,
    "domain": null,
    "domain_age_days": "9692",
    "smtp_provider": "example",
    "mx_found": "true",
    "mx_record": "mx.example.com",
    "firstname": "zero",
    "lastname": "bounce",
    "gender": "male",
    "country": "United States",
    "region": "Florida",
    "city": "West Palm Beach",
    "zipcode": "33401",
    "processed_at": "2023-03-23 12:30:28.003"
}
"#;

pub const VALIDATION_RESPONSE_NULL_FIELDS: &str = r#"
{
    "address": "invalid@example.com",
    "status": "invalid",
    "sub_status": "failed_syntax_check",
    "free_email": false,
    "did_you_mean": null,
    "account": "",
    "domain": null,
    "domain_age_days": "",
    "smtp_provider": "",
    "mx_found": "false",
    "mx_record": null,
    "firstname": "",
    "lastname": "",
    "gender": "",
    "country": null,
    "region": null,
    "city": null,
    "zipcode": null,
    "processed_at": "2023-04-05 08:55:40.661"
}
"#;

pub const BATCH_VALIDATION_WITH_ERROR: &str = r#"
{
    "email_batch": [
        {
            "address": "valid@example.com",
            "status": "valid",
            "sub_status": "",
            "free_email": false,
            "did_you_mean": null,
            "account": null,
            "domain": null,
            "domain_age_days": "9692",
            "smtp_provider": "example",
            "mx_found": "true",
            "mx_record": "mx.example.com",
            "firstname": "zero",
            "lastname": "bounce",
            "gender": "male",
            "country": "United States",
            "region": "Florida",
            "city": "West Palm Beach",
            "zipcode": "33401",
            "processed_at": "2023-03-23 13:30:28.105"
        }
    ],
    "errors": [
        {
            "error": "Mock error message",
            "email_address": "invalid@example.com"
        }
    ]
}
"#;

pub const BATCH_VALIDATION_ERROR_ONLY: &str = r#"
{
    "email_batch": [],
    "errors": [
        {
            "error": "Mock error message",
            "email_address": "invalid@example.com"
        }
    ]
}
"#;

pub const BATCH_VALIDATION_NO_ERROR: &str = r#"
{
    "email_batch": [
        {
            "address": "valid@example.com",
            "status": "valid",
            "sub_status": "",
            "free_email": false,
            "did_you_mean": null,
            "account": null,
            "domain": null,
            "domain_age_days": "9692",
            "smtp_provider": "example",
            "mx_found": "true",
            "mx_record": "mx.example.com",
            "firstname": "zero",
            "lastname": "bounce",
            "gender": "male",
            "country": "United States",
            "region": "Florida",
            "city": "West Palm Beach",
            "zipcode": "33401",
            "processed_at": "2023-03-23 13:30:28.105"
        }
    ],
    "errors": []
}
"#;

pub const BULK_VALIDATION_SUBMIT_OK: &str = r#"
{
    "success": true,
    "message": "File Accepted",
    "file_name": "emails2.txt",
    "file_id": "e90e9b1d-8dc7-40eb-a7d9-999d52086a56"
}
"#;

pub const BULK_VALIDATION_SUBMIT_ERROR: &str = r#"
{
    "success": false,
    "error_message": "Over 50% of the file you uploaded contains emails in a wrong format, this is usually because you are uploading the wrong file, the file is in the wrong format or wrong column for email has been selected.",
    "message": "Over 50% of the file you uploaded contains emails in a wrong format, this is usually because you are uploading the wrong file, the file is in the wrong format or wrong column for email has been selected."
}
"#;

pub const BULK_VALIDATION_STATUS_OK: &str = r#"
{
    "success": true,
    "file_id": "e90e9b1d-8dc7-40eb-a7d9-999d52086a56",
    "file_name": "emails2.txt",
    "upload_date": "2023-04-26T17:52:23Z",
    "file_status": "Processing",
    "complete_percentage": "100%",
    "error_reason": null,
    "return_url": "https://mock.value.com/"
}
"#;

pub const BULK_VALIDATION_STATUS_DELETED: &str = r#"
{
    "success": true,
    "file_id": "e90e9b1d-8dc7-40eb-a7d9-999d52086a56",
    "file_name": "emails2.txt",
    "upload_date": "2023-04-26T17:52:23Z",
    "file_status": "Deleted",
    "complete_percentage": "0%",
    "error_reason": "mock value",
    "return_url": null
}
"#;

pub const BULK_VALIDATION_RESULT_DELETED: &str = r#"
{
    "success": false,
    "message": "File deleted."
}
"#;

pub const BULK_VALIDATION_DELETE_OK: &str = r#"
{
    "success": true,
    "message": "File Deleted",
    "file_name": "emails2.txt",
    "file_id": "e90e9b1d-8dc7-40eb-a7d9-999d52086a56"
}
"#;

pub const BULK_VALIDATION_DELETE_NOT_FOUND: &str = r#"
{
    "success": false,
    "message": "File cannot be found."
}
"#;

pub const FILE_FEEDBACK_SUPPORTED_VARIANT_1: &str = r#"
{
    "success": false,
    "message": [
        "Mock message 1",
        "Mock message 2"
    ]
}
"#;


pub const FILE_FEEDBACK_SUPPORTED_VARIANT_2: &str = r#"
{
    "success": false,
    "message": {
        "file_id": ["Mock message"]
    }
}
"#;
