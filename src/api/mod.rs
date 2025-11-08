pub mod bulk;
pub mod validation;

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use serde_json::from_str;

pub use crate::ZeroBounce;
use crate::utility::structures::generic::{FindEmailResponse, FindEmailResponseV2, DomainSearchResponseV2};
use crate::utility::{ZBError, ZBResult, ENDPOINT_EMAIL_FINDER};
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

    /// Deprecated: Use `find_email_v2` instead.
    /// 
    /// This function is kept for backward compatibility but will be removed in a future version.
    #[deprecated(
        since = "1.2.0",
        note = "Use `find_email_v2` instead. The new version supports both domain and company_name parameters."
    )]
    pub fn find_email(&self, domain: &str, first_name: &str, middle_name: &str, last_name: &str) -> ZBResult<FindEmailResponse> {
        let mut query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
            ("domain", domain),
        ]);
        if !first_name.is_empty() {
            query_args.insert("first_name", first_name);
        }
        if !middle_name.is_empty() {
            query_args.insert("middle_name", middle_name);
        }
        if !last_name.is_empty() {
            query_args.insert("last_name", last_name);
        }

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args
        )?;

        let activity_data = from_str::<FindEmailResponse>(&response_content)?;
        Ok(activity_data)
    }

    /// Find an email address using either a domain or company name.
    /// 
    /// # Parameters
    /// - `first_name`: Mandatory first name
    /// - `domain`: Optional domain name (e.g., "example.com")
    /// - `company_name`: Optional company name (e.g., "Example Inc")
    /// - `middle_name`: Optional middle name
    /// - `last_name`: Optional last name
    /// 
    /// # Requirements
    /// Exactly one of `domain` or `company_name` must be provided (XOR requirement).
    /// 
    /// # Example
    /// ```
    /// use zero_bounce::ZeroBounce;
    /// 
    /// let zb = ZeroBounce::new("your_api_key");
    /// // Using domain
    /// let result = zb.find_email_v2("John", "example.com", None, None, "Doe");
    /// // Or using company name
    /// let result = zb.find_email_v2("John", None, "Example Inc", None, "Doe");
    /// ```
    pub fn find_email_v2<'a>(
        &self,
        first_name: &str,
        domain: impl Into<Option<&'a str>>,
        company_name: impl Into<Option<&'a str>>,
        middle_name: impl Into<Option<&'a str>>,
        last_name: impl Into<Option<&'a str>>,
    ) -> ZBResult<FindEmailResponseV2> {
        let domain = domain.into();
        let company_name = company_name.into();
        let middle_name = middle_name.into();
        let last_name = last_name.into();
        if first_name.is_empty() {
            return Err(ZBError::explicit("first_name is mandatory and cannot be empty"));
        }

        match (domain, company_name) {
            (Some(d), None) => {
                if d.is_empty() {
                    return Err(ZBError::explicit("domain cannot be empty"));
                }
            }
            (None, Some(c)) => {
                if c.is_empty() {
                    return Err(ZBError::explicit("company_name cannot be empty"));
                }
            }
            (Some(_), Some(_)) => {
                return Err(ZBError::explicit("exactly one of domain or company_name must be provided, not both"));
            }
            (None, None) => {
                return Err(ZBError::explicit("either domain or company_name must be provided"));
            }
        }

        let mut query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
            ("first_name", first_name),
        ]);

        if let Some(d) = domain {
            query_args.insert("domain", d);
        }

        if let Some(c) = company_name {
            query_args.insert("company_name", c);
        }

        if let Some(middle) = middle_name {
            if !middle.is_empty() {
                query_args.insert("middle_name", middle);
            }
        }

        if let Some(last) = last_name {
            if !last.is_empty() {
                query_args.insert("last_name", last);
            }
        }

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args
        )?;

        // Debug: Print raw response to examine structure in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("Raw API response: {}", response_content);
        }

        let find_email_response = from_str::<FindEmailResponseV2>(&response_content)?;
        Ok(find_email_response)
    }

    /// Deprecated: Use `domain_search_v2` instead.
    /// 
    /// This function is kept for backward compatibility but will be removed in a future version.
    #[deprecated(
        since = "1.2.0",
        note = "Use `domain_search_v2` instead. The new version supports both domain and company_name parameters."
    )]
    pub fn domain_search(&self, domain: &str) -> ZBResult<FindEmailResponse> {
        self.find_email(domain, "", "", "")
    }

    /// Search for email formats using either a domain or company name.
    /// 
    /// # Parameters
    /// - `domain`: Optional domain name (e.g., "example.com")
    /// - `company_name`: Optional company name (e.g., "Example Inc")
    /// 
    /// # Requirements
    /// Exactly one of `domain` or `company_name` must be provided (XOR requirement).
    /// 
    /// # Example
    /// ```
    /// use zero_bounce::ZeroBounce;
    /// 
    /// let zb = ZeroBounce::new("your_api_key");
    /// // Using domain
    /// let result = zb.domain_search_v2("example.com", None);
    /// // Or using company name
    /// let result = zb.domain_search_v2(None, "Example Inc");
    /// ```
    pub fn domain_search_v2<'a>(
        &self,
        domain: impl Into<Option<&'a str>>,
        company_name: impl Into<Option<&'a str>>,
    ) -> ZBResult<DomainSearchResponseV2> {
        let domain = domain.into();
        let company_name = company_name.into();
        match (domain, company_name) {
            (Some(d), None) => {
                if d.is_empty() {
                    return Err(ZBError::explicit("domain cannot be empty"));
                }
            }
            (None, Some(c)) => {
                if c.is_empty() {
                    return Err(ZBError::explicit("company_name cannot be empty"));
                }
            }
            (Some(_), Some(_)) => {
                return Err(ZBError::explicit("exactly one of domain or company_name must be provided, not both"));
            }
            (None, None) => {
                return Err(ZBError::explicit("either domain or company_name must be provided"));
            }
        }

        let mut query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
        ]);

        if let Some(d) = domain {
            query_args.insert("domain", d);
        }

        if let Some(c) = company_name {
            query_args.insert("company_name", c);
        }

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args
        )?;

        // Debug: Print raw response to examine structure in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("Raw API response: {}", response_content);
        }

        let domain_search_response = from_str::<DomainSearchResponseV2>(&response_content)?;
        Ok(domain_search_response)
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
