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

/// Builder for the `find_email_v2` API call.
/// 
/// # Example
/// ```no_run
/// use zero_bounce::ZeroBounce;
/// use zero_bounce::utility::ZBResult;
/// 
/// # fn main() -> ZBResult<()> {
/// let zb = ZeroBounce::new("your_api_key");
/// let result = zb.find_email_v2()
///     .first_name("John")
///     .domain("example.com")
///     .last_name("Doe")
///     .call()?;
/// # Ok(())
/// # }
/// ```
pub struct FindEmailV2Builder<'a> {
    client: &'a ZeroBounce,
    first_name: Option<&'a str>,
    domain: Option<&'a str>,
    company_name: Option<&'a str>,
    middle_name: Option<&'a str>,
    last_name: Option<&'a str>,
}

/// Builder for the `domain_search_v2` API call.
/// 
/// # Example
/// ```no_run
/// use zero_bounce::ZeroBounce;
/// use zero_bounce::utility::ZBResult;
/// 
/// # fn main() -> ZBResult<()> {
/// let zb = ZeroBounce::new("your_api_key");
/// let result = zb.domain_search_v2()
///     .domain("example.com")
///     .call()?;
/// # Ok(())
/// # }
/// ```
pub struct DomainSearchV2Builder<'a> {
    client: &'a ZeroBounce,
    domain: Option<&'a str>,
    company_name: Option<&'a str>,
}

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
        let query_args = HashMap::new();

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_CREDITS), query_args
        )?;

        Self::get_credits_from_string(response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date: NaiveDate) -> ZBResult<ApiUsage> {
        let start_date_str = start_date.format("%F").to_string();
        let end_date_str = end_date.format("%F").to_string();
        let query_args = HashMap::from([
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
    /// Returns a builder that allows you to set parameters using method chaining.
    /// 
    /// # Requirements
    /// - `first_name` is mandatory
    /// - Exactly one of `domain` or `company_name` must be provided (XOR requirement)
    /// 
    /// # Example
    /// ```no_run
    /// use zero_bounce::ZeroBounce;
    /// use zero_bounce::utility::ZBResult;
    /// 
    /// # fn main() -> ZBResult<()> {
    /// let zb = ZeroBounce::new("your_api_key");
    /// // Using domain
    /// let result = zb.find_email_v2()
    ///     .first_name("John")
    ///     .domain("example.com")
    ///     .last_name("Doe")
    ///     .call()?;
    /// // Or using company name
    /// let result = zb.find_email_v2()
    ///     .first_name("John")
    ///     .company_name("Example Inc")
    ///     .last_name("Doe")
    ///     .call()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_email_v2(&self) -> FindEmailV2Builder<'_> {
        FindEmailV2Builder {
            client: self,
            first_name: None,
            domain: None,
            company_name: None,
            middle_name: None,
            last_name: None,
        }
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
    /// Returns a builder that allows you to set parameters using method chaining.
    /// 
    /// # Requirements
    /// Exactly one of `domain` or `company_name` must be provided (XOR requirement).
    /// 
    /// # Example
    /// ```no_run
    /// use zero_bounce::ZeroBounce;
    /// use zero_bounce::utility::ZBResult;
    /// 
    /// # fn main() -> ZBResult<()> {
    /// let zb = ZeroBounce::new("your_api_key");
    /// // Using domain
    /// let result = zb.domain_search_v2()
    ///     .domain("example.com")
    ///     .call()?;
    /// // Or using company name
    /// let result = zb.domain_search_v2()
    ///     .company_name("Example Inc")
    ///     .call()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn domain_search_v2(&self) -> DomainSearchV2Builder<'_> {
        DomainSearchV2Builder {
            client: self,
            domain: None,
            company_name: None,
        }
    }

}

impl<'a> FindEmailV2Builder<'a> {
    /// Set the first name (mandatory).
    pub fn first_name(mut self, name: &'a str) -> Self {
        self.first_name = Some(name);
        self
    }

    /// Set the domain name (exactly one of domain or company_name must be provided).
    pub fn domain(mut self, domain: &'a str) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Set the company name (exactly one of domain or company_name must be provided).
    pub fn company_name(mut self, company: &'a str) -> Self {
        self.company_name = Some(company);
        self
    }

    /// Set the middle name (optional).
    pub fn middle_name(mut self, name: &'a str) -> Self {
        self.middle_name = Some(name);
        self
    }

    /// Set the last name (optional).
    pub fn last_name(mut self, name: &'a str) -> Self {
        self.last_name = Some(name);
        self
    }

    /// Execute the API call and return the result.
    pub fn call(self) -> ZBResult<FindEmailResponseV2> {
        let first_name = self.first_name.ok_or_else(|| ZBError::explicit("first_name is mandatory and must be set"))?;
        
        if first_name.is_empty() {
            return Err(ZBError::explicit("first_name cannot be empty"));
        }

        // Validate XOR requirement: exactly one of domain or company_name must be provided
        match (self.domain, self.company_name) {
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
            ("first_name", first_name),
        ]);

        if let Some(d) = self.domain {
            query_args.insert("domain", d);
        }

        if let Some(c) = self.company_name {
            query_args.insert("company_name", c);
        }

        if let Some(middle) = self.middle_name {
            if !middle.is_empty() {
                query_args.insert("middle_name", middle);
            }
        }

        if let Some(last) = self.last_name {
            if !last.is_empty() {
                query_args.insert("last_name", last);
            }
        }

        let response_content = self.client.generic_get_request(
            self.client.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args
        )?;

        // Debug: Print raw response to examine structure in debug mode
        #[cfg(debug_assertions)]
        {
            eprintln!("Raw API response: {}", response_content);
        }

        let find_email_response = from_str::<FindEmailResponseV2>(&response_content)?;
        Ok(find_email_response)
    }
}

impl<'a> DomainSearchV2Builder<'a> {
    /// Set the domain name (exactly one of domain or company_name must be provided).
    pub fn domain(mut self, domain: &'a str) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Set the company name (exactly one of domain or company_name must be provided).
    pub fn company_name(mut self, company: &'a str) -> Self {
        self.company_name = Some(company);
        self
    }

    /// Execute the API call and return the result.
    pub fn call(self) -> ZBResult<DomainSearchResponseV2> {
        // Validate XOR requirement: exactly one of domain or company_name must be provided
        match (self.domain, self.company_name) {
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

        let mut query_args = HashMap::new();

        if let Some(d) = self.domain {
            query_args.insert("domain", d);
        }

        if let Some(c) = self.company_name {
            query_args.insert("company_name", c);
        }

        let response_content = self.client.generic_get_request(
            self.client.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args
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
