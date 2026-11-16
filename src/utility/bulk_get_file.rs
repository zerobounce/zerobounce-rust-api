//! Helpers for bulk getfile responses (v2): JSON error detection and messages.

use serde_json::Value;

/// The body without any leading U+FEFF (byte order marks). `str::trim` does
/// not remove them, so a BOM in front of a JSON error would hide it.
pub(crate) fn strip_leading_boms(body: &str) -> &str {
    body.trim_start_matches(|c: char| c == '\u{FEFF}' || c.is_whitespace())
}

/// Whether a getfile response body looks like a JSON error payload (including HTTP 200).
///
/// A download is a CSV, so a body that starts with `{` or `[` (after any
/// byte order marks and whitespace) is an error even when it is not valid
/// JSON, and whatever fields it has.
pub fn get_file_json_indicates_error(body: &str) -> bool {
    let t = strip_leading_boms(body);
    t.starts_with('{') || t.starts_with('[')
}

/// Human-readable message from a getfile JSON error body.
pub fn format_get_file_error_message(body: &str) -> String {
    let body = strip_leading_boms(body);
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
    lower.contains("application/json") || lower.contains("+json")
}

pub(crate) fn should_treat_get_file_body_as_error(body: &str, content_type: &str) -> bool {
    if content_type_includes_application_json(content_type) {
        return true;
    }
    get_file_json_indicates_error(body)
}

/// Whether a `Content-Encoding` value means the body is not the plain bytes.
/// The client does not enable reqwest's decompression, so a compressed body
/// would otherwise be saved as the raw compressed bytes.
pub(crate) fn content_encoding_is_transformed<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> bool {
    values.into_iter().any(|value| match value {
        // A value that is not text is not `identity` either.
        None => true,
        Some(text) => {
            let text = text.trim();
            !(text.is_empty() || text.eq_ignore_ascii_case("identity"))
        }
    })
}

/// The message for a `Content-Encoding` this client does not decode.
pub(crate) const ENCODED_BODY_MESSAGE: &str =
    "ZeroBounce returned an encoded (compressed) download, which this client does not decode.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_file_json_indicates_error_matches_other_sdks() {
        assert!(get_file_json_indicates_error(r#"{"success":false,"message":""}"#));
        // Any JSON object or array is an error, not a result file.
        assert!(get_file_json_indicates_error(r#"{"file_id":"x"}"#));
        assert!(get_file_json_indicates_error(r#"["a","b"]"#));
        assert!(get_file_json_indicates_error("{not json"));
        assert!(!get_file_json_indicates_error("email,status\na@b.c,valid\n"));
    }

    #[test]
    fn leading_boms_do_not_hide_json() {
        assert!(get_file_json_indicates_error("\u{FEFF}{\"file_id\":\"x\"}"));
        assert!(get_file_json_indicates_error("\u{FEFF}\u{FEFF}{not json"));
        assert!(get_file_json_indicates_error("\u{FEFF} \u{FEFF}[1]"));
        assert!(!get_file_json_indicates_error("\u{FEFF}email,status\n"));
        assert_eq!(strip_leading_boms("\u{FEFF}\u{FEFF}"), "");
        assert_eq!(
            format_get_file_error_message("\u{FEFF}\u{FEFF}{\"message\":\"m\"}"),
            "m"
        );
    }

    #[test]
    fn json_content_types_are_errors() {
        assert!(should_treat_get_file_body_as_error("a,b", "application/json"));
        assert!(should_treat_get_file_body_as_error("a,b", "application/problem+json"));
        assert!(should_treat_get_file_body_as_error("a,b", "Application/Vnd.API+JSON; charset=utf-8"));
        assert!(!should_treat_get_file_body_as_error("a,b", "text/csv"));
    }

    #[test]
    fn content_encoding_other_than_identity_is_transformed() {
        assert!(!content_encoding_is_transformed(std::iter::empty()));
        assert!(!content_encoding_is_transformed([Some(""), Some(" Identity ")]));
        assert!(content_encoding_is_transformed([Some("gzip")]));
        assert!(content_encoding_is_transformed([Some("identity, gzip")]));
        assert!(content_encoding_is_transformed([None]));
    }
}
