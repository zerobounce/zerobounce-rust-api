use std::collections::HashMap;

use chrono::NaiveDate;
use serde_json::from_str;

use crate::{ZeroBounce, ZBUrlProvider};
use crate::utility::{ZBError, ZBResult};
use crate::utility::{ENDPOINT_ACTIVITY_DATA, ENDPOINT_API_USAGE, ENDPOINT_CREDITS};
use crate::utility::structures::{ActivityData, ApiUsage};

impl ZeroBounce {
    pub fn new(api_key: &str) -> ZeroBounce {
        ZeroBounce {
            api_key: api_key.to_string().clone(),
            client: reqwest::blocking::Client::default(),
            url_provider: ZBUrlProvider::default()
        }
    }

    fn generic_get_request(
        &self, endpoint: &str, query_args: HashMap<&str, String>
    ) -> ZBResult<String>
    {

        let url = self.url_provider.url_of(endpoint);

        let response = self.client.get(url)
            .query(&query_args)
            .send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        if !response_ok {
            return Err(ZBError::explicit(response_content.as_str()));
        }

        Ok(response_content)
    }

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
            ("api_key", self.api_key.clone()),
        ]);

        let response_content = self.generic_get_request(
            ENDPOINT_CREDITS, query_args
        )?;

        Self::get_credits_from_string(response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date:NaiveDate) -> ZBResult<ApiUsage> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.clone()),
            ("start_date", start_date.format("%F").to_string()),
            ("end_date", end_date.format("%F").to_string()),
        ]);

        let response_content = self.generic_get_request(ENDPOINT_API_USAGE, query_args)?;

        let api_usage = from_str::<ApiUsage>(&response_content)?;
        Ok(api_usage)
    }

    pub fn get_api_usage_overall(&self) -> ZBResult<ApiUsage> {
        self.get_api_usage(NaiveDate::MIN, NaiveDate::MAX)
    }

    pub fn get_activity_data(&self, email: &str) -> ZBResult<ActivityData> {
        let query_args = HashMap::from([
            ("api_key", self.api_key.clone()),
            ("email", email.to_string().clone()),
        ]);

        let response_content = self.generic_get_request(
            ENDPOINT_ACTIVITY_DATA, query_args
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