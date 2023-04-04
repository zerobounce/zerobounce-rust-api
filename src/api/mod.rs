use std::collections::HashMap;

use chrono::NaiveDate;
use serde_json::from_str;

use crate::utility::{ZBError, ZBResult, structures::ApiUsage};

// Structure meant to generate the URLs to be accessed with the HTTP requests
// based on the base API URLs (for the base API and bulk API).
pub struct ZBUrlProvider{
    pub url: String,
    pub bulk_url: String,
}

impl ZBUrlProvider {
    fn url_of(&self, endpoint: &str) -> String {
        return self.url.to_owned() + endpoint;
    }
}

impl Default for ZBUrlProvider {
    fn default() -> Self {
        ZBUrlProvider{
            url: crate::utility::URI.clone().to_string(),
            bulk_url: crate::utility::BULK_URI.clone().to_string(),
        }
    }
}

pub struct ZeroBounce {
    pub api_key: String,
    pub client: reqwest::blocking::Client,
    pub url_provider: ZBUrlProvider,
}

impl ZeroBounce {
    pub fn new(api_key: &str) -> ZeroBounce {
        ZeroBounce {
            api_key: api_key.to_string().clone(),
            client: reqwest::blocking::Client::default(),
            url_provider: ZBUrlProvider::default()
        }
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
        let url = self.url_provider.url_of(crate::utility::ENDPOINT_CREDITS);
        let mut query_args = HashMap::<&str, String>::new();

        query_args.insert("api_key", self.api_key.clone());

        let response = self.client.get(url)
            .query(&query_args)
            .send()?;

        let response_ok = response.status().is_success();
        let response_content = response
            .text()?;

        if !response_ok {
            return Err(ZBError::explicit(response_content.as_str()));
        }

        Self::get_credits_from_string(response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date:NaiveDate) -> ZBResult<ApiUsage> {
        let url = self.url_provider.url_of(crate::utility::ENDPOINT_API_USAGE);

        let mut query_args = HashMap::<&str, String>::new();
        let start_date_str = start_date.format("%F").to_string();
        let end_date_str = end_date.format("%F").to_string();

        query_args.insert("api_key", self.api_key.clone());
        query_args.insert("start_date", start_date_str);
        query_args.insert("end_date", end_date_str);

        let response = self.client.get(url)
            .query(&query_args)
            .send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        if !response_ok {
            return Err(ZBError::explicit(response_content.as_str()));
        }

        let api_usage = from_str::<ApiUsage>(&response_content)?;
        Ok(api_usage)
    }

    pub fn get_api_usage_overall(&self) -> ZBResult<ApiUsage> {
        self.get_api_usage(NaiveDate::MIN, NaiveDate::MAX)
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