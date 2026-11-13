use super::{ZBError, ZBResult};

/// At most the first `max_bytes` bytes of `text`, cut back to a character
/// boundary. Slicing at a fixed byte offset panics when a multi-byte
/// character straddles it, and response bodies are not ASCII-only.
/// (`str::floor_char_boundary` would do this, but it is newer than the MSRV.)
fn prefix_at_char_boundary(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

pub fn body_looks_like_html(body: &str) -> bool {
    let sample = prefix_at_char_boundary(body, 2000);
    let lower = sample.to_ascii_lowercase();
    lower.contains("<!doctype html")
        || lower.contains("<html")
        || lower.contains("<head")
        || lower.contains("<body")
}

pub fn content_type_is_html(content_type: &str) -> bool {
    content_type.to_ascii_lowercase().contains("text/html")
}

pub fn strip_error_prefix(message: &str) -> String {
    let mut message = message.trim().to_string();
    loop {
        let trimmed = message.trim();
        if let Some(rest) = trimmed.strip_prefix("Error:") {
            message = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("[Error]:") {
            message = rest.trim().to_string();
            continue;
        }
        return trimmed.to_string();
    }
}

fn html_title(body: &str) -> String {
    let lower = body.to_ascii_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(open_end_rel) = lower[start..].find('>') else {
        return String::new();
    };
    let open_end = start + open_end_rel;
    let Some(close_rel) = lower[open_end..].find("</title>") else {
        return String::new();
    };
    body[open_end + 1..open_end + close_rel].trim().to_string()
}

/// Keys that only ever mean an error. Narrower than `MESSAGE_KEYS` on purpose:
/// a successful response uses "message"/"Message" too (a bulk submit answers
/// {"success": true, "message": "File Accepted"}, a greylisted validate carries
/// "Message" next to the result), so those cannot decide success by themselves.
/// "Error" is how the filters spell it: {"Error": "Invalid API key ..."}.
const ERROR_ONLY_KEYS: [&str; 3] = ["error", "error_message", "Error"];

/// Every key an error message may hide under, once the response is already
/// known to be a failure.
/// "detail" is how the Inbox Placement Tester (Django REST framework) reports errors.
const MESSAGE_KEYS: [&str; 6] = ["error", "message", "error_message", "Message", "Error", "detail"];

fn api_error_field(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> String {
    for key in keys {
        match object.get(*key) {
            Some(serde_json::Value::String(text)) if !text.trim().is_empty() => {
                return text.trim().to_string();
            }
            Some(serde_json::Value::Array(items)) => {
                if let Some(serde_json::Value::String(text)) = items.first() {
                    if !text.trim().is_empty() {
                        return text.trim().to_string();
                    }
                }
            }
            _ => {}
        }
    }
    String::new()
}

fn first_api_error_field(object: &serde_json::Map<String, serde_json::Value>) -> String {
    api_error_field(object, &MESSAGE_KEYS)
}

/// The API's own message when an HTTP 200 body is nothing but an error: an
/// "error"/"error_message" field, and no "success" field to gate on. An
/// invalid or drained key answers every v2 endpoint that way.
fn error_only_body(object: &serde_json::Map<String, serde_json::Value>) -> String {
    if object.contains_key("success") {
        return String::new();
    }
    api_error_field(object, &ERROR_ONLY_KEYS)
}

/// The error for a 3xx. Redirects are never followed: the API key travels
/// with the request, and a redirect could send it to another host.
pub(crate) const REDIRECT_REFUSED_MESSAGE: &str =
    "ZeroBounce returned a redirect, which this client does not follow.";

pub fn message_from_body(status: u16, content_type: &str, body: &str) -> String {
    if (300..400).contains(&status) {
        return REDIRECT_REFUSED_MESSAGE.into();
    }
    if body_looks_like_html(body) || content_type_is_html(content_type) {
        let title = html_title(body);
        if title.is_empty() {
            return "ZeroBounce returned an HTML error page instead of an API response.".into();
        }
        return format!(
            "ZeroBounce returned an HTML error page (\"{title}\") instead of an API response."
        );
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return format!("Empty response from ZeroBounce (HTTP {status})");
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(serde_json::Value::Object(object)) => {
                if let Some(serde_json::Value::String(title)) = object.get("title") {
                    let access = title.contains("Access denied") || title.contains("1020");
                    let cloudflare =
                        object.get("cloudflare_error") == Some(&serde_json::Value::Bool(true));
                    if access && cloudflare {
                        return title.clone();
                    }
                }
                let message = first_api_error_field(&object);
                if !message.is_empty() {
                    return strip_error_prefix(&message);
                }
                if trimmed.len() > 1000 {
                    return "ZeroBounce returned a response that is not valid JSON.".into();
                }
                return strip_error_prefix(trimmed);
            }
            Ok(_) => {
                if trimmed.len() > 1000 {
                    return "ZeroBounce returned a response that is not valid JSON.".into();
                }
                return strip_error_prefix(trimmed);
            }
            Err(_) => return "ZeroBounce returned a response that is not valid JSON.".into(),
        }
    }
    if trimmed.len() > 1000 {
        return "ZeroBounce returned a response that is not valid JSON.".into();
    }
    let message = strip_error_prefix(trimmed);
    if message.is_empty() {
        return format!("Empty response from ZeroBounce (HTTP {status})");
    }
    message
}

/// Whether a "success" field means failure. The API sends it as a JSON false,
/// "false" or "False" depending on the endpoint.
fn is_falsy_api_success(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(flag) => !flag,
        serde_json::Value::String(text) => text.eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// Accept a 2xx JSON document. Anything else is a short error.
pub fn ensure_json_api_body(status: u16, content_type: &str, body: &str) -> ZBResult<()> {
    let success = (200..300).contains(&status);
    let trimmed = body.trim();
    if !success
        || trimmed.is_empty()
        || body_looks_like_html(body)
        || content_type_is_html(content_type)
    {
        return Err(ZBError::api(
            status,
            message_from_body(status, content_type, body),
        ));
    }
    // Strict RFC 8259 JSON, parsed from the body as received: serde_json
    // allows only space, tab, CR and LF around the value, and no BOM,
    // comment, trailing comma or value, NaN or other extension
    // (tests/fixtures/strict-json-corpus.tsv). `str::trim` would also drop
    // Unicode spaces such as U+00A0. Text that is not JSON is its own message;
    // JSON that does not parse is not an API answer either. (A body that
    // parses but does not fit the response type is the SDK's parse error.)
    let parsed = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(parsed) => parsed,
        Err(_) if !trimmed.starts_with('{') && !trimmed.starts_with('[') => {
            return Err(ZBError::api(
                status,
                message_from_body(status, content_type, body),
            ))
        }
        Err(_) => {
            return Err(ZBError::api(
                status,
                "ZeroBounce returned a response that is not valid JSON.",
            ))
        }
    };
    // LIVE: {"success": false} on an HTTP 200 is a failure, not a result. The
    // API answers an unknown bulk file id that way, with a 200.
    if let serde_json::Value::Object(object) = parsed {
        if object.get("success").is_some_and(is_falsy_api_success) {
            return Err(ZBError::api(
                status,
                message_from_body(status, content_type, body),
            ));
        }
        // LIVE: an invalid or drained key answers with an HTTP 200 and a
        // body that is only {"error": "Invalid API key"} -- no "success"
        // field to gate on. Without this the body reached serde, which
        // failed on the unexpected shape and reported a parse error,
        // hiding the real cause.
        if !error_only_body(&object).is_empty() {
            return Err(ZBError::api(
                status,
                message_from_body(status, content_type, body),
            ));
        }
    }
    Ok(())
}

/// Whether a JSON body is nothing but a message: an object with no "success"
/// field whose keys are all message keys. The keys decide, whatever the
/// message's value (a string, blank, null or a list), since no result object
/// has these keys.
fn message_only_body(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    !object.is_empty()
        && !object.contains_key("success")
        && object.keys().all(|key| MESSAGE_KEYS.contains(&key.as_str()))
}

/// For a call whose result is never a bare message -- the email finder and
/// domain search -- a message-only body is an error. LIVE: guessformat answers
/// an invalid or drained key with an HTTP 200 and
/// {"Message": "Invalid API key or your account ran out of credits"}.
/// Not for validate: a repeated greylist_processing request answers
/// {"Message": "Greylist validation still in progress..."}, which is a result.
pub fn reject_message_only_body(status: u16, content_type: &str, body: &str) -> ZBResult<()> {
    if let Ok(serde_json::Value::Object(object)) = serde_json::from_str::<serde_json::Value>(body.trim()) {
        if message_only_body(&object) {
            return Err(ZBError::api(
                status,
                message_from_body(status, content_type, body),
            ));
        }
    }
    Ok(())
}

/// The message for a 2xx body that is valid JSON but not the shape the call
/// returns (`0`, `"x"`, `null`, a list where an object is expected, a field of
/// the wrong type).
pub const UNEXPECTED_JSON_SHAPE_MESSAGE: &str =
    "ZeroBounce returned JSON that is not the expected response.";

/// Parse a 2xx body the caller has already checked
/// ([`ensure_json_api_body`]) as the call's result type. A body that does not
/// fit is the API's answer, not a transport or SDK error: a
/// [`ZBError::ApiError`] with status 200 and [`UNEXPECTED_JSON_SHAPE_MESSAGE`].
/// The message carries no body content.
///
/// A JSON array is never read into a struct: serde's derived structs also
/// take their fields positionally from an array (`["x"]` would fill the first
/// field), so the body is read through [`ObjectsOnly`], which offers a struct
/// only a JSON object, at every depth.
pub fn parse_json_body<T: serde::de::DeserializeOwned>(body: &str) -> ZBResult<T> {
    serde_json::from_str::<serde_json::Value>(body)
        .and_then(|value| T::deserialize(ObjectsOnly(value)))
        .map_err(|_| ZBError::api(200, UNEXPECTED_JSON_SHAPE_MESSAGE))
}

/// A [`serde_json::Value`] deserializer that refuses an array where a struct
/// is expected, at every depth. Everything else behaves as the `Value`'s own
/// deserializer: the values inside objects and arrays are `ObjectsOnly` too.
struct ObjectsOnly(serde_json::Value);

impl<'de> serde::Deserializer<'de> for ObjectsOnly {
    type Error = serde_json::Error;

    fn deserialize_any<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            serde_json::Value::Array(items) => {
                let mut seq = serde::de::value::SeqDeserializer::<_, serde_json::Error>::new(
                    items.into_iter().map(ObjectsOnly),
                );
                let value = visitor.visit_seq(&mut seq)?;
                seq.end()?;
                Ok(value)
            }
            serde_json::Value::Object(members) => {
                let mut map = serde::de::value::MapDeserializer::<_, serde_json::Error>::new(
                    members.into_iter().map(|(key, value)| (key, ObjectsOnly(value))),
                );
                let value = visitor.visit_map(&mut map)?;
                map.end()?;
                Ok(value)
            }
            other => other.deserialize_any(visitor),
        }
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0 {
            serde_json::Value::Object(_) => self.deserialize_any(visitor),
            serde_json::Value::Array(_) => {
                Err(serde::de::Error::invalid_type(serde::de::Unexpected::Seq, &visitor))
            }
            other => other.deserialize_struct(name, fields, visitor),
        }
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            serde_json::Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_enum(name, variants, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple tuple_struct map identifier ignored_any
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde_json::Error> for ObjectsOnly {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utility::ZBError;

    const JSON: &str = "application/json";

    fn error_message(status: u16, body: &str) -> String {
        match ensure_json_api_body(status, JSON, body) {
            Err(ZBError::ApiError { message, .. }) => message,
            other => panic!("expected an ApiError, got {other:#?}"),
        }
    }

    /// LIVE: an invalid or drained key answers with an HTTP 200 and nothing
    /// but an "error" field. It used to reach serde and surface as a bogus
    /// "not valid JSON" error.
    #[test]
    fn test_200_with_error_field_is_an_api_error() {
        assert_eq!(
            error_message(200, r#"{"error":"Invalid API key"}"#),
            "Invalid API key"
        );
        assert_eq!(
            error_message(
                200,
                r#"{"error":"Invalid API key or your account ran out of credits"}"#
            ),
            "Invalid API key or your account ran out of credits"
        );
    }

    #[test]
    fn test_200_with_error_message_field_is_an_api_error() {
        assert_eq!(
            error_message(200, r#"{"error_message":"Missing parameter: api_key."}"#),
            "Missing parameter: api_key."
        );
    }

    #[test]
    fn test_200_with_empty_error_field_is_not_an_error() {
        // A payload that merely carries an empty "error" is still a result.
        assert!(ensure_json_api_body(200, JSON, r#"{"error":"","found":true}"#).is_ok());
    }

    /// A falsy "success" still decides on its own, and a truthy one wins over
    /// a message field.
    #[test]
    fn test_success_field_still_decides() {
        assert_eq!(
            error_message(200, r#"{"success":false,"message":"File cannot be found."}"#),
            "File cannot be found."
        );
        assert!(ensure_json_api_body(
            200,
            JSON,
            r#"{"success":true,"message":"File Accepted","file_id":"abc"}"#
        )
        .is_ok());
    }

    /// A greylisted validate is a result, not an error, and it carries
    /// "Message" with no "success" field -- the shape the narrow key list
    /// protects.
    #[test]
    fn test_greylisted_validation_is_not_an_error() {
        assert!(ensure_json_api_body(
            200,
            JSON,
            r#"{"address":"a@b.com","status":"unknown","sub_status":"greylisted","Message":"Greylisted, try again later"}"#
        )
        .is_ok());
    }

    /// Batch validate reports per-address failures in a nested "errors" array
    /// while the call itself succeeded.
    #[test]
    fn test_batch_nested_errors_are_not_a_failure() {
        assert!(ensure_json_api_body(
            200,
            JSON,
            r#"{"email_batch":[],"errors":[{"error":"Invalid email address","email_address":"bad@"}]}"#
        )
        .is_ok());
    }

    /// The HTML sniff looked at `&body[..2000]`, which panicked when a
    /// multi-byte character straddled byte 2000.
    #[test]
    fn test_html_sniff_does_not_split_a_multi_byte_character() {
        for wide in ["é", "€", "😀"] {
            for start in 2000 - wide.len() + 1..2000 {
                let body = format!("{}{wide}{}", "a".repeat(start), "b".repeat(100));
                assert!(!body.is_char_boundary(2000), "{wide} at {start}");
                assert!(!body_looks_like_html(&body), "{wide} at {start}");
                let page = format!("<html>{}{wide}</html>", "a".repeat(start - 6));
                assert!(body_looks_like_html(&page), "{wide} at {start}");
                assert!(ensure_json_api_body(200, JSON, &format!("{{\"city\":\"{}{wide}\"}}", "a".repeat(start - 9))).is_ok());
            }
        }
        assert_eq!(prefix_at_char_boundary("aé", 2), "a");
        assert_eq!(prefix_at_char_boundary("aé", 3), "aé");
        assert_eq!(prefix_at_char_boundary("", 0), "");
    }
}
