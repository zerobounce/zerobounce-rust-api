//! Inbox Placement Tester (bulkapi.zerobounce.net/api/ipt).
//!
//! The API key goes on the query string rather than in the body, and bodies are
//! JSON with camelCase fields. Only fields the caller set are sent. Uses the
//! account's inbox-test credits.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::utility::{encode_path_segment, ZBError, ZBResult};
use crate::ZeroBounce;

const TEST_TYPES: [u8; 3] = [1, 2, 3];
const TEST_STATUSES: [&str; 2] = ["running", "finished"];

/// A per-provider seed cap for [`ZeroBounce::create_inbox_placement_test`].
#[derive(Clone, Debug)]
pub struct InboxPlacementProviderSelection {
    pub id: i64,
    /// At least 1 when given.
    pub max_seeds: Option<i64>,
}

/// Options for [`ZeroBounce::create_inbox_placement_test`]. Unset fields are not sent.
#[derive(Clone, Debug, Default)]
pub struct CreateInboxPlacementTestOptions {
    /// 1 (Email, needs `sender_email`), 2 (Unique ID) or 3 (Auto Alias).
    pub test_type: Option<u8>,
    /// The From address to match; required when `test_type` is 1.
    pub sender_email: Option<String>,
    /// Use every active seed of these providers.
    pub provider_ids: Option<Vec<i64>>,
    /// Use these seed mailboxes.
    pub seed_ids: Option<Vec<i64>>,
    /// Cap the seeds taken from each provider.
    pub providers: Option<Vec<InboxPlacementProviderSelection>>,
    /// Where the finished result is posted.
    pub webhook_url: Option<String>,
}

/// Filters for [`ZeroBounce::list_inbox_placement_tests`]. Unset fields are not sent.
#[derive(Clone, Debug, Default)]
pub struct ListInboxPlacementTestsOptions {
    /// "running" or "finished".
    pub status: Option<String>,
    /// ISO 8601 date or datetime.
    pub created_after: Option<String>,
    /// ISO 8601 date or datetime.
    pub created_before: Option<String>,
    /// From 1.
    pub page: Option<u32>,
    /// 1 to 100.
    pub page_size: Option<u32>,
}

impl ZeroBounce {
    /// Seed providers available for new tests. `GET /providers`
    pub fn get_inbox_placement_providers(&self) -> ZBResult<Value> {
        self.inbox_placement_get("/providers", HashMap::new())
    }

    /// Supported test types and the default one. `GET /test-types`
    pub fn get_inbox_placement_test_types(&self) -> ZBResult<Value> {
        self.inbox_placement_get("/test-types", HashMap::new())
    }

    /// Create an inbox placement test. `POST /test`
    ///
    /// `name` must be non-empty and at most 64 characters. Spends one inbox test.
    pub fn create_inbox_placement_test(
        &self,
        name: &str,
        options: &CreateInboxPlacementTestOptions,
    ) -> ZBResult<Value> {
        if name.trim().is_empty() {
            return Err(ZBError::explicit("Empty parameter: name"));
        }
        if name.chars().count() > 64 {
            return Err(ZBError::explicit(
                "Invalid parameter: name (at most 64 characters)",
            ));
        }
        if let Some(test_type) = options.test_type {
            if !TEST_TYPES.contains(&test_type) {
                return Err(ZBError::explicit(
                    "Invalid parameter: test_type (must be 1, 2 or 3)",
                ));
            }
            if test_type == 1
                && options
                    .sender_email
                    .as_deref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true)
            {
                return Err(ZBError::explicit(
                    "Empty parameter: sender_email (required when test_type is 1)",
                ));
            }
        }
        for provider in options.providers.iter().flatten() {
            if provider.max_seeds.map(|seeds| seeds < 1).unwrap_or(false) {
                return Err(ZBError::explicit(
                    "Invalid parameter: providers (max_seeds must be at least 1)",
                ));
            }
        }

        let mut body = Map::new();
        body.insert("name".to_string(), json!(name));
        if let Some(test_type) = options.test_type {
            body.insert("testType".to_string(), json!(test_type));
        }
        if let Some(sender_email) = options.sender_email.as_deref() {
            body.insert("senderEmail".to_string(), json!(sender_email));
        }
        if let Some(provider_ids) = options.provider_ids.as_ref() {
            body.insert("providerIds".to_string(), json!(provider_ids));
        }
        if let Some(seed_ids) = options.seed_ids.as_ref() {
            body.insert("seedIds".to_string(), json!(seed_ids));
        }
        if let Some(providers) = options.providers.as_ref() {
            let entries: Vec<Value> = providers
                .iter()
                .map(|provider| match provider.max_seeds {
                    Some(max_seeds) => json!({"id": provider.id, "maxSeeds": max_seeds}),
                    None => json!({"id": provider.id}),
                })
                .collect();
            body.insert("providers".to_string(), Value::Array(entries));
        }
        if let Some(webhook_url) = options.webhook_url.as_deref() {
            body.insert("webhookUrl".to_string(), json!(webhook_url));
        }
        self.inbox_placement_json("POST", "/test", Value::Object(body))
    }

    /// A test and its placement results. `GET /test/{id}`
    pub fn get_inbox_placement_test(&self, test_id: &str) -> ZBResult<Value> {
        let path = format!("/test/{}", Self::inbox_placement_test_path(test_id)?);
        self.inbox_placement_get(&path, HashMap::new())
    }

    /// List tests, newest first. `GET /tests`
    pub fn list_inbox_placement_tests(
        &self,
        options: &ListInboxPlacementTestsOptions,
    ) -> ZBResult<Value> {
        if let Some(status) = options.status.as_deref() {
            if !TEST_STATUSES.contains(&status) {
                return Err(ZBError::explicit(
                    "Invalid parameter: status (must be running or finished)",
                ));
            }
        }
        if options.page.map(|page| page < 1).unwrap_or(false) {
            return Err(ZBError::explicit(
                "Invalid parameter: page (must be at least 1)",
            ));
        }
        if options
            .page_size
            .map(|size| !(1..=100).contains(&size))
            .unwrap_or(false)
        {
            return Err(ZBError::explicit(
                "Invalid parameter: page_size (must be between 1 and 100)",
            ));
        }

        let page_param;
        let page_size_param;
        let mut query: HashMap<&str, &str> = HashMap::new();
        if let Some(status) = options.status.as_deref() {
            query.insert("status", status);
        }
        if let Some(created_after) = options.created_after.as_deref() {
            query.insert("createdAfter", created_after);
        }
        if let Some(created_before) = options.created_before.as_deref() {
            query.insert("createdBefore", created_before);
        }
        if let Some(page) = options.page {
            page_param = page.to_string();
            query.insert("page", &page_param);
        }
        if let Some(page_size) = options.page_size {
            page_size_param = page_size.to_string();
            query.insert("page_size", &page_size_param);
        }
        self.inbox_placement_get("/tests", query)
    }

    /// Delete a finished test. `DELETE /test/{id}`
    pub fn delete_inbox_placement_test(&self, test_id: &str) -> ZBResult<Value> {
        let path = format!("/test/{}", Self::inbox_placement_test_path(test_id)?);
        self.inbox_placement_request("DELETE", &path, HashMap::new(), None)
    }

    /// The webhook URL saved on this API key. `GET /webhook`
    pub fn get_inbox_placement_webhook(&self) -> ZBResult<Value> {
        self.inbox_placement_get("/webhook", HashMap::new())
    }

    /// Save the URL finished results are posted to. `PUT /webhook`
    pub fn set_inbox_placement_webhook(&self, webhook_url: &str) -> ZBResult<Value> {
        if webhook_url.trim().is_empty() {
            return Err(ZBError::explicit("Empty parameter: webhook_url"));
        }
        self.inbox_placement_json("PUT", "/webhook", json!({ "webhookUrl": webhook_url }))
    }

    fn inbox_placement_test_path(test_id: &str) -> ZBResult<String> {
        if test_id.trim().is_empty() {
            return Err(ZBError::explicit("Empty parameter: test_id"));
        }
        Ok(encode_path_segment(test_id.trim()))
    }

    fn inbox_placement_get<'a>(
        &'a self,
        path: &str,
        query: HashMap<&'a str, &'a str>,
    ) -> ZBResult<Value> {
        self.inbox_placement_request("GET", path, query, None)
    }

    fn inbox_placement_json(&self, method: &str, path: &str, body: Value) -> ZBResult<Value> {
        self.inbox_placement_request(method, path, HashMap::new(), Some(body))
    }

    fn inbox_placement_request<'a>(
        &'a self,
        method: &str,
        path: &str,
        query: HashMap<&'a str, &'a str>,
        body: Option<Value>,
    ) -> ZBResult<Value> {
        self.require_key()?;

        let url = self.url_provider.inbox_placement_url_of(path);
        let parsed = reqwest::Url::parse(&url)
            .map_err(|_| ZBError::explicit("Invalid parameter: inbox placement base URL"))?;

        let mut request = match method {
            "POST" => self.client.post(parsed),
            "PUT" => self.client.put(parsed),
            "DELETE" => self.client.delete(parsed),
            _ => self.client.get(parsed),
        }
        .query(&query)
        // LIVE: the Inbox Placement Tester authenticates with a Bearer header.
        // Sent as an api_key parameter the request is rejected as unauthorized.
        .bearer_auth(self.api_key.as_str());

        if let Some(body) = body {
            // reqwest's json() helper needs its "json" feature, which this crate
            // does not enable, so the body and content type are set directly.
            let encoded = serde_json::to_string(&body)
                .map_err(|_| ZBError::explicit("Invalid parameter: request body"))?;
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(encoded);
        }

        let response = request.send()?;
        let content = self.read_api_response(response)?;
        crate::utility::http_error::parse_json_body(&content)
    }
}
