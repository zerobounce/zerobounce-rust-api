use std::collections::HashMap;

use serde::Serialize;
use serde_json::from_str;
use serde_json::{Map as SerdeMap, Value};

use crate::{ZeroBounce,  ZBResult};
use crate::utility::{ENDPOINT_VALIDATE, ZBError, ENDPOINT_BATCH_VALIDATE, CONTENT_TYPE_JSON};
use crate::utility::structures::validation::{ZBValidation, ZBBatchValidation};


impl ZeroBounce {

    pub fn validate_email_and_ip(&self, email: &str, ip_address: &str) -> ZBResult<ZBValidation> {
        let mut query_args = HashMap::from([
            ("api_key", self.api_key.as_str()),
            ("email", email),
        ]);

        if ip_address.is_empty() {
            query_args.insert("ip_address", ip_address);
        }

        let response_content = self.generic_get_request(
            self.url_provider.url_of(ENDPOINT_VALIDATE), query_args
        )?;

        let validation = from_str::<ZBValidation>(&response_content)?;
        Ok(validation)
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
    fn batch_validate_prepare_body(&self, emails_and_ip_addresses: Vec<(String, String)>) -> ZBResult<String> {
        let email_batch = emails_and_ip_addresses
            .into_iter()
            .map(|(email, ip_address)|
                [
                    ("email_address".to_string(), Value::String(email)),
                    ("ip_address".to_string(), Value::String(ip_address)),
                ]
            )
            .map(SerdeMap::<String, Value>::from_iter)
            .map(Value::Object)
            .collect::<Vec<Value>>();

        let request_body_map = SerdeMap::from_iter([
            ("api_key".to_string(), Value::String(self.api_key.clone())),
            ("email_batch".to_string(), Value::Array(email_batch)),
        ]);

        // let request_body_object = Value::Object(request_body_map);
        let mut serializer = serde_json::Serializer::new(Vec::new());
        Value::Object(request_body_map)
            .serialize(&mut serializer)
            .map_err(ZBError::JsonError)?;

        let final_string = String::from_utf8(serializer.into_inner())
            .map_err(|error| ZBError::ExplicitError(error.to_string()))?;

        Ok(final_string)
    }

    pub fn batch_validate(&self, emails_and_ip_addresses: Vec<(String, String)>) -> ZBResult<ZBBatchValidation> {
        let body_content = self.batch_validate_prepare_body(emails_and_ip_addresses)?;
        let url = self.url_provider.url_of(ENDPOINT_BATCH_VALIDATE);

        let response = self.client.post(url)
            .body(body_content)
            .header("content-type", CONTENT_TYPE_JSON)
            .send()?;

        let response_ok = response.status().is_success();
        let response_content = response.text()?;

        if !response_ok {
            return Err(ZBError::ExplicitError(response_content));
        }

        let validation = from_str::<ZBBatchValidation>(response_content.as_str())?;
        Ok(validation)
    }

}

#[cfg(test)]
mod test {
    use crate::ZeroBounce;

    #[test]
    fn test_serializing_example() {
        let emails_and_ip_addresses = vec![
            ("valid@example.com".to_string(), "123.123.123.123".to_string()),
            ("invalid@example.com".to_string(), "".to_string()),
        ];

        let body_result = ZeroBounce::new("some_api_key")
            .batch_validate_prepare_body(emails_and_ip_addresses);

        assert!(body_result.is_ok())
    }

}
