use chrono::NaiveDate;
use serde::{Deserialize, de::Error as SerdeError};


pub(crate) fn deserialize_only_date<'de, D>(
    deserializer: D,
) -> Result<NaiveDate, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string: &str = serde::Deserialize::deserialize(deserializer)?;
    let stripped_string = string.replace('"', "");
	NaiveDate::parse_from_str(stripped_string.as_str(), "%m/%d/%Y").map_err(SerdeError::custom)
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiUsage {
	pub total: u64,
	pub status_valid: u64,
	pub status_invalid: u64,
	pub status_catch_all: u64,
	pub status_do_not_mail: u64,
	pub status_spamtrap: u64,
	pub status_unknown: u64,
	pub sub_status_toxic: u64,
	pub sub_status_disposable: u64,
	pub sub_status_role_based: u64,
	pub sub_status_possible_trap: u64,
	pub sub_status_global_suppression: u64,
	pub sub_status_timeout_exceeded: u64,
	pub sub_status_mail_server_temporary_error: u64,
	pub sub_status_mail_server_did_not_respond: u64,
	pub sub_status_greylisted: u64,
	pub sub_status_antispam_system: u64,
	pub sub_status_does_not_accept_mail: u64,
	pub sub_status_exception_occurred: u64,
	pub sub_status_failed_syntax_check: u64,
	pub sub_status_mailbox_not_found: u64,
	pub sub_status_unroutable_ip_address: u64,
	pub sub_status_possible_typo: u64,
	pub sub_status_no_dns_entries: u64,
	pub sub_status_role_based_catch_all: u64,
	pub sub_status_mailbox_quota_exceeded: u64,
	pub sub_status_forcible_disconnect: u64,
	pub sub_status_failed_smtp_connection: u64,
	pub sub_status_mx_forward: u64,
	
	#[serde(deserialize_with="deserialize_only_date")]
	pub start_date: NaiveDate,
	
	#[serde(deserialize_with="deserialize_only_date")]
	pub end_date: NaiveDate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utility::mock_constants::API_USAGE_RESPONSE;

    #[test]
    fn json_parsing_test() {
        let api_usage: serde_json::Result<ApiUsage> = serde_json::from_str(API_USAGE_RESPONSE);
        assert!(api_usage.is_ok());

        let api_usage_obj = api_usage.unwrap();
        let expected_start_date = NaiveDate::from_ymd_opt(2010, 1, 12).unwrap();
        let expected_end_date = NaiveDate::from_ymd_opt(2030, 12, 1).unwrap();
        assert_eq!(api_usage_obj.start_date, expected_start_date);
        assert_eq!(api_usage_obj.end_date, expected_end_date);
	}
}