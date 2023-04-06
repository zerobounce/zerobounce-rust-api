use std::collections::HashMap;

use crate::utility::{URI, ENDPOINT_CREDITS, ZBError};

pub struct ZeroBounce<'key> {
    api_key: &'key str,
    client: reqwest::blocking::Client
}

impl ZeroBounce<'_> {
    pub fn new(api_key: &str, client: reqwest::blocking::Client) -> ZeroBounce {
        ZeroBounce {api_key, client}
    }

    fn get_credits_from_string(string_value: String) -> Result<i64, ZBError> {

        let parsed_value = serde_json::from_str::<serde_json::Value>(string_value.as_ref())
            .map_err(ZBError::JsonError)?;

        parsed_value
            .get("Credits")
            .and_then(serde_json::Value::as_str)
            .map(str::parse::<i64>)
            .ok_or(ZBError::explicit("credits value not in json"))?
            .map_err(ZBError::IntParseError)
    }

    pub fn get_credits(&self) -> Result<i64, ZBError> {
        
        let url_string = URI.to_owned() + ENDPOINT_CREDITS;
        
        let mut query_args = HashMap::<&str, &str>::new();
        query_args.insert("api_key", self.api_key);

        let response = self.client.get(url_string)
            .query(&query_args)
            .send()
            .map_err(ZBError::RequestError)?;

        let response_ok = response.status().is_success();

        let response_content = response
            .text()
            .map_err(ZBError::RequestError)?;

        if !response_ok {
            return Err(ZBError::explicit(response_content.as_str()));
        }

        Self::get_credits_from_string(response_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_integration_test() {
        let key = "mock_api";
        let credits = ZeroBounce::new(key, reqwest::blocking::Client::new())
            .get_credits();
        assert!(credits.is_ok());

        println!("Credits amount: {:#?}", credits.ok());
    }

}