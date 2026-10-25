//! Client for the ZeroBounce Warmup API (<https://www.zerobounce.net/docs/warmup-api>).
//!
//! The Warmup API is separate from the email validation API: its own host, its
//! own team-scoped key, and an `API-Key` header instead of an `api_key`
//! parameter. Use [`ZeroBounceWarmup`] with a Warmup API key; [`crate::ZeroBounce`]
//! stays for the validation API key.

use std::collections::HashMap;
use std::time::Duration;

use chrono::NaiveDate;
use serde_json::{Map, Value};

use crate::utility::http_error::message_from_body;
use crate::utility::{ZBError, ZBResult};

const DEFAULT_BASE_URL: &str = "https://warmup-api.zerobounce.net";
const BASE_PATH: &str = "/api/v1/warmup";

/// Required by the schema but always replaced with the key's own team.
const ANY_TEAM_ID: i64 = 1;

/// How a sending account is run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingModel {
    /// You run the mailbox yourself.
    SelfManaged,
    /// ZeroBounce provisions and runs the mailbox.
    ZbManaged,
}

impl OnboardingModel {
    fn as_str(self) -> &'static str {
        match self {
            OnboardingModel::SelfManaged => "self_managed",
            OnboardingModel::ZbManaged => "zb_managed",
        }
    }
}

/// What ZeroBounce should send on your behalf, for `zb_managed` onboarding.
#[derive(Debug, Clone, Default)]
pub struct EmailContent {
    /// Any of `transactional`, `newsletter`, `marketing`.
    pub types_of_emails: Vec<String>,
    pub industry: Option<String>,
    pub company_name: Option<String>,
    pub website: Option<String>,
}

/// When ZeroBounce should send, for `zb_managed` onboarding.
#[derive(Debug, Clone, Default)]
pub struct ManagedSending {
    /// -11 to 14.
    pub timezone_offset: Option<i32>,
    pub warmup_start_date: Option<NaiveDate>,
    pub daily_limit: Option<u32>,
}

const EMAIL_TYPES: [&str; 3] = ["transactional", "newsletter", "marketing"];

#[derive(Debug)]
pub struct ZeroBounceWarmup {
    api_key: String,
    root: String,
    client: reqwest::blocking::Client,
}

impl ZeroBounceWarmup {
    /// A client for the default Warmup host.
    pub fn new(api_key: &str) -> ZBResult<Self> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    /// A client for a custom https base URL.
    pub fn with_base_url(api_key: &str, base_url: &str) -> ZBResult<Self> {
        if api_key.trim().is_empty() {
            return Err(ZBError::explicit("api_key is required"));
        }
        // A key with surrounding whitespace or control characters would be
        // quoted, unredacted, in the HTTP library's own error.
        if api_key != api_key.trim() || api_key.chars().any(|c| c.is_control()) {
            return Err(ZBError::explicit(
                "api_key has surrounding whitespace or control characters",
            ));
        }
        // https only, with the same loopback exception the validation client
        // makes so the offline suite can point at a local fake.
        crate::parse_request_url(base_url)
            .map_err(|_| ZBError::explicit("base_url must be an https:// URL"))?;
        Ok(ZeroBounceWarmup {
            api_key: api_key.to_string(),
            root: format!("{}{BASE_PATH}", base_url.trim_end_matches('/')),
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::default()),
        })
    }

    // -- transport -----------------------------------------------------------

    /// Replace every form of a secret in a message with `***`.
    fn redact(&self, text: &str, secrets: &[&str]) -> String {
        let mut value = text.to_string();
        for secret in std::iter::once(self.api_key.as_str()).chain(secrets.iter().copied()) {
            if secret.is_empty() {
                continue;
            }
            value = value.replace(secret, "***");
        }
        value
    }

    fn send(
        &self,
        method: &str,
        path: &str,
        query: &HashMap<&str, String>,
        body: Option<&Value>,
        secrets: &[&str],
    ) -> ZBResult<(u16, String, String)> {
        let url = crate::parse_request_url(&format!("{}{path}", self.root))?;
        let mut request = match method {
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            _ => self.client.get(url),
        }
        .header("API-Key", self.api_key.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .query(query);

        if let Some(body) = body {
            // reqwest's json() helper needs its "json" feature, which this
            // crate does not enable, so the body and content type are set here.
            let encoded = serde_json::to_string(body)
                .map_err(|_| ZBError::explicit("Invalid parameter: request body"))?;
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(encoded);
        }

        let response = request
            .send()
            .map_err(|error| ZBError::ExplicitError(self.redact(&error.to_string(), secrets)))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let max_bytes = if content_type.starts_with("text/csv") {
            crate::utility::MAX_GETFILE_BYTES
        } else {
            crate::utility::MAX_JSON_RESPONSE_BYTES
        };
        let body = crate::utility::read_limited(
            response,
            max_bytes,
            "response body exceeds size limit",
        )
        .map_err(|error| ZBError::ExplicitError(self.redact(&error.to_string(), secrets)))?;
        let text = String::from_utf8_lossy(&body).into_owned();

        if (300..400).contains(&status) {
            return Err(ZBError::api(
                status,
                "ZeroBounce returned a redirect, which this client does not follow.",
            ));
        }
        if status > 299 {
            return Err(ZBError::api(
                status,
                self.redact(&message_from_body(status, &content_type, &text), secrets),
            ));
        }
        Ok((status, content_type, text))
    }

    /// The JSON payload of a success, or `None` for the documented empty
    /// successes (204, and an empty 200).
    fn json(
        &self,
        method: &str,
        path: &str,
        query: &HashMap<&str, String>,
        body: Option<&Value>,
        secrets: &[&str],
    ) -> ZBResult<Option<Value>> {
        let (status, content_type, text) = self.send(method, path, query, body, secrets)?;
        if status == 204 || text.trim().is_empty() {
            return Ok(None);
        }
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Err(ZBError::api(
                status,
                self.redact(&message_from_body(status, &content_type, &text), secrets),
            )),
        }
    }

    fn get(&self, path: &str, query: &HashMap<&str, String>) -> ZBResult<Option<Value>> {
        self.json("GET", path, query, None, &[])
    }

    fn write(&self, method: &str, path: &str, body: Value) -> ZBResult<Option<Value>> {
        self.json(method, path, &HashMap::new(), Some(&body), &[])
    }

    /// A CSV download, returned as text.
    fn csv(&self, path: &str, query: &HashMap<&str, String>) -> ZBResult<String> {
        let (status, content_type, text) = self.send("GET", path, query, None, &[])?;
        if text.trim().is_empty() || content_type.to_ascii_lowercase().contains("application/json")
        {
            return Err(ZBError::api(
                status,
                message_from_body(status, &content_type, &text),
            ));
        }
        Ok(text)
    }

    // -- argument checks -----------------------------------------------------

    fn required<'a>(value: &'a str, name: &str) -> ZBResult<&'a str> {
        if value.trim().is_empty() {
            return Err(ZBError::ExplicitError(format!("{name} is required")));
        }
        Ok(value)
    }

    fn in_range(value: i64, low: i64, high: i64, name: &str) -> ZBResult<i64> {
        if !(low..=high).contains(&value) {
            return Err(ZBError::ExplicitError(format!(
                "{name} must be {low} to {high}"
            )));
        }
        Ok(value)
    }

    fn engagement_body(
        c_from: &str,
        reply_pct: i64,
        important_pct: i64,
        click_pct: i64,
    ) -> ZBResult<Value> {
        Self::required(c_from, "c_from")?;
        Self::in_range(reply_pct, 5, 45, "reply_pct")?;
        Self::in_range(important_pct, 5, 40, "important_pct")?;
        Self::in_range(click_pct, 0, 70, "click_pct")?;
        if (click_pct as f64) > important_pct as f64 * 1.75 {
            return Err(ZBError::explicit(
                "click_pct must be at most important_pct x 1.75",
            ));
        }
        Ok(Value::Object(Map::from_iter([
            ("c_from".into(), Value::String(c_from.to_string())),
            ("reply_pct".into(), Value::from(reply_pct)),
            ("important_pct".into(), Value::from(important_pct)),
            ("click_pct".into(), Value::from(click_pct)),
        ])))
    }

    // -- sending accounts ----------------------------------------------------

    /// Every sending account of the key's team. `GET /sending-accounts`
    pub fn list_sending_accounts(&self) -> ZBResult<Option<Value>> {
        self.get("/sending-accounts", &HashMap::new())
    }

    /// One sending account. `GET /sending-account`
    pub fn get_sending_account(&self, from_address: &str) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        self.get(
            "/sending-account",
            &HashMap::from([("from_address", from_address.to_string())]),
        )
    }

    /// Add a from-address to warmup. `POST /sending-accounts`
    ///
    /// `SelfManaged` runs your own mailbox and needs `engagement_rule_id` 0-4.
    /// `ZbManaged` has ZeroBounce provision it and takes `email_content` and
    /// `managed_sending`; a 200 means accepted, and the mailbox is provisioned
    /// afterwards.
    pub fn add_sending_account(
        &self,
        from_address: &str,
        onboarding_model: OnboardingModel,
        engagement_rule_id: Option<i64>,
        ignore_conflict: Option<bool>,
        email_content: Option<&EmailContent>,
        managed_sending: Option<&ManagedSending>,
    ) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        let mut body = Map::from_iter([
            ("from_address".into(), Value::String(from_address.into())),
            (
                "onboarding_model".into(),
                Value::String(onboarding_model.as_str().into()),
            ),
        ]);

        if onboarding_model == OnboardingModel::SelfManaged {
            let Some(rule) = engagement_rule_id else {
                return Err(ZBError::explicit(
                    "engagement_rule_id is required for self_managed",
                ));
            };
            body.insert(
                "engagement_rule_id".into(),
                Value::from(Self::in_range(rule, 0, 4, "engagement_rule_id")?),
            );
            if email_content.is_some() || managed_sending.is_some() {
                return Err(ZBError::explicit(
                    "email_content and managed_sending are for zb_managed",
                ));
            }
        }
        if let Some(flag) = ignore_conflict {
            body.insert("ignore_conflict".into(), Value::Bool(flag));
        }
        if let Some(content) = email_content {
            let mut object = Map::new();
            if !content.types_of_emails.is_empty() {
                for kind in &content.types_of_emails {
                    if !EMAIL_TYPES.contains(&kind.as_str()) {
                        return Err(ZBError::explicit(
                            "types_of_emails must be transactional, newsletter and/or marketing",
                        ));
                    }
                }
                object.insert(
                    "types_of_emails".into(),
                    Value::Array(
                        content
                            .types_of_emails
                            .iter()
                            .map(|kind| Value::String(kind.clone()))
                            .collect(),
                    ),
                );
            }
            for (key, value) in [
                ("industry", &content.industry),
                ("company_name", &content.company_name),
                ("website", &content.website),
            ] {
                if let Some(value) = value {
                    object.insert(key.into(), Value::String(value.clone()));
                }
            }
            body.insert("email_content".into(), Value::Object(object));
        }
        if let Some(sending) = managed_sending {
            let mut object = Map::new();
            if let Some(offset) = sending.timezone_offset {
                Self::in_range(offset as i64, -11, 14, "timezone_offset")?;
                object.insert("timezone_offset".into(), Value::from(offset));
            }
            if let Some(date) = sending.warmup_start_date {
                object.insert(
                    "warmup_start_date".into(),
                    Value::String(date.format("%F").to_string()),
                );
            }
            if let Some(limit) = sending.daily_limit {
                object.insert("daily_limit".into(), Value::from(limit));
            }
            body.insert("managed_sending".into(), Value::Object(object));
        }
        self.write("POST", "/sending-accounts", Value::Object(body))
    }

    /// Change an account's engagement rule, status and auto-enable flag.
    /// `PUT /sending-accounts`. `status_id` is 0 to 8 (4 Paused, 5 Disabled).
    pub fn update_sending_account(
        &self,
        from_address: &str,
        engagement_rule_id: i64,
        status_id: i64,
        auto_enable: bool,
    ) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        let body = Value::Object(Map::from_iter([
            ("team_id".into(), Value::from(ANY_TEAM_ID)),
            ("from_address".into(), Value::String(from_address.into())),
            (
                "engagement_rule_id".into(),
                Value::from(Self::in_range(engagement_rule_id, 0, 4, "engagement_rule_id")?),
            ),
            (
                "status_id".into(),
                Value::from(Self::in_range(status_id, 0, 8, "status_id")?),
            ),
            ("auto_enable".into(), Value::Bool(auto_enable)),
        ]));
        self.write("PUT", "/sending-accounts", body)
    }

    /// Remove a sending account. `DELETE /sending-accounts`. Destructive and final.
    pub fn delete_sending_account(&self, from_address: &str) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        self.json(
            "DELETE",
            "/sending-accounts",
            &HashMap::from([("from_address", from_address.to_string())]),
            None,
            &[],
        )
    }

    /// Move an account and its history to a new from-address.
    /// `PUT /smtp-credentials` — despite the path, no SMTP credentials are stored.
    pub fn change_from_address(
        &self,
        old_from_address: &str,
        new_from_address: &str,
    ) -> ZBResult<Option<Value>> {
        Self::required(old_from_address, "old_from_address")?;
        Self::required(new_from_address, "new_from_address")?;
        let body = Value::Object(Map::from_iter([
            ("team_id".into(), Value::from(ANY_TEAM_ID)),
            (
                "old_from_address".into(),
                Value::String(old_from_address.into()),
            ),
            (
                "new_from_address".into(),
                Value::String(new_from_address.into()),
            ),
        ]));
        self.write("PUT", "/smtp-credentials", body)
    }

    /// Registered from-addresses. `GET /from-addresses`
    ///
    /// `search_from` takes letters, digits and `. - _ @ +` only. `offset` and
    /// `limit` go on the wire as strings.
    pub fn list_from_addresses(
        &self,
        search_from: Option<&str>,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> ZBResult<Option<Value>> {
        let mut query = HashMap::new();
        if let Some(search) = search_from {
            if !search
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-_@+".contains(c))
            {
                return Err(ZBError::explicit(
                    "search_from takes letters, digits and . - _ @ + only",
                ));
            }
            query.insert("search_from", search.to_string());
        }
        if let Some(offset) = offset {
            query.insert("offset", offset.to_string());
        }
        if let Some(limit) = limit {
            query.insert("limit", limit.to_string());
        }
        self.get("/from-addresses", &query)
    }

    // -- engagement rules ----------------------------------------------------

    /// -1 Custom, 0 Default, 1 Slow Pace, 2 Repair, 3 Boost, 4 Protect.
    pub fn get_engagement_rules(&self) -> ZBResult<Option<Value>> {
        self.get("/engagement-rules", &HashMap::new())
    }

    /// Set a standard rule (0 to 4) on an address. `PUT /engagement-rule`
    pub fn set_engagement_rule(&self, c_from: &str, rule_id: i64) -> ZBResult<Option<Value>> {
        Self::required(c_from, "c_from")?;
        let body = Value::Object(Map::from_iter([(
            "rule_id".to_string(),
            Value::from(Self::in_range(rule_id, 0, 4, "rule_id")?),
        )]));
        self.json(
            "PUT",
            "/engagement-rule",
            &HashMap::from([("c_from", c_from.to_string())]),
            Some(&body),
            &[],
        )
    }

    /// An address's custom rule, with `can_update` and `remaining_seconds`.
    pub fn get_custom_engagement(&self, c_from: &str) -> ZBResult<Option<Value>> {
        Self::required(c_from, "c_from")?;
        let path = format!(
            "/custom-engagement/{}",
            crate::utility::encode_path_segment(c_from.trim())
        );
        if c_from.trim() == "." || c_from.trim() == ".." {
            return Err(ZBError::explicit("c_from is invalid"));
        }
        self.get(&path, &HashMap::new())
    }

    /// Create a custom rule. Starts a 7 day cooldown on updates.
    pub fn create_custom_engagement(
        &self,
        c_from: &str,
        reply_pct: i64,
        important_pct: i64,
        click_pct: i64,
    ) -> ZBResult<Option<Value>> {
        let body = Self::engagement_body(c_from, reply_pct, important_pct, click_pct)?;
        self.write("POST", "/custom-engagement", body)
    }

    /// Update a custom rule. During the 7 day cooldown the API answers 429.
    pub fn update_custom_engagement(
        &self,
        c_from: &str,
        reply_pct: i64,
        important_pct: i64,
        click_pct: i64,
    ) -> ZBResult<Option<Value>> {
        let body = Self::engagement_body(c_from, reply_pct, important_pct, click_pct)?;
        self.write("PUT", "/custom-engagement", body)
    }

    /// Delete a custom rule and fall back to standard rule 1-4.
    pub fn delete_custom_engagement(
        &self,
        c_from: &str,
        engagement_rule_id: i64,
    ) -> ZBResult<Option<Value>> {
        Self::required(c_from, "c_from")?;
        let body = Value::Object(Map::from_iter([
            ("c_from".to_string(), Value::String(c_from.into())),
            (
                "engagement_rule_id".to_string(),
                Value::from(Self::in_range(engagement_rule_id, 1, 4, "engagement_rule_id")?),
            ),
        ]));
        self.write("DELETE", "/custom-engagement", body)
    }

    // -- statistics, seeds and utilities -------------------------------------

    /// The seed list. `GET /seeds`
    pub fn get_seeds(&self) -> ZBResult<Option<Value>> {
        self.get("/seeds", &HashMap::new())
    }

    /// The seed list as CSV (columns `seed,provider`). `GET /seeds/download`
    pub fn download_seeds(&self) -> ZBResult<String> {
        self.csv("/seeds/download", &HashMap::new())
    }

    /// Number of seeds. `GET /seed-count`
    pub fn get_seed_count(&self) -> ZBResult<Option<Value>> {
        self.get("/seed-count", &HashMap::new())
    }

    /// Today's warmup progress. `GET /progress-today`
    pub fn get_progress_today(&self) -> ZBResult<Option<Value>> {
        self.get("/progress-today", &HashMap::new())
    }

    /// Messages sent today. `GET /sent-today`
    pub fn get_sent_today(&self) -> ZBResult<Option<Value>> {
        self.get("/sent-today", &HashMap::new())
    }

    /// Current daily volume. `GET /emails-per-day`
    pub fn get_emails_per_day(&self) -> ZBResult<Option<Value>> {
        self.get("/emails-per-day", &HashMap::new())
    }

    /// Statistics for an address. An address with no history returns zeros.
    pub fn get_statistics(&self, from_address: &str) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        self.get(
            "/statistics",
            &HashMap::from([("from_address", from_address.to_string())]),
        )
    }

    /// Daily statistics between two dates, inclusive. `GET /statistics-per-day`
    pub fn get_statistics_per_day(
        &self,
        from_address: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        if start_date > end_date {
            return Err(ZBError::explicit("start_date is later than end_date"));
        }
        self.get(
            "/statistics-per-day",
            &HashMap::from([
                ("from_address", from_address.to_string()),
                ("start_date", start_date.format("%F").to_string()),
                ("end_date", end_date.format("%F").to_string()),
            ]),
        )
    }

    /// Detailed statistics for an address. `GET /statistics-detailed`
    pub fn get_statistics_detailed(&self, c_from: &str) -> ZBResult<Option<Value>> {
        Self::required(c_from, "c_from")?;
        self.get(
            "/statistics-detailed",
            &HashMap::from([("c_from", c_from.to_string())]),
        )
    }

    /// An address's statistics as CSV. `GET /statistics/download`
    pub fn download_statistics(&self, c_from: &str) -> ZBResult<String> {
        Self::required(c_from, "c_from")?;
        self.csv(
            "/statistics/download",
            &HashMap::from([("c_from", c_from.to_string())]),
        )
    }

    /// Statistics for one from-address. `GET /from-address-statistics`
    pub fn get_from_address_statistics(&self, from_address: &str) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        self.get(
            "/from-address-statistics",
            &HashMap::from([("from_address", from_address.to_string())]),
        )
    }

    /// SPF, DKIM and DMARC status of a from-address's domain.
    /// `GET /check-domain-dmarc`
    ///
    /// LIVE: pass an **email address**. The live API answers "Please provide a
    /// valid email address" for a bare domain, although the docs describe one.
    pub fn check_domain_dmarc(
        &self,
        from_address: &str,
        dkim_selector: Option<&str>,
    ) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        if !from_address.contains('@') {
            return Err(ZBError::explicit(
                "from_address must be an email address, e.g. sender@example.com",
            ));
        }
        let mut query = HashMap::from([("from_address", from_address.to_string())]);
        if let Some(selector) = dkim_selector {
            Self::required(selector, "dkim_selector")?;
            query.insert("dkim_selector", selector.to_string());
        }
        self.get("/check-domain-dmarc", &query)
    }

    /// Check that SMTP credentials connect and hand off a message.
    /// `POST /test-smtp-configuration`
    ///
    /// Nothing is stored, and the test message goes to a ZeroBounce test
    /// recipient, not `from_address`.
    ///
    /// LIVE: the API answers any failed SMTP login with HTTP 500. That is an
    /// API bug. The username and password are masked in every error from here.
    pub fn test_smtp_configuration(
        &self,
        from_address: &str,
        username: &str,
        password: &str,
        server: &str,
        port: u16,
    ) -> ZBResult<Option<Value>> {
        Self::required(from_address, "from_address")?;
        Self::required(username, "username")?;
        Self::required(password, "password")?;
        Self::required(server, "server")?;
        if port == 0 {
            return Err(ZBError::explicit("port must be 1 to 65535"));
        }
        let body = Value::Object(Map::from_iter([
            ("from_address".to_string(), Value::String(from_address.into())),
            ("username".to_string(), Value::String(username.into())),
            ("password".to_string(), Value::String(password.into())),
            ("server".to_string(), Value::String(server.into())),
            ("port".to_string(), Value::from(port)),
        ]));
        self.json(
            "POST",
            "/test-smtp-configuration",
            &HashMap::new(),
            Some(&body),
            &[password, username],
        )
    }
}
