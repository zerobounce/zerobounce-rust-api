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