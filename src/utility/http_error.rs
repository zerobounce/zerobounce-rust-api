use super::{ZBError, ZBResult};

pub fn body_looks_like_html(body: &str) -> bool {
    let sample = if body.len() > 2000 {
        &body[..2000]
    } else {
        body
    };
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

fn first_api_error_field(object: &serde_json::Map<String, serde_json::Value>) -> String {
    // "detail" is how the Inbox Placement Tester (Django REST framework) reports errors.
    for key in ["error", "message", "error_message", "Message", "detail"] {
        match object.get(key) {
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

pub fn message_from_body(status: u16, content_type: &str, body: &str) -> String {
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
        || (!trimmed.starts_with('{') && !trimmed.starts_with('['))
    {
        return Err(ZBError::api(
            status,
            message_from_body(status, content_type, body),
        ));
    }
    // LIVE: {"success": false} on an HTTP 200 is a failure, not a result. The
    // API answers an unknown bulk file id that way, with a 200.
    if trimmed.starts_with('{') {
        if let Ok(serde_json::Value::Object(object)) =
            serde_json::from_str::<serde_json::Value>(trimmed)
        {
            if object.get("success").is_some_and(is_falsy_api_success) {
                return Err(ZBError::api(
                    status,
                    message_from_body(status, content_type, body),
                ));
            }
        }
    }
    Ok(())
}

pub fn parse_json_body<T: serde::de::DeserializeOwned>(body: &str) -> ZBResult<T> {
    serde_json::from_str(body)
        .map_err(|_| ZBError::explicit("ZeroBounce returned a response that is not valid JSON."))
}
