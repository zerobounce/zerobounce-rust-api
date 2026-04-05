//! Helpers for bulk getfile responses (v2): JSON error detection and messages.

use serde_json::Value;

/// Whether a getfile response body looks like a JSON error payload (including HTTP 200).
pub fn get_file_json_indicates_error(body: &str) -> bool {
    let t = body.trim_start();
    if !t.starts_with('{') {
        return false;
    }
    let v: Value = match serde_json::from_str(t) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let o = match v.as_object() {
        Some(o) => o,
        None => return false,
    };
    if let Some(s) = o.get("success") {
        if s == &Value::Bool(false) {
            return true;
        }
        if let Some(s) = s.as_str() {
            if s.eq_ignore_ascii_case("false") {
                return true;
            }
        }
    }
    for k in ["message", "error", "error_message"] {
        if let Some(v) = o.get(k) {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return true;
                }
            }
            if let Some(a) = v.as_array() {
                if !a.is_empty() {
                    return true;
                }
            }
        }
    }
    o.contains_key("success")
}

/// Human-readable message from a getfile JSON error body.
pub fn format_get_file_error_message(body: &str) -> String {
    let v: Value = match serde_json::from_str(body.trim()) {
        Ok(v) => v,
        Err(_) => {
            return if body.is_empty() {
                "Invalid getfile response".to_string()
            } else {
                body.to_string()
            };
        }
    };
    let o = match v.as_object() {
        Some(o) => o,
        None => {
            return if body.is_empty() {
                "Invalid getfile response".to_string()
            } else {
                body.to_string()
            };
        }
    };
    for k in ["message", "error", "error_message"] {
        if let Some(v) = o.get(k) {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
            if let Some(a) = v.as_array() {
                if let Some(first) = a.first() {
                    if let Some(s) = first.as_str() {
                        if !s.is_empty() {
                            return s.to_string();
                        }
                    }
                }
            }
        }
    }
    body.to_string()
}

pub(crate) fn content_type_includes_application_json(content_type: &str) -> bool {
    let lower: String = content_type.to_ascii_lowercase();
    lower.contains("application/json")
}

pub(crate) fn should_treat_get_file_body_as_error(body: &str, content_type: &str) -> bool {
    if content_type_includes_application_json(content_type) {
        return true;
    }
    get_file_json_indicates_error(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_file_json_indicates_error_matches_other_sdks() {
        assert!(get_file_json_indicates_error(r#"{"success":false,"message":""}"#));
        assert!(!get_file_json_indicates_error(r#"{"file_id":"x"}"#));
    }
}
