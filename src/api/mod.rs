use std::{collections::HashMap};

use chrono::NaiveDate;
use serde_json::from_str;

use crate::utility::{ZBError, ZBResult, structures::ApiUsage};

pub trait ZBUrlProvider {
    fn url_of(&self, endpoint: &str) -> String;
    fn bulk_url_of(&self, endpoint: &str) -> String;
}

pub struct ZBDefaultUrlProvider;

impl ZBUrlProvider for ZBDefaultUrlProvider {
    fn url_of(&self, endpoint: &str) -> String {
        return crate::utility::URI.to_owned() + endpoint;
    }
    fn bulk_url_of(&self, endpoint: &str) -> String {
        return crate::utility::BULK_URI.to_owned() + endpoint;
    }
}

pub struct ZeroBounce<'zb> {
    api_key: &'zb str,
    client: reqwest::blocking::Client,
    url_provider: Box<dyn ZBUrlProvider>,
}

impl ZeroBounce<'_> {
    pub fn new(api_key: &str, client: reqwest::blocking::Client) -> ZeroBounce {
        let url_provider = Box::new(ZBDefaultUrlProvider{});
        ZeroBounce {api_key, client, url_provider}
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
        let mut query_args = HashMap::<&str, &str>::new();

        query_args.insert("api_key", self.api_key);

        let response = self.client.get(url)
            .query(&query_args)
            .send()?;

        let response_content = response
            .text()?;

        Self::get_credits_from_string(response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date:NaiveDate) -> ZBResult<ApiUsage> {
        let url = self.url_provider.url_of(crate::utility::ENDPOINT_API_USAGE);

        let mut query_args = HashMap::<&str, &str>::new();
        let start_date_str = start_date.format("%F").to_string();
        let end_date_str = end_date.format("%F").to_string();

        query_args.insert("api_key", self.api_key);
        query_args.insert("start_date", &start_date_str);
        query_args.insert("end_date", &end_date_str);

        let response = self.client.get(url)
            .query(&query_args)
            .send()?;

        let response_content = response.text()?;
        let api_usage = from_str::<ApiUsage>(&response_content)?;
        Ok(api_usage)
    }

    pub fn get_api_usage_overall(&self) -> ZBResult<ApiUsage> {
        self.get_api_usage(NaiveDate::MIN, NaiveDate::MAX)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_integration_test() {
        let key = "c1b432f97c0145d0aca98b2f80b044da";
        let credits = ZeroBounce::new(key, reqwest::blocking::Client::new())
            .get_credits();
        assert!(credits.is_ok());

        println!("Credits amount: {:#?}", credits.ok());
    }

}