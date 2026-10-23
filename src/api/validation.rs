use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map as SerdeMap, Value};

use crate::utility::structures::validate_options::ZBValidateOptions;
use crate::utility::structures::validation::{ZBBatchValidation, ZBValidation};
use crate::utility::{ZBError, CONTENT_TYPE_JSON, ENDPOINT_BATCH_VALIDATE, ENDPOINT_VALIDATE};
use crate::{ZBResult, ZeroBounce};

impl ZeroBounce {
    pub fn validate_email_and_ip(&self, email: &str, ip_address: &str) -> ZBResult<ZBValidation> {
        self.validate_email_with_options(email, ip_address, &ZBValidateOptions::default())
    }

    pub fn validate_email_with_options(
        &self,
        email: &str,
        ip_address: &str,
        options: &ZBValidateOptions,
    ) -> ZBResult<ZBValidation> {
        if let Some(timeout) = options.timeout {
            if !(3..=60).contains(&timeout) {
                return Err(ZBError::explicit(
                    "Invalid parameter: timeout (must be between 3 and 60 seconds)",
                ));
            }
        }
        // callback_url only means anything while greylist processing is on, so it is
        // rejected here rather than letting the API take the request.
        if let Some(callback_url) = options.callback_url.as_deref() {
            if options.greylist_processing != Some(true) {
                return Err(ZBError::explicit(
                    "Invalid parameter: callback_url requires greylist_processing = true",
                ));
            }
            if callback_url.trim().is_empty() {
                return Err(ZBError::explicit("Empty parameter: callback_url"));
            }
        }

        let mut query_args = HashMap::from([("email", email)]);
        let timeout_param;
        let activity_param;
        let verify_param;
        let domain_info_param;
        let greylist_param;

        if !ip_address.is_empty() {
            query_args.insert("ip_address", ip_address);
        }
        if let Some(timeout) = options.timeout {
            timeout_param = timeout.to_string();
            query_args.insert("timeout", &timeout_param);
        }
        if let Some(activity_data) = options.activity_data {
            activity_param = if activity_data { "true" } else { "false" };
            query_args.insert("activity_data", activity_param);
        }
        if let Some(verify_plus) = options.verify_plus {
            verify_param = if verify_plus { "true" } else { "false" };
            query_args.insert("verify_plus", verify_param);
        }
        if let Some(domain_info) = options.domain_info {
            domain_info_param = if domain_info { "true" } else { "false" };
            query_args.insert("domain_info", domain_info_param);
        }
        if let Some(greylist_processing) = options.greylist_processing {
            greylist_param = if greylist_processing { "true" } else { "false" };
            query_args.insert("greylist_processing", greylist_param);
        }
        if let Some(callback_url) = options.callback_url.as_deref() {
            query_args.insert("callback_url", callback_url);
        }

        let response_content = if options.use_post {
            let mut form_args = HashMap::from([("email", email)]);
            let timeout_param;
            let activity_param;
            let verify_param;
            let domain_info_param;
            let greylist_param;
            if !ip_address.is_empty() {
                form_args.insert("ip_address", ip_address);
            }
            if let Some(timeout) = options.timeout {
                timeout_param = timeout.to_string();
                form_args.insert("timeout", &timeout_param);
            }
            if let Some(activity_data) = options.activity_data {
                activity_param = if activity_data { "true" } else { "false" };
                form_args.insert("activity_data", activity_param);
            }
            if let Some(verify_plus) = options.verify_plus {
                verify_param = if verify_plus { "true" } else { "false" };
                form_args.insert("verify_plus", verify_param);
            }
            if let Some(domain_info) = options.domain_info {
                domain_info_param = if domain_info { "true" } else { "false" };
                form_args.insert("domain_info", domain_info_param);
            }
            if let Some(greylist_processing) = options.greylist_processing {
                greylist_param = if greylist_processing { "true" } else { "false" };
                form_args.insert("greylist_processing", greylist_param);
            }
            if let Some(callback_url) = options.callback_url.as_deref() {
                form_args.insert("callback_url", callback_url);
            }
            self.generic_form_post_request(self.url_provider.url_of(ENDPOINT_VALIDATE), form_args)?
        } else {
            self.generic_get_request(self.url_provider.url_of(ENDPOINT_VALIDATE), query_args)?
        };

        crate::utility::http_error::parse_json_body(&response_content)
    }

    pub fn validate_email(&self, email: &str) -> ZBResult<ZBValidation> {
        self.validate_email_and_ip(email, "")
    }

    // Represent a list of tuples (containing email and ip_address) into a
    // serializable `serde_json::Value` that respects the expected structure
    // of the batch validation endpoint.
    //
    // Said structure:
    // ```json
    // {
    //     "api_key": {{apikey}},
    //     "email_batch": [
    //         {"email_address": "valid@example.com", "ip_address": "0.0.0.0"},
    //         {"email_address": "invalid@example.com", "ip_address": "1.1.1.1"}
    //     ]
    // }
    // ```
    // After the value is built, serialize and return the resulted string.
    fn batch_validate_prepare_body(
        &self,
        emails_and_ip_addresses: Vec<(String, String)>,
        options: &ZBValidateOptions,
    ) -> ZBResult<String> {
        let email_batch = emails_and_ip_addresses
            .into_iter()
            .map(|(email, ip_address)| {
                [
                    ("email_address".to_string(), Value::String(email)),
                    ("ip_address".to_string(), Value::String(ip_address)),
                ]
            })
            .map(SerdeMap::<String, Value>::from_iter)
            .map(Value::Object)
            .collect::<Vec<Value>>();

        let mut request_body_map = SerdeMap::from_iter([
            ("api_key".to_string(), Value::String(self.api_key.clone())),
            ("email_batch".to_string(), Value::Array(email_batch)),
        ]);
        if let Some(timeout) = options.timeout {
            request_body_map.insert("timeout".to_string(), Value::Number(timeout.into()));
        }
        if let Some(activity_data) = options.activity_data {
            request_body_map.insert("activity_data".to_string(), Value::Bool(activity_data));
        }
        if let Some(verify_plus) = options.verify_plus {
            request_body_map.insert("verify_plus".to_string(), Value::Bool(verify_plus));
        }

        // let request_body_object = Value::Object(request_body_map);
        let mut serializer = serde_json::Serializer::new(Vec::new());
        Value::Object(request_body_map)
            .serialize(&mut serializer)
            .map_err(ZBError::JsonError)?;

        let final_string = String::from_utf8(serializer.into_inner())
            .map_err(|error| ZBError::ExplicitError(error.to_string()))?;

        Ok(final_string)
    }

    pub fn batch_validate(
        &self,
        emails_and_ip_addresses: Vec<(String, String)>,
    ) -> ZBResult<ZBBatchValidation> {
        self.batch_validate_with_options(emails_and_ip_addresses, &ZBValidateOptions::default())
    }

    pub fn batch_validate_with_options(
        &self,
        emails_and_ip_addresses: Vec<(String, String)>,
        options: &ZBValidateOptions,
    ) -> ZBResult<ZBBatchValidation> {
        if let Some(timeout) = options.timeout {
            if !(10..=120).contains(&timeout) {
                return Err(ZBError::explicit(
                    "Invalid parameter: timeout (must be between 10 and 120 seconds)",
                ));
            }
        }
        self.require_key()?;
        let body_content = self.batch_validate_prepare_body(emails_and_ip_addresses, options)?;
        let url = self.url_provider.url_of(ENDPOINT_BATCH_VALIDATE);

        let response = self
            .client
            .post(crate::parse_request_url(&url)?)
            .body(body_content)
            .header("content-type", CONTENT_TYPE_JSON)
            .send()?;

        let response_content = self.read_api_response(response)?;
        let validation: ZBBatchValidation =
            crate::utility::http_error::parse_json_body(&response_content)?;
        for item in &validation.errors {
            if item.email_address.eq_ignore_ascii_case("all") {
                let message = crate::utility::http_error::strip_error_prefix(&item.error);
                let message = if message.is_empty() {
                    "Invalid API Key or your account ran out of credits".to_string()
                } else {
                    message
                };
                return Err(ZBError::ExplicitError(message));
            }
        }
        Ok(validation)
    }
}

#[cfg(test)]
mod test {
    use crate::ZeroBounce;

    #[test]
    fn test_serializing_example() {
        let emails_and_ip_addresses = vec![
            (
                "valid@example.com".to_string(),
                "123.123.123.123".to_string(),
            ),
            ("invalid@example.com".to_string(), "".to_string()),
        ];

        let body_result = ZeroBounce::new("some_api_key").batch_validate_prepare_body(
            emails_and_ip_addresses,
            &crate::utility::structures::validate_options::ZBValidateOptions::default(),
        );

        assert!(body_result.is_ok())
    }
}
