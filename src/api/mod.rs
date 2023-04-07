pub mod bulk;
pub mod validation;

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use serde_json::from_str;

pub use crate::ZeroBounce;
use crate::utility::{ZBError, ZBResult};
use crate::utility::structures::{ActivityData, ApiUsage};
use crate::utility::{ENDPOINT_ACTIVITY_DATA, ENDPOINT_API_USAGE, ENDPOINT_CREDITS};

impl ZeroBounce {

    fn get_credits_from_string(string_value: String) -> ZBResult<i64> {
        from_str::<serde_json::Value>(string_value.as_ref())?
            .get("Credits")
            .and_then(serde_json::Value::as_str)
            .map(str::parse::<i64>)
            .ok_or(ZBError::explicit("credits value not in json"))?
            .map_err(ZBError::IntParseError)
    }

    pub fn get_credits(&self) -> ZBResult<i64> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_CREDITS), query_args
        )?;

        Self::get_credits_from_string(response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date: NaiveDate) -> ZBResult<ApiUsage> {
        let start_date_str = start_date.format("%F").to_string();
        let end_date_str = end_date.format("%F").to_string();
        let query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
            ("start_date", start_date_str.as_str()),
            ("end_date", end_date_str.as_str()),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_API_USAGE), query_args
        )?;

        let api_usage = from_str::<ApiUsage>(&response_content)?;
        Ok(api_usage)
    }

    pub fn get_api_usage_overall(&self) -> ZBResult<ApiUsage> {
        let start_date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end_date = Utc::now().naive_local().date();
        self.get_api_usage(start_date, end_date)
    }

    pub fn get_activity_data(&self, email: &str) -> ZBResult<ActivityData> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
            ("email", email),
        ]);

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_ACTIVITY_DATA), query_args
        )?;

        let activity_data = from_str::<ActivityData>(&response_content)?;
        Ok(activity_data)
    }

}

#[cfg(test)]
mod tests {
    use crate::utility::mock_constants::CREDITS_RESPONSE_OK;
    use crate::utility::mock_constants::CREDITS_RESPONSE_NEGATIVE;

    use super::*;

    #[test]
    fn test_credits_negative() {
        let credits = ZeroBounce::get_credits_from_string(CREDITS_RESPONSE_NEGATIVE.to_string());
        assert!(credits.is_ok());

        let amount = credits.unwrap();
        assert_eq!(amount, -1);
    }

    #[test]
    fn test_credits_ok() {
        let credits = ZeroBounce::get_credits_from_string(CREDITS_RESPONSE_OK.to_string());
        assert!(credits.is_ok());

        let amount = credits.unwrap();
        assert_eq!(amount, 123456);
    }

}