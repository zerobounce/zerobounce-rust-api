pub mod structures;
pub mod mock_constants;

use serde::de::Error as SerdeError;

pub const URI: &str = "https://api.zerobounce.net/v2";
pub const BULK_URI: &str = "https://bulkapi.zerobounce.net/v2";
pub const ENDPOINT_CREDITS: &str = "/getcredits";
pub const ENDPOINT_ACTIVITY_DATA: &str = "/activity";
pub const ENDPOINT_VALIDATE: &str = "/validate";
pub const ENDPOINT_API_USAGE: &str = "/getapiusage";
pub const ENDPOINT_BATCH_VALIDATE: &str = "/validatebatch";
pub const ENDPOINT_FILE_SEND: &str = "/sendfile";
pub const ENDPOINT_FILE_STATUS: &str = "/filestatus";
pub const ENDPOINT_FILE_RESULT: &str = "/getfile";
pub const ENDPOINT_FILE_DELETE: &str = "/deletefile";
pub const ENDPOINT_SCORING_SEND: &str = "/scoring/sendfile";
pub const ENDPOINT_SCORING_STATUS: &str = "/scoring/filestatus";
pub const ENDPOINT_SCORING_RESULT: &str = "/scoring/getfile";
pub const ENDPOINT_SCORING_DELETE: &str = "/scoring/deletefile";
pub const SANDBOX_IP: &str = "99.110.204.1";

// validation statuses
pub const S_VALID: &str = "valid";
pub const S_INVALID: &str = "invalid";
pub const S_CATCH_ALL: &str = "catch-all";
pub const S_UNKNOWN: &str = "unknown";
pub const S_SPAMTRAP: &str = "spamtrap";
pub const S_ABUSE: &str = "abuse";
pub const S_DO_NOT_MAIL: &str = "do_not_mail";

// validation sub statuses
pub const SS_ANTISPAM_SYSTEM: &str = "antispam_system";
pub const SS_GREYLISTED: &str = "greylisted";
pub const SS_MAIL_SERVER_TEMPORARY_ERROR: &str = "mail_server_temporary_error";
pub const SS_FORCIBLE_DISCONNECT: &str = "forcible_disconnect";
pub const SS_MAIL_SERVER_DID_NOT_RESPOND: &str = "mail_server_did_not_respond";
pub const SS_TIMEOUT_EXCEEDED: &str = "timeout_exceeded";
pub const SS_FAILED_SMTP_CONNECTION: &str = "failed_smtp_connection";
pub const SS_MAILBOX_QUOTA_EXCEEDED: &str = "mailbox_quota_exceeded";
pub const SS_EXCEPTION_OCCURRED: &str = "exception_occurred";
pub const SS_POSSIBLE_TRAP: &str = "possible_trap";
pub const SS_ROLE_BASED: &str = "role_based";
pub const SS_GLOBAL_SUPPRESSION: &str = "global_suppression";
pub const SS_MAILBOX_NOT_FOUND: &str = "mailbox_not_found";
pub const SS_NO_DNS_ENTRIES: &str = "no_dns_entries";
pub const SS_FAILED_SYNTAX_CHECK: &str = "failed_syntax_check";
pub const SS_POSSIBLE_TYPO: &str = "possible_typo";
pub const SS_UNROUTABLE_IP_ADDRESS: &str = "unroutable_ip_address";
pub const SS_LEADING_PERIOD_REMOVED: &str = "leading_period_removed";
pub const SS_DOES_NOT_ACCEPT_MAIL: &str = "does_not_accept_mail";
pub const SS_ALIAS_ADDRESS: &str = "alias_address";
pub const SS_ROLE_BASED_CATCH_ALL: &str = "role_based_catch_all";
pub const SS_DISPOSABLE: &str = "disposable";
pub const SS_TOXIC: &str = "toxic";

#[derive(Debug)]
pub enum ZBError {
    ExplicitError(String),
    JsonError(serde_json::Error),
    IntParseError(std::num::ParseIntError),
    RequestError(reqwest::Error)
}

pub type ZBResult<T> = Result<T, ZBError>;

impl ZBError {
    pub fn explicit(string: &str) -> ZBError {
        ZBError::ExplicitError(string.to_string())
    }
}

// Implementation made in order to automatically convert errors
// generated by the reqwuest library into a ZBError instance.
//
// This automatic conversion is expected when using the client
// and issuing http requests.
impl From<reqwest::Error> for ZBError {
    fn from(value: reqwest::Error) -> ZBError {
        ZBError::RequestError(value)
    }
}

// Implementation made in order to automatically convert errors
// generated by the serde_json library into a ZBError instance.
//
// This automatic conversion is expected when deserializing.
impl From<serde_json::Error> for ZBError {
    fn from(value: serde_json::Error) -> ZBError {
        ZBError::JsonError(value)
    }
}


pub fn deserialize_date_rfc<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    chrono::DateTime::parse_from_rfc3339(string).map_err(SerdeError::custom)
}


pub fn parsed_percentage<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;

    let raw_error = SerdeError::custom("invalid percentage amount");
    let first_part = string.split('%').next().ok_or(raw_error)?;
    first_part
        .parse::<f32>()
        .map_err(|error| SerdeError::custom(error.to_string()))
}

// struct FileStatus {
//     success: bool,
//     file_id: String,
//     file_name: String,
//     #[serde(deserialize_with="deserialize_date_rfc")]
//     upload_date: String,
//     file_status: String,
//     complete_percentage: f32
//     error_reason
//     return_url
// }