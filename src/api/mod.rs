pub mod bulk;
pub mod inbox_placement;
pub mod priority3;
pub mod validation;

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};

pub use crate::ZeroBounce;
use crate::utility::http_error::{parse_json_body_with_status, UNEXPECTED_JSON_SHAPE_MESSAGE};
use crate::utility::structures::generic::{
    FindEmailResponse, FindEmailResponseV2, DomainSearchResponseV2, DOMAIN_SEARCH_V2_FIELD_NAMES,
};
use crate::utility::{ZBError, ZBResult, ENDPOINT_EMAIL_FINDER};
use crate::utility::structures::{ActivityData, ApiUsage};
use crate::utility::structures::generic::ScoreEmailResponse;
use crate::utility::{ENDPOINT_ACTIVITY_DATA, ENDPOINT_API_USAGE, ENDPOINT_CREDITS, ENDPOINT_SCORING};

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

    /// The status and body of a 2xx answer to a GET, checked like
    /// `generic_get_request`. When `message_only_is_failure` is set, a
    /// message-only or empty object is an error. The response status is kept
    /// so that a body of the wrong shape is reported with it.
    pub(crate) fn get_request_with_status<'a>(
        &'a self,
        url: String,
        mut query_args: HashMap<&'a str, &'a str>,
        message_only_is_failure: bool,
    ) -> ZBResult<(u16, String)> {
        self.require_key()?;
        query_args.insert("api_key", self.api_key.as_str());
        let response = self
            .client
            .get(crate::parse_request_url(&url)?)
            .query(&query_args)
            .send()?;
        self.read_api_response_with_status(response, message_only_is_failure)
    }

    /// `get_request_with_status` for a form POST.
    pub(crate) fn form_post_request_with_status<'a>(
        &'a self,
        url: String,
        mut form_args: HashMap<&'a str, &'a str>,
    ) -> ZBResult<(u16, String)> {
        self.require_key()?;
        form_args.insert("api_key", self.api_key.as_str());
        let response = self
            .client
            .post(crate::parse_request_url(&url)?)
            .form(&form_args)
            .send()?;
        self.read_api_response_with_status(response, false)
    }

    /// `read_api_response` that also returns the status.
    pub(crate) fn read_api_response_with_status(
        &self,
        response: reqwest::blocking::Response,
        message_only_is_failure: bool,
    ) -> ZBResult<(u16, String)> {
        let (status, content_type, content) = Self::read_api_body(response)?;
        crate::utility::http_error::ensure_json_api_body(status, &content_type, &content)
            .map_err(|error| self.redact_error(error))?;
        if message_only_is_failure {
            crate::utility::http_error::reject_message_only_body(status, &content_type, &content)
                .map_err(|error| self.redact_error(error))?;
        }
        Ok((status, content))
    }

    /// The `Credits` value of a 2xx body: an integer sent as a JSON string
    /// (`"-1"`, `"12345"`, as the live API sends it) or as a JSON integer
    /// (`{"Credits":2375323}`, zerobounce.net's own example). Anything else
    /// (missing, `null`, `5.5`, `5.0`, `"abc"`, another JSON type, out of
    /// `i64` range) is JSON of the wrong shape: a [`ZBError::ApiError`] with
    /// the response status, like on every other call.
    fn get_credits_from_body(status: u16, string_value: &str) -> ZBResult<i64> {
        let body = parse_json_body_with_status::<serde_json::Value>(status, string_value)?;
        match body.get("Credits") {
            Some(serde_json::Value::String(credits)) => credits.parse::<i64>().ok(),
            Some(serde_json::Value::Number(credits)) => credits.as_i64(),
            _ => None,
        }
        .ok_or_else(|| ZBError::api(status, UNEXPECTED_JSON_SHAPE_MESSAGE))
    }

    #[cfg(test)]
    fn get_credits_from_string(string_value: String) -> ZBResult<i64> {
        Self::get_credits_from_body(200, &string_value)
    }

    pub fn get_credits(&self) -> ZBResult<i64> {
        let query_args = HashMap::new();

        let (status, response_content) = self.get_request_with_status(
            self.url_provider.url_of(ENDPOINT_CREDITS), query_args, false
        )?;

        Self::get_credits_from_body(status, &response_content)
    }

    pub fn get_api_usage(&self, start_date: NaiveDate, end_date: NaiveDate) -> ZBResult<ApiUsage> {
        let start_date_str = start_date.format("%F").to_string();
        let end_date_str = end_date.format("%F").to_string();
        let query_args = HashMap::from([
            ("start_date", start_date_str.as_str()),
            ("end_date", end_date_str.as_str()),
        ]);

        let (status, response_content) = self.get_request_with_status(
            self.url_provider.url_of(ENDPOINT_API_USAGE), query_args, false
        )?;

        let api_usage = parse_json_body_with_status::<ApiUsage>(status, &response_content)?;
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

        let (status, response_content) = self.get_request_with_status(
            self.url_provider.url_of(ENDPOINT_ACTIVITY_DATA), query_args, false
        )?;

        let activity_data = crate::utility::http_error::parse_unless_message_beside_nulls::<ActivityData>(
            status, &response_content, &["found", "active_in_days"],
        )?;
        Ok(activity_data)
    }

    pub fn score_email(&self, email: &str) -> ZBResult<ScoreEmailResponse> {
        let query_args = HashMap::from([("email", email)]);

        let (status, response_content) = self.get_request_with_status(
            self.url_provider.url_of(ENDPOINT_SCORING),
            query_args,
            false,
        )?;

        let score = crate::utility::http_error::parse_unless_message_beside_nulls::<ScoreEmailResponse>(
            status, &response_content, &["email", "score"],
        )?;
        Ok(score)
    }

    pub fn score_email_post(&self, email: &str) -> ZBResult<ScoreEmailResponse> {
        let form_args = HashMap::from([("email", email)]);

        let (status, response_content) = self.form_post_request_with_status(
            self.url_provider.url_of(ENDPOINT_SCORING),
            form_args,
        )?;

        let score = crate::utility::http_error::parse_unless_message_beside_nulls::<ScoreEmailResponse>(
            status, &response_content, &["email", "score"],
        )?;
        Ok(score)
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

        let (status, response_content) = self.get_request_with_status(
            self.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args, true
        )?;

        let activity_data = parse_json_body_with_status::<FindEmailResponse>(status, &response_content)?;
        Ok(activity_data)
    }

    /// Find an email address using either a domain or company name.
    /// 
    /// Returns a builder that allows you to set parameters using method chaining.
    /// 
    /// # Requirements
    /// - Exactly one of `domain` or `company_name` must be provided (XOR requirement)
    /// - `first_name` is optional; without it the API answers with the domain's
    ///   address formats, the same as `domain_search_v2`
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
    #[allow(deprecated)] // This method uses deprecated find_email, which is expected
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
    /// Set the first name (optional).
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
        // first_name is optional: /guessformat answers with the domain's formats
        // when no name is given. Setting it to an empty string is still a mistake.
        if let Some("") = self.first_name {
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

        let mut query_args = HashMap::new();

        if let Some(first) = self.first_name {
            query_args.insert("first_name", first);
        }

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

        let (status, response_content) = self.client.get_request_with_status(
            self.client.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args, true
        )?;

        // A message beside fields that are all missing or null (`{"detail":"x","email":null}`)
        // is not a found address. `{"email":null}` with no message stays a result.
        let body = parse_json_body_with_status::<serde_json::Value>(status, &response_content)?;
        if let Some(object) = body.as_object() {
            const NAMES: [&str; 6] = [
                "email",
                "domain",
                "email_confidence",
                "company_name",
                "did_you_mean",
                "failure_reason",
            ];
            if crate::utility::http_error::object_has_message_key(object)
                && !crate::utility::http_error::object_has_non_null_named_field(object, &NAMES)
            {
                return Err(ZBError::api(status, UNEXPECTED_JSON_SHAPE_MESSAGE));
            }
        }
        let find_email_response =
            parse_json_body_with_status::<FindEmailResponseV2>(status, &response_content)?;
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

        let (status, response_content) = self.client.get_request_with_status(
            self.client.url_provider.url_of(ENDPOINT_EMAIL_FINDER), query_args, true
        )?;

        // Every field of the result defaults, because the live API leaves out
        // the ones that do not apply, so serde takes any object, `{}` and
        // `{"message":"m","file_id":null}` included. An object that carries none
        // of the struct's field names is not a domain search answer.
        let body = parse_json_body_with_status::<serde_json::Value>(status, &response_content)?;
        let object = body.as_object();
        let named = object.is_some_and(|object| {
            DOMAIN_SEARCH_V2_FIELD_NAMES
                .iter()
                .any(|name| object.contains_key(*name))
        });
        let real = object.is_some_and(|object| {
            crate::utility::http_error::object_has_non_null_named_field(
                object,
                &DOMAIN_SEARCH_V2_FIELD_NAMES,
            )
        });
        let message = object.is_some_and(crate::utility::http_error::object_has_message_key);
        // `{"domain":null}` is a result. A message beside only null fields is not.
        if !named || (message && !real) {
            return Err(ZBError::api(status, UNEXPECTED_JSON_SHAPE_MESSAGE));
        }
        let domain_search_response =
            parse_json_body_with_status::<DomainSearchResponseV2>(status, &response_content)?;
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
