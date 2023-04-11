use chrono::NaiveDate;
use serde::Deserialize;

use crate::utility::structures::custom_deserialize::deserialize_only_date;
use crate::utility::structures::custom_deserialize::deserialize_stringified_uint;

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

#[derive(Clone, Debug, Deserialize)]
pub struct ActivityData {
    pub found: bool,

    #[serde(deserialize_with="deserialize_stringified_uint")]
    pub active_in_days: Option<u128>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utility::mock_constants::API_USAGE_RESPONSE;
    use crate::utility::mock_constants::ACTIVITY_DATA_RESPONSE_ACTIVE;
    use crate::utility::mock_constants::ACTIVITY_DATA_RESPONSE_INACTIVE;

    #[test]
    fn parse_activity_date_without_amount() {
        let activity_data_res = serde_json::from_str::<ActivityData>(
            ACTIVITY_DATA_RESPONSE_INACTIVE
        );
        assert!(activity_data_res.is_ok(), "error: {}", activity_data_res.unwrap_err());

        let activity_data = activity_data_res.unwrap();
        assert_eq!(activity_data.found, false);
        assert_eq!(activity_data.active_in_days, None);
    }

    #[test]
    fn parse_activity_date_with_amount() {
        let activity_data_res: serde_json::Result<ActivityData> = serde_json::from_str(ACTIVITY_DATA_RESPONSE_ACTIVE);
        assert!(activity_data_res.is_ok());

        let activity_data = activity_data_res.unwrap();
        assert_eq!(activity_data.found, true);
        assert_eq!(activity_data.active_in_days, Some(180));
    }

    #[test]
    fn parse_api_usage() {
        let api_usage: serde_json::Result<ApiUsage> = serde_json::from_str(API_USAGE_RESPONSE);
        assert!(api_usage.is_ok());

        let api_usage_obj = api_usage.unwrap();
        let expected_start_date = NaiveDate::from_ymd_opt(2010, 1, 12).unwrap();
        let expected_end_date = NaiveDate::from_ymd_opt(2030, 12, 1).unwrap();
        assert_eq!(api_usage_obj.start_date, expected_start_date);
        assert_eq!(api_usage_obj.end_date, expected_end_date);
    }
}
