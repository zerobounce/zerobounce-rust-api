use std::collections::{BTreeSet, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::NaiveDate;
use zero_bounce::utility::structures::bulk::{
    zb_download_type, ZBBulkResponse, ZBEmailFinderFileOptions, ZBFile, ZBGetFileOptions,
};
use zero_bounce::utility::structures::validate_options::ZBValidateOptions;
use zero_bounce::api::inbox_placement::{
    CreateInboxPlacementTestOptions, InboxPlacementProviderSelection,
    ListInboxPlacementTestsOptions,
};
use zero_bounce::utility::{ZBError, BULK_URI};
use zero_bounce::{ApiBaseUrl, ZeroBounce};

struct Recorded {
    method: String,
    path: String,
    api_key: String,
    #[allow(dead_code)]
    authorization: String,
    fields: HashMap<String, String>,
}

struct Fake {
    records: Mutex<Vec<Recorded>>,
}

impl Fake {
    fn take(&self) -> Vec<Recorded> {
        std::mem::take(&mut *self.records.lock().unwrap())
    }
    fn len(&self) -> usize {
        self.records.lock().unwrap().len()
    }
}

fn html_page() -> String {
    let mut page = String::from("<!DOCTYPE html><html lang=\"en-US\"><head><title>Access denied | zerobounce.net | Zero Bounce</title></head><body>");
    for _ in 0..20000 {
        page.push_str("<p>blocked</p>");
    }
    page.push_str("</body></html>");
    page
}

fn json_escape(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

fn success_body(path: &str, key: &str) -> (u16, &'static str, String) {
    let quoted = json_escape(key);
    if path.contains("getfile") {
        return (
            200,
            "application/octet-stream",
            format!("email\n{key}@example.com\n"),
        );
    }
    if path.contains("/api/ipt") {
        if key.starts_with("ipt-detail") {
            // The Inbox Placement Tester (Django REST framework) reports "detail".
            return (
                404,
                "application/json",
                r#"{"detail":"Inbox placement test not found."}"#.to_string(),
            );
        }
        return (
            200,
            "application/json",
            format!(r#"{{"ok":true,"echo_api_key":{quoted}}}"#),
        );
    }
    if path.contains("listS3files") {
        return (
            200,
            "application/json",
            format!(r#"[{{"name":{quoted},"size":1,"last_modified":""}}]"#),
        );
    }
    if path.contains("validatebatch") {
        return (
            200,
            "application/json",
            format!(
                r#"{{"email_batch":[{{"address":{quoted},"status":"valid","sub_status":"","free_email":false,"did_you_mean":null,"account":null,"domain":null,"domain_age_days":null,"active_in_days":null,"active_first_seen":null,"smtp_provider":null,"catchall_domain":null,"mx_record":null,"mx_found":null,"firstname":null,"lastname":null,"gender":null,"country":null,"region":null,"city":null,"zipcode":null,"processed_at":"2023-04-18 12:09:39.074"}}],"errors":[]}}"#
            ),
        );
    }
    if path.ends_with("/validate") {
        return (
            200,
            "application/json",
            format!(
                r#"{{"address":{quoted},"status":"valid","sub_status":"","free_email":false,"did_you_mean":null,"account":null,"domain":null,"domain_age_days":null,"active_in_days":null,"active_first_seen":null,"smtp_provider":null,"catchall_domain":null,"mx_record":null,"mx_found":null,"firstname":null,"lastname":null,"gender":null,"country":null,"region":null,"city":null,"zipcode":null,"processed_at":"2023-04-18 12:09:39.074"}}"#
            ),
        );
    }
    if path.ends_with("/scoring") {
        return (
            200,
            "application/json",
            format!(r#"{{"email":{quoted},"score":10}}"#),
        );
    }
    if path.contains("getcredits") {
        return (200, "application/json", r#"{"Credits":"7"}"#.to_string());
    }
    if path.contains("getapiusage") {
        return (200, "application/json", r#"{"total":1,"status_valid":0,"status_invalid":0,"status_catch_all":0,"status_do_not_mail":0,"status_spamtrap":0,"status_unknown":0,"sub_status_toxic":0,"sub_status_disposable":0,"sub_status_role_based":0,"sub_status_possible_trap":0,"sub_status_global_suppression":0,"sub_status_timeout_exceeded":0,"sub_status_mail_server_temporary_error":0,"sub_status_mail_server_did_not_respond":0,"sub_status_greylisted":0,"sub_status_antispam_system":0,"sub_status_does_not_accept_mail":0,"sub_status_exception_occurred":0,"sub_status_failed_syntax_check":0,"sub_status_mailbox_not_found":0,"sub_status_unroutable_ip_address":0,"sub_status_possible_typo":0,"sub_status_no_dns_entries":0,"sub_status_role_based_catch_all":0,"sub_status_accept_all":0,"sub_status_mailbox_quota_exceeded":0,"sub_status_forcible_disconnect":0,"sub_status_failed_smtp_connection":0,"sub_status_mx_forward":0,"sub_status_alternate":0,"sub_status_allowed":0,"sub_status_blocked":0,"sub_status_gold":0,"sub_status_role_based_accept_all":0,"start_date":"1/1/2018","end_date":"1/31/2018"}"#.to_string());
    }
    if path.ends_with("/activity") {
        return (
            200,
            "application/json",
            r#"{"found":true,"active_in_days":"3"}"#.to_string(),
        );
    }
    if path.contains("filestatus") {
        return (
            200,
            "application/json",
            format!(
                r#"{{"success":true,"file_id":{quoted},"file_name":"f.csv","file_status":"Complete","complete_percentage":"100%","return_url":null,"upload_date":"2023-04-26T17:52:23Z","error_reason":null}}"#
            ),
        );
    }
    if path.contains("guessformat") {
        return (
            200,
            "application/json",
            format!(
                r#"{{"email":{quoted},"domain":{quoted},"format":"","status":"","sub_status":"","confidence":"","did_you_mean":"","failure_reason":"","other_domain_formats":[]}}"#
            ),
        );
    }
    (
        200,
        "application/json",
        format!(
            r#"{{"success":true,"message":"ok","file_name":"f.csv","file_id":{quoted},"email":{quoted},"domain":{quoted},"echo_api_key":{quoted}}}"#
        ),
    )
}

fn answer(path: &str, key: &str) -> (u16, &'static str, String) {
    let page = html_page();
    if key.starts_with("html-403-") {
        return (403, "text/html; charset=UTF-8", page);
    }
    if key.starts_with("html-200-") {
        return (200, "text/html; charset=UTF-8", page);
    }
    if key.starts_with("empty-") {
        return (200, "application/json", String::new());
    }
    if key.starts_with("malformed-") {
        return (
            200,
            "application/json",
            format!("<<<not-json>>>{}", "x".repeat(5000)),
        );
    }
    if key.starts_with("cf-json-") {
        return (403, "application/json", r#"{"type":"https://developers.cloudflare.com/support/troubleshooting/http-status-codes/cloudflare-1xxx-errors/error-1020/","title":"Error 1020: Access denied","status":403,"detail":"The request was blocked by a Cloudflare firewall rule configured by the site owner.","error_code":1020,"cloudflare_error":true,"what_you_should_do":"Do not retry. This block is intentional."}"#.to_string());
    }
    if key.starts_with("redirect-") && !path.starts_with("/collect") {
        return (302, "text/html", String::new());
    }
    if key.starts_with("double-error-") {
        return (
            400,
            "text/plain",
            "Error: Error: api_key is invalid".to_string(),
        );
    }
    // LIVE (2026-11-11): what an invalid or drained key gets back, each with an
    // HTTP 200 and no "success" field.
    if key.starts_with("dead-key-") {
        let body = if path.contains("guessformat") {
            r#"{"Message":"Invalid API key or your account ran out of credits"}"#
        } else if path.contains("filters") {
            r#"{  "Error": "Invalid API key or your account ran out of credits"}"#
        } else if path.contains("getapiusage") {
            r#"{"error":"Invalid API key"}"#
        } else if path.contains("getcredits") {
            r#"{"Credits":"-1"}"#
        } else {
            r#"{"error":"Invalid API key or your account ran out of credits"}"#
        };
        return (200, "application/json", body.to_string());
    }
    // The documented answer to a repeated greylist_processing validate: a
    // result whose only field is Message, not an error.
    if key.starts_with("greylist-pending-") && path.ends_with("/validate") {
        return (200, "application/json", r#"{"Message":"Greylist validation still in progress. Please try again later."}"#.to_string());
    }
    if key.starts_with("filter-message-") && path.contains("filters") {
        return (200, "application/json", r#"{"Message":"Filter has been added."}"#.to_string());
    }
    if path.contains("validatebatch") && key.starts_with("batch-account-") {
        return (200, "application/json", r#"{"email_batch":[],"errors":[{"error":"Invalid API Key or your account ran out of credits","email_address":"all"}]}"#.to_string());
    }
    if path.contains("validatebatch") && key.starts_with("batch-partial-") {
        return (200, "application/json", r#"{"email_batch":[{"address":"good@example.com","status":"valid","sub_status":"","free_email":false,"did_you_mean":null,"account":null,"domain":null,"domain_age_days":null,"active_in_days":null,"active_first_seen":null,"smtp_provider":null,"catchall_domain":null,"mx_record":null,"mx_found":null,"firstname":null,"lastname":null,"gender":null,"country":null,"region":null,"city":null,"zipcode":null,"processed_at":"2023-04-18 12:09:39.074"}],"errors":[{"error":"Invalid email address","email_address":"bad@"}]}"#.to_string());
    }
    success_body(path, key)
}

fn url_decode(value: &str) -> String {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = u8::from_str_radix(&value[i + 1..i + 3], 16).unwrap_or(0);
                out.push(hex);
                i += 2;
            }
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_query(query: &str, fields: &mut HashMap<String, String>) {
    if query.is_empty() {
        return;
    }
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = url_decode(parts.next().unwrap_or(""));
        let value = url_decode(parts.next().unwrap_or(""));
        fields.insert(key, value);
    }
}

fn flatten_json(body: &str, fields: &mut HashMap<String, String>) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(object) = value.as_object() {
            for (key, item) in object {
                let text = match item {
                    serde_json::Value::String(text) => text.clone(),
                    serde_json::Value::Bool(flag) => {
                        if *flag {
                            "true".into()
                        } else {
                            "false".into()
                        }
                    }
                    other => other.to_string(),
                };
                fields.insert(key.clone(), text);
            }
        }
    }
}

fn record_request(head: &str, body: &[u8]) -> Recorded {
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut fields = HashMap::new();
    parse_query(query, &mut fields);
    let mut content_type = String::new();
    let mut authorization = String::new();
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-type:") {
            content_type = value.trim().to_string();
        }
        if lower.starts_with("authorization:") {
            authorization = line["authorization:".len()..].trim().to_string();
        }
    }
    let body_text = String::from_utf8_lossy(body);
    if content_type.contains("application/json") || body_text.trim_start().starts_with('{') {
        flatten_json(&body_text, &mut fields);
    } else if content_type.contains("application/x-www-form-urlencoded")
        || body_text.contains('=') && !content_type.contains("multipart")
    {
        parse_query(&body_text, &mut fields);
    } else if content_type.contains("multipart") {
        for part in body_text.split("\r\n\r\n") {
            if let Some(header) = part.split("\r\n").next() {
                if let Some(name_at) = header.find("name=\"") {
                    let rest = &header[name_at + 6..];
                    if let Some(end) = rest.find('"') {
                        let name = rest[..end].to_string();
                        let value = if header.contains("filename=") {
                            "<file>".to_string()
                        } else {
                            part.split("\r\n\r\n")
                                .nth(1)
                                .unwrap_or("")
                                .trim_end_matches("--")
                                .trim()
                                .to_string()
                        };
                        fields.insert(name, value);
                    }
                }
            }
        }
        // The split above is coarse. Walk the raw body for name= fields.
        let raw = body_text.as_ref();
        let mut search_from = 0;
        while let Some(rel) = raw[search_from..].find("name=\"") {
            let start = search_from + rel + 6;
            if let Some(end_rel) = raw[start..].find('"') {
                let name = raw[start..start + end_rel].to_string();
                let after = start + end_rel;
                let filename =
                    raw[after..].starts_with("\"; filename=") || raw[after..].contains("filename=");
                if let Some(sep) = raw[after..].find("\r\n\r\n") {
                    let value_start = after + sep + 4;
                    let value_end = raw[value_start..]
                        .find("\r\n--")
                        .map(|i| value_start + i)
                        .unwrap_or(raw.len());
                    let value = if filename || raw[after..value_start].contains("filename=") {
                        "<file>".to_string()
                    } else {
                        raw[value_start..value_end].to_string()
                    };
                    fields.insert(name, value);
                    search_from = value_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    // The Inbox Placement Tester authenticates with a Bearer header rather than
    // an api_key parameter, so the key can arrive either way.
    let api_key = fields
        .get("api_key")
        .cloned()
        .or_else(|| {
            authorization
                .strip_prefix("Bearer ")
                .map(|key| key.trim().to_string())
        })
        .unwrap_or_default();
    Recorded {
        method,
        path: path.to_string(),
        api_key,
        authorization,
        fields,
    }
}

fn handle(mut stream: TcpStream, fake: Arc<Fake>) {
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    let mut buffer = Vec::new();
    let mut temp = [0u8; 8192];
    loop {
        match stream.read(&mut temp) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);
                if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&buffer[..pos]).to_string();
                    let mut length = 0usize;
                    for line in header.split("\r\n") {
                        if let Some(value) =
                            line.to_ascii_lowercase().strip_prefix("content-length:")
                        {
                            length = value.trim().parse().unwrap_or(0);
                        }
                    }
                    let body_start = pos + 4;
                    while buffer.len() < body_start + length {
                        match stream.read(&mut temp) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => buffer.extend_from_slice(&temp[..n]),
                        }
                    }
                    let body = buffer
                        .get(body_start..body_start + length)
                        .unwrap_or(&[])
                        .to_vec();
                    let recorded = record_request(&header, &body);
                    let (status, content_type, payload) = answer(&recorded.path, &recorded.api_key);
                    fake.records.lock().unwrap().push(recorded);
                    thread::sleep(Duration::from_millis((std::process::id() as u64 % 7) + 1));
                    // A 3xx points at /collect on the same fake, which answers
                    // normally, so a client that follows redirects is caught on
                    // the first hop rather than looping.
                    let location = if (300..400).contains(&status) {
                        "Location: /collect\r\n"
                    } else {
                        ""
                    };
                    let response = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        payload.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(payload.as_bytes());
                    break;
                }
            }
        }
    }
}

fn start_fake() -> (u16, Arc<Fake>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let fake = Arc::new(Fake {
        records: Mutex::new(Vec::new()),
    });
    let shared = fake.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let shared = shared.clone();
            thread::spawn(move || handle(stream, shared));
        }
    });
    (port, fake)
}

fn client(port: u16, key: &str) -> ZeroBounce {
    let mut zb = ZeroBounce::new("placeholder");
    zb.api_key = key.to_string();
    zb.base_url = format!("http://127.0.0.1:{port}/api/v2");
    zb.url_provider.url = format!("http://127.0.0.1:{port}/api/v2");
    zb.url_provider.bulk_url = format!("http://127.0.0.1:{port}/bulk/v2");
    // These default to the live hosts, so they must be pointed at the fake too or
    // the suite would reach the real API.
    zb.url_provider.list_evaluator_url = format!("http://127.0.0.1:{port}/bulk/v2/listevaluator");
    zb.url_provider.inbox_placement_url = format!("http://127.0.0.1:{port}/api/ipt");
    zb.client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    zb
}

fn only(fake: &Fake) -> Recorded {
    let records = fake.take();
    assert_eq!(records.len(), 1, "requests");
    records.into_iter().next().unwrap()
}

fn assert_short(err: &impl std::fmt::Display) {
    let message = err.to_string();
    assert!(message.len() < 500, "{message}");
    for forbidden in [
        "<html",
        "<!DOCTYPE",
        "<p>blocked</p>",
        "Error: Error:",
        "expected",
        "serde",
    ] {
        assert!(!message.contains(forbidden), "{message}");
    }
}

#[test]
fn version_constant_matches_package() {
    assert_eq!(
        ApiBaseUrl::Default.as_str(),
        "https://api.zerobounce.net/v2/"
    );
    assert_eq!(BULK_URI, "https://bulkapi.zerobounce.net/v2");
    assert_eq!(env!("CARGO_PKG_VERSION"), "4.0.0");
}

#[test]
fn credits_validate_and_batch_use_the_main_api() {
    let (port, fake) = start_fake();
    let zb = client(port, "credits-key");
    assert_eq!(zb.get_credits().unwrap(), 7);
    let request = only(&fake);
    assert_eq!(request.method, "GET");
    assert!(
        request.path.ends_with("/api/v2/getcredits"),
        "{}",
        request.path
    );
    assert_eq!(request.api_key, "credits-key");

    let options = ZBValidateOptions {
        activity_data: Some(false),
        verify_plus: Some(true),
        ..Default::default()
    };
    let response = zb
        .validate_email_with_options("person@example.com", "", &options)
        .unwrap();
    assert_eq!(response.address, "credits-key");
    let request = only(&fake);
    assert!(request.path.ends_with("/validate"), "{}", request.path);
    assert_eq!(
        request.fields.get("email").map(String::as_str),
        Some("person@example.com")
    );
    assert_eq!(
        request.fields.get("activity_data").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("verify_plus").map(String::as_str),
        Some("true")
    );
    assert!(!request.fields.contains_key("timeout"));
    assert!(!request.fields.contains_key("ip_address"));

    let posted = ZBValidateOptions {
        use_post: true,
        timeout: Some(15),
        ..Default::default()
    };
    zb.validate_email_with_options("person@example.com", "203.0.113.5", &posted)
        .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.fields.get("timeout").map(String::as_str),
        Some("15")
    );
    assert_eq!(
        request.fields.get("ip_address").map(String::as_str),
        Some("203.0.113.5")
    );
    assert!(!request.fields.contains_key("activity_data"));

    let batch = zb
        .batch_validate(vec![
            ("a@example.com".into(), String::new()),
            ("b@example.com".into(), "203.0.113.9".into()),
        ])
        .unwrap();
    assert_eq!(batch.email_batch[0].address, "credits-key");
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(
        request.path.contains("/api/v2/validatebatch"),
        "{}",
        request.path
    );
    assert!(request
        .fields
        .get("email_batch")
        .unwrap()
        .contains("a@example.com"));
    assert!(!request.fields.contains_key("timeout"));

    let batch_options = ZBValidateOptions {
        timeout: Some(30),
        activity_data: Some(false),
        verify_plus: Some(true),
        ..Default::default()
    };
    zb.batch_validate_with_options(
        vec![("a@example.com".into(), String::new())],
        &batch_options,
    )
    .unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("timeout").map(String::as_str),
        Some("30")
    );
    assert_eq!(
        request.fields.get("activity_data").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("verify_plus").map(String::as_str),
        Some("true")
    );
}

#[test]
fn files_scoring_and_priority3() {
    let (port, fake) = start_fake();
    let zb = client(port, "file-key");
    let file = ZBFile::from_content(b"email\n".to_vec())
        .set_email_address_column(0)
        .set_remove_duplicate(false)
        .set_has_header_row(true)
        .set_allow_phase_2(Some(false))
        .set_return_url(Some("https://example.com/hook"));
    let sent = zb.bulk_validation_file_submit(&file).unwrap();
    assert_eq!(sent.file_id.as_deref(), Some("file-key"));
    let request = only(&fake);
    assert!(
        request.path.ends_with("/bulk/v2/sendfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request
            .fields
            .get("email_address_column")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        request.fields.get("remove_duplicate").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("allow_phase_2").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("file").map(String::as_str),
        Some("<file>")
    );
    assert!(!request.fields.contains_key("first_name_column"));

    let scoring = file
        .set_first_name_column(Some(2))
        .set_allow_phase_2(Some(true));
    zb.ai_scoring_file_submit(&scoring).unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/scoring/sendfile"),
        "{}",
        request.path
    );
    assert!(!request.fields.contains_key("allow_phase_2"));
    assert!(!request.fields.contains_key("first_name_column"));

    assert_eq!(
        zb.bulk_validation_file_status_check("file-1")
            .unwrap()
            .file_id,
        "file-key"
    );
    assert!(only(&fake).path.ends_with("/filestatus"));
    zb.ai_scoring_file_status_check("file-2").unwrap();
    assert!(only(&fake).path.contains("/scoring/filestatus"));
    zb.bulk_validation_result_delete("file-1").unwrap();
    assert!(only(&fake).path.ends_with("/deletefile"));
    zb.ai_scoring_result_delete("file-2").unwrap();
    assert!(only(&fake).path.contains("/scoring/deletefile"));

    let options = ZBGetFileOptions {
        download_type: Some(zb_download_type::PHASE_2.into()),
        activity_data: Some(true),
    };
    match zb
        .bulk_validation_result_fetch_with_options("file-9", &options)
        .unwrap()
    {
        ZBBulkResponse::Content(bytes) => {
            assert!(String::from_utf8_lossy(&bytes).contains("file-key@example.com"))
        }
        other => panic!("{other:?}"),
    }
    let request = only(&fake);
    assert!(request.path.ends_with("/getfile"));
    assert_eq!(
        request.fields.get("download_type").map(String::as_str),
        Some("phase_2")
    );
    assert_eq!(
        request.fields.get("activity_data").map(String::as_str),
        Some("true")
    );
    let scoring_options = ZBGetFileOptions {
        download_type: Some(zb_download_type::PHASE_1.into()),
        activity_data: Some(true),
    };
    zb.ai_scoring_result_fetch_with_options("file-8", &scoring_options)
        .unwrap();
    let request = only(&fake);
    assert!(request.path.contains("/scoring/getfile"));
    assert!(!request.fields.contains_key("activity_data"));

    let start = NaiveDate::from_ymd_opt(2018, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2018, 1, 31).unwrap();
    assert_eq!(zb.get_api_usage(start, end).unwrap().total, 1);
    let request = only(&fake);
    assert!(request.path.ends_with("/getapiusage"));
    assert_eq!(
        request.fields.get("start_date").map(String::as_str),
        Some("2018-01-01")
    );
    assert!(zb.get_activity_data("person@example.com").unwrap().found);
    assert!(only(&fake).path.ends_with("/activity"));
    assert_eq!(
        zb.score_email("person@example.com").unwrap().email,
        "file-key"
    );
    assert!(only(&fake).path.ends_with("/scoring"));
    zb.score_email_post("person@example.com").unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(request.path.ends_with("/scoring"));

    let found = zb
        .find_email_v2()
        .domain("example.com")
        .first_name("Ada")
        .middle_name("M")
        .last_name("Lovelace")
        .call()
        .unwrap();
    assert_eq!(found.email, "file-key");
    let request = only(&fake);
    assert!(request.path.ends_with("/guessformat"), "{}", request.path);
    assert_eq!(
        request.fields.get("domain").map(String::as_str),
        Some("example.com")
    );
    assert!(!request.fields.contains_key("company_name"));
    zb.find_email_v2()
        .company_name("ZeroBounce")
        .first_name("Ada")
        .call()
        .unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("company_name").map(String::as_str),
        Some("ZeroBounce")
    );
    assert!(!request.fields.contains_key("domain"));
    assert!(!request.fields.contains_key("last_name"));
    // first_name is optional on /guessformat; unset, it is left off the query.
    zb.find_email_v2()
        .domain("example.com")
        .last_name("Doe")
        .call()
        .unwrap();
    let request = only(&fake);
    assert!(!request.fields.contains_key("first_name"));
    assert_eq!(
        request.fields.get("last_name").map(String::as_str),
        Some("Doe")
    );
    assert_eq!(
        zb.domain_search_v2()
            .domain("example.com")
            .call()
            .unwrap()
            .domain,
        "file-key"
    );
    assert!(!only(&fake).fields.contains_key("company_name"));
    zb.domain_search_v2()
        .company_name("ZeroBounce")
        .call()
        .unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("company_name").map(String::as_str),
        Some("ZeroBounce")
    );
    assert!(!request.fields.contains_key("domain"));

    zb.send_email_finder_file(
        1,
        &ZBEmailFinderFileOptions {
            first_name_column: Some(2),
            ..Default::default()
        },
        "finder.csv",
        &b"example.com\n"[..],
    )
    .unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/email-finder/sendfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("domain_column").map(String::as_str),
        Some("1")
    );
    zb.email_finder_file_status("fid").unwrap();
    assert!(only(&fake).path.ends_with("/email-finder/filestatus"));
    zb.send_domain_search_file(2, None, "domains.csv", &b"example.com\n"[..])
        .unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/domain-search/sendfile"));
    assert_eq!(
        request.fields.get("domain_column").map(String::as_str),
        Some("2")
    );
    zb.domain_search_file_status("fid").unwrap();
    assert!(only(&fake).path.ends_with("/domain-search/filestatus"));
    zb.send_list_evaluator_file(1, "list.csv", &b"a@example.com\n"[..])
        .unwrap();
    assert!(only(&fake).path.contains("/listevaluator/"));
    zb.get_list_evaluator_result("fid").unwrap();
    assert!(only(&fake).path.contains("/listevaluator/fid/"));
    // LIVE: send-evaluated-file takes the evaluator's file_id as a form field,
    // not a file upload.
    zb.send_evaluated_file("eval-file-1").unwrap();
    let request = only(&fake);
    assert!(request.path.contains("/send-evaluated-file/"));
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.fields.get("file_id").map(String::as_str),
        Some("eval-file-1")
    );

    zb.add_filter("exclude", "domain", "example.com").unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(
        request.path.contains("/api/v2/filters/add"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("rule").map(String::as_str),
        Some("exclude")
    );
    zb.list_filters(false).unwrap();
    assert_eq!(only(&fake).method, "GET");
    zb.list_filters(true).unwrap();
    assert_eq!(only(&fake).method, "POST");
    zb.delete_filter("exclude", "domain", "example.com")
        .unwrap();
    assert!(only(&fake).path.ends_with("/filters/delete"));
    zb.validate_s3_file("inbox.csv", true, None, "https://example.com/done")
        .unwrap();
    assert!(only(&fake).path.ends_with("/validateS3file"));
    zb.score_s3_file("inbox.csv", true, None, "https://example.com/done")
        .unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/scoreS3file"));
    assert!(!request.fields.contains_key("email_address_column"));
    zb.export_s3_file("fid", Some("bucket-out")).unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("export_bucket").map(String::as_str),
        Some("bucket-out")
    );
    let files = zb.list_s3_files(None, None).unwrap();
    assert_eq!(files[0].name, "file-key");
    let request = only(&fake);
    assert!(request.path.ends_with("/listS3files"));
    assert!(!request.fields.contains_key("bucket"));
    zb.list_s3_files(Some("inbox"), Some("csv")).unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("bucket").map(String::as_str),
        Some("inbox")
    );
    assert_eq!(
        request.fields.get("search").map(String::as_str),
        Some("csv")
    );
}

#[test]
fn bulk_finder_and_domain_search_files_send_and_omit_the_documented_fields() {
    let (port, fake) = start_fake();
    let zb = client(port, "finder-key");

    zb.send_email_finder_file(
        3,
        &ZBEmailFinderFileOptions {
            first_name_column: Some(1),
            full_name_column: Some(5),
            middle_name_column: Some(2),
            last_name_column: Some(4),
            has_header_row: Some(true),
        },
        "leads.csv",
        &b"example.com\n"[..],
    )
    .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(
        request.path.ends_with("/email-finder/sendfile"),
        "{}",
        request.path
    );
    for (field, value) in [
        ("domain_column", "3"),
        ("first_name_column", "1"),
        ("full_name_column", "5"),
        ("middle_name_column", "2"),
        ("last_name_column", "4"),
        // Booleans go on the wire as the strings "true"/"false".
        ("has_header_row", "true"),
    ] {
        assert_eq!(request.fields.get(field).map(String::as_str), Some(value));
    }

    // Unset columns stay off the request entirely. Some(0) is still sent.
    zb.send_email_finder_file(
        1,
        &ZBEmailFinderFileOptions {
            full_name_column: Some(0),
            ..Default::default()
        },
        "leads.csv",
        &b"example.com\n"[..],
    )
    .unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("full_name_column").map(String::as_str),
        Some("0")
    );
    for field in [
        "first_name_column",
        "middle_name_column",
        "last_name_column",
        "has_header_row",
    ] {
        assert!(!request.fields.contains_key(field), "{field}");
    }

    zb.send_domain_search_file(2, Some(false), "domains.csv", &b"example.com\n"[..])
        .unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/domain-search/sendfile"));
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("false")
    );
    zb.send_domain_search_file(2, None, "domains.csv", &b"example.com\n"[..])
        .unwrap();
    assert!(!only(&fake).fields.contains_key("has_header_row"));
}

#[test]
fn bulk_finder_and_domain_search_getfile_and_deletefile_use_the_bulk_root() {
    let (port, fake) = start_fake();
    let zb = client(port, "finder-key");

    let ZBBulkResponse::Content(content) = zb.email_finder_result_fetch("fid-1").unwrap() else {
        panic!("expected Content");
    };
    assert_eq!(content.as_ref(), b"email\nfinder-key@example.com\n");
    let request = only(&fake);
    assert_eq!(request.method, "GET");
    // These sit on the bulk host without the /v2 the other bulk APIs use.
    assert!(
        request.path.ends_with("/bulk/email-finder/getfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("file_id").map(String::as_str),
        Some("fid-1")
    );
    assert_eq!(request.api_key, "finder-key");
    // getfile for these two takes no download_type or activity_data.
    assert!(!request.fields.contains_key("activity_data"));
    assert!(!request.fields.contains_key("download_type"));

    zb.domain_search_result_fetch("fid-2").unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/bulk/domain-search/getfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("file_id").map(String::as_str),
        Some("fid-2")
    );

    let feedback = zb.email_finder_result_delete("fid-3").unwrap();
    assert!(feedback.success);
    let request = only(&fake);
    assert_eq!(request.method, "GET");
    assert!(
        request.path.ends_with("/bulk/email-finder/deletefile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("file_id").map(String::as_str),
        Some("fid-3")
    );

    zb.domain_search_result_delete("fid-4").unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/bulk/domain-search/deletefile"),
        "{}",
        request.path
    );

    // An HTML access-denied page is a short error, not file bytes.
    let err = client(port, "html-200-finder")
        .email_finder_result_fetch("fid-1")
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Access denied"));
}

#[test]
fn s3_and_sftp_file_jobs_send_the_documented_fields_only() {
    let (port, fake) = start_fake();
    let zb = client(port, "user");

    zb.email_finder_s3_file("lists/leads.csv", true, None, "https://example.com/done")
        .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(
        request.path.ends_with("/emailFinderS3file"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("file_name").map(String::as_str),
        Some("lists/leads.csv")
    );
    // Booleans go on the wire as the strings "true"/"false".
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("true")
    );
    // Unset options stay off the request entirely.
    assert!(!request.fields.contains_key("remove_duplicates"));

    zb.domain_search_s3_file(
        "lists/leads.csv",
        false,
        Some(true),
        "https://example.com/done",
    )
    .unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/domainSearchS3file"));
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("remove_duplicates").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        request.fields.get("return_url").map(String::as_str),
        Some("https://example.com/done")
    );

    // 31 and 32 go through the same helper as their SFTP twins, so they send
    // has_header_row, remove_duplicates and return_url rather than file_name alone.
    zb.validate_s3_file("lists/leads.csv", false, Some(false), "https://example.com/done")
        .unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/validateS3file"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("remove_duplicates").map(String::as_str),
        Some("false")
    );
    assert_eq!(
        request.fields.get("return_url").map(String::as_str),
        Some("https://example.com/done")
    );

    zb.score_s3_file("lists/leads.csv", true, None, "https://example.com/done")
        .unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/scoreS3file"), "{}", request.path);
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("true")
    );
    assert!(!request.fields.contains_key("remove_duplicates"));
    assert_eq!(
        request.fields.get("return_url").map(String::as_str),
        Some("https://example.com/done")
    );

    // export_bucket is optional: without it the account's export settings apply.
    zb.export_s3_file("fid", None).unwrap();
    let request = only(&fake);
    assert!(request.path.ends_with("/exportS3file"), "{}", request.path);
    assert!(!request.fields.contains_key("export_bucket"));
    assert_eq!(request.fields.get("file_id").map(String::as_str), Some("fid"));

    // LIVE: file_name is required here too, although the docs list it as
    // optional for this one endpoint.
    zb.validate_sftp_file("my_file.txt", true, None, "https://example.com/done")
        .unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/validateSFTPfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("file_name").map(String::as_str),
        Some("my_file.txt")
    );
    assert_eq!(
        request.fields.get("has_header_row").map(String::as_str),
        Some("true")
    );

    zb.score_sftp_file("my_file.txt", true, None, "https://example.com/done")
        .unwrap();
    assert!(only(&fake).path.ends_with("/scoreSFTPfile"));
    zb.email_finder_sftp_file("my_file.txt", true, None, "https://example.com/done")
        .unwrap();
    assert!(only(&fake).path.ends_with("/emailFinderSFTPfile"));
    zb.domain_search_sftp_file("my_file.txt", true, None, "https://example.com/done")
        .unwrap();
    assert!(only(&fake).path.ends_with("/domainSearchSFTPfile"));

    // exportSFTPfile takes export_path, not the export_bucket exportS3file uses.
    zb.export_sftp_file("fid", Some("/exports")).unwrap();
    let request = only(&fake);
    assert!(
        request.path.ends_with("/exportSFTPfile"),
        "{}",
        request.path
    );
    assert_eq!(
        request.fields.get("export_path").map(String::as_str),
        Some("/exports")
    );
    assert!(!request.fields.contains_key("export_bucket"));

    zb.export_sftp_file("fid", None).unwrap();
    assert!(!only(&fake).fields.contains_key("export_path"));
}

#[test]
fn s3_and_sftp_file_jobs_reject_invalid_arguments_before_a_request() {
    let (port, fake) = start_fake();
    let zb = client(port, "user");

    let done = "https://example.com/done";
    for error in [
        zb.email_finder_s3_file("  ", true, None, done).unwrap_err(),
        zb.domain_search_s3_file("", true, None, done).unwrap_err(),
        zb.score_sftp_file(" ", true, None, done).unwrap_err(),
        zb.email_finder_sftp_file("", true, None, done).unwrap_err(),
        zb.domain_search_sftp_file("", true, None, done).unwrap_err(),
        zb.validate_sftp_file(" ", true, None, done).unwrap_err(),
        zb.validate_s3_file("", true, None, done).unwrap_err(),
        zb.score_s3_file("  ", true, None, done).unwrap_err(),
    ] {
        assert!(error.to_string().contains("file_name"), "{error}");
    }

    // LIVE: return_url is required on every one of these jobs.
    for error in [
        zb.email_finder_s3_file("f.csv", true, None, "").unwrap_err(),
        zb.domain_search_s3_file("f.csv", true, None, " ").unwrap_err(),
        zb.validate_sftp_file("f.csv", true, None, "").unwrap_err(),
        zb.score_sftp_file("f.csv", true, None, "").unwrap_err(),
        zb.email_finder_sftp_file("f.csv", true, None, "").unwrap_err(),
        zb.domain_search_sftp_file("f.csv", true, None, "").unwrap_err(),
        // LIVE: the S3 validate and score jobs reject a job with no return_url too.
        zb.validate_s3_file("f.csv", true, None, "").unwrap_err(),
        zb.score_s3_file("f.csv", true, None, " ").unwrap_err(),
    ] {
        assert!(error.to_string().contains("return_url"), "{error}");
    }

    assert!(zb
        .export_sftp_file("  ", None)
        .unwrap_err()
        .to_string()
        .contains("file_id"));
    assert!(zb
        .export_sftp_file("fid", Some(" "))
        .unwrap_err()
        .to_string()
        .contains("export_path"));
    assert!(zb
        .export_s3_file("  ", None)
        .unwrap_err()
        .to_string()
        .contains("file_id"));
    assert!(zb
        .export_s3_file("fid", Some(" "))
        .unwrap_err()
        .to_string()
        .contains("export_bucket"));

    assert_eq!(fake.len(), 0);
}

#[test]
fn invalid_arguments_are_rejected_before_a_request() {
    let (port, fake) = start_fake();
    let empty = client(port, "");
    assert!(empty
        .get_credits()
        .unwrap_err()
        .to_string()
        .contains("api_key"));
    assert!(empty
        .find_email_v2()
        .domain("example.com")
        .first_name("Ada")
        .call()
        .unwrap_err()
        .to_string()
        .contains("api_key"));
    assert!(empty
        .domain_search_v2()
        .company_name("ZeroBounce")
        .call()
        .unwrap_err()
        .to_string()
        .contains("api_key"));
    assert!(empty
        .list_filters(false)
        .unwrap_err()
        .to_string()
        .contains("api_key"));
    assert!(empty
        .send_domain_search_file(1, None, "d.csv", &b"a\n"[..])
        .unwrap_err()
        .to_string()
        .contains("api_key"));
    assert_eq!(fake.len(), 0);

    let zb = client(port, "keyed");
    let bad = ZBValidateOptions {
        timeout: Some(2),
        ..Default::default()
    };
    assert!(zb
        .validate_email_with_options("a@example.com", "", &bad)
        .unwrap_err()
        .to_string()
        .contains("timeout"));
    let batch = ZBValidateOptions {
        timeout: Some(9),
        ..Default::default()
    };
    assert!(zb
        .batch_validate_with_options(vec![("a@example.com".into(), String::new())], &batch)
        .unwrap_err()
        .to_string()
        .contains("timeout"));
    assert!(zb
        .send_email_finder_file(
            0,
            &ZBEmailFinderFileOptions {
                first_name_column: Some(2),
                ..Default::default()
            },
            "f.csv",
            &b"a\n"[..]
        )
        .unwrap_err()
        .to_string()
        .contains("domain_column"));
    // A finder file with no name column at all is rejected before a request.
    assert!(zb
        .send_email_finder_file(1, &ZBEmailFinderFileOptions::default(), "f.csv", &b"a\n"[..])
        .unwrap_err()
        .to_string()
        .contains("full_name_column"));
    assert!(zb
        .validate_s3_file("  ", true, None, "https://example.com/done")
        .unwrap_err()
        .to_string()
        .contains("file_name"));
    assert_eq!(fake.len(), 0);
}

#[test]
fn error_bodies_stay_short_and_batch_account_errors_fail() {
    let (port, _) = start_fake();
    let err = client(port, "html-403-user").get_credits().unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Access denied"));
    assert!(html_page().len() > 100_000);

    let err = client(port, "html-200-user")
        .validate_email("a@example.com")
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Access denied"));

    let err = client(port, "html-200-file")
        .bulk_validation_result_fetch("fid")
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Access denied"));

    let err = client(port, "cf-json-user").get_credits().unwrap_err();
    assert_short(&err);
    assert!(!err.to_string().contains("developers.cloudflare.com"));
    assert!(err.to_string().contains("Access denied"));

    let err = client(port, "empty-user")
        .validate_email("a@example.com")
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Empty response"));

    let err = client(port, "malformed-user")
        .validate_email("a@example.com")
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("not valid JSON"));
    assert!(!err.to_string().contains("<<<not-json>>>"));

    let err = client(port, "double-error-user")
        .get_activity_data("a@example.com")
        .unwrap_err();
    assert_short(&err);
    assert_eq!(err.to_string(), "api_key is invalid");

    let err = client(port, "batch-account-user")
        .batch_validate(vec![("a@example.com".into(), String::new())])
        .unwrap_err();
    assert_short(&err);
    assert!(err.to_string().contains("Invalid API Key"));
    assert!(err.to_string().contains("ran out of credits"));

    let partial = client(port, "batch-partial-user")
        .batch_validate(vec![
            ("good@example.com".into(), String::new()),
            ("bad@".into(), String::new()),
        ])
        .unwrap();
    assert_eq!(partial.email_batch.len(), 1);
    assert_eq!(partial.email_batch[0].address, "good@example.com");
    assert_eq!(partial.errors.len(), 1);
    assert_eq!(partial.errors[0].email_address, "bad@");
}

#[test]
fn concurrent_clients_do_not_share_keys() {
    let (port, fake) = start_fake();
    let mut handles = Vec::new();
    for i in 0..12 {
        handles.push(thread::spawn(move || {
            let key = format!("conc-{i}");
            let response = client(port, &key)
                .validate_email("person@example.com")
                .unwrap();
            (key, response.address)
        }));
    }
    let mut echoes = Vec::new();
    for handle in handles {
        echoes.push(handle.join().unwrap());
    }
    thread::sleep(Duration::from_millis(50));
    let records = fake.take();
    assert_eq!(records.len(), 12);
    for (key, address) in echoes {
        assert_eq!(address, key);
        assert_eq!(
            records
                .iter()
                .filter(|record| record.api_key == key)
                .count(),
            1
        );
    }
}

#[test]
fn validate_sends_the_new_options_only_when_set() {
    let (port, fake) = start_fake();
    let zb = client(port, "parity-key");

    zb.validate_email("a@example.com").unwrap();
    let request = only(&fake);
    assert!(!request.fields.contains_key("domain_info"));
    assert!(!request.fields.contains_key("greylist_processing"));
    assert!(!request.fields.contains_key("callback_url"));

    let options = ZBValidateOptions {
        domain_info: Some(true),
        greylist_processing: Some(false),
        ..Default::default()
    };
    zb.validate_email_with_options("a@example.com", "", &options)
        .unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("domain_info").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        request.fields.get("greylist_processing").map(String::as_str),
        Some("false")
    );

    let options = ZBValidateOptions {
        greylist_processing: Some(true),
        callback_url: Some("https://example.com/cb".to_string()),
        use_post: true,
        ..Default::default()
    };
    zb.validate_email_with_options("a@example.com", "", &options)
        .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.fields.get("callback_url").map(String::as_str),
        Some("https://example.com/cb")
    );
}

#[test]
fn callback_url_without_greylist_processing_sends_no_request() {
    let (port, fake) = start_fake();
    let zb = client(port, "parity-key");

    let options = ZBValidateOptions {
        callback_url: Some("https://example.com/cb".to_string()),
        ..Default::default()
    };
    let error = zb
        .validate_email_with_options("a@example.com", "", &options)
        .unwrap_err();
    assert!(error.to_string().contains("greylist_processing"));

    let blank = ZBValidateOptions {
        greylist_processing: Some(true),
        callback_url: Some("   ".to_string()),
        ..Default::default()
    };
    let error = zb
        .validate_email_with_options("a@example.com", "", &blank)
        .unwrap_err();
    assert!(error.to_string().contains("callback_url"));
    assert_eq!(fake.len(), 0);
}

#[test]
fn inbox_placement_uses_its_own_host_and_camel_case_json() {
    let (port, fake) = start_fake();
    let zb = client(port, "ipt-key");

    zb.get_inbox_placement_providers().unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "GET");
    assert!(request.path.ends_with("/api/ipt/providers"));
    assert_eq!(request.api_key, "ipt-key");

    zb.get_inbox_placement_test_types().unwrap();
    assert!(only(&fake).path.ends_with("/api/ipt/test-types"));

    zb.create_inbox_placement_test("basic", &Default::default())
        .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "POST");
    assert!(request.path.ends_with("/api/ipt/test"));
    assert_eq!(request.fields.get("name").map(String::as_str), Some("basic"));
    assert!(!request.fields.contains_key("testType"));
    assert!(!request.fields.contains_key("senderEmail"));

    let options = CreateInboxPlacementTestOptions {
        test_type: Some(1),
        sender_email: Some("from@example.com".to_string()),
        webhook_url: Some("https://example.com/hook".to_string()),
        ..Default::default()
    };
    zb.create_inbox_placement_test("full", &options).unwrap();
    let request = only(&fake);
    assert_eq!(request.fields.get("testType").map(String::as_str), Some("1"));
    assert_eq!(
        request.fields.get("senderEmail").map(String::as_str),
        Some("from@example.com")
    );
    assert_eq!(
        request.fields.get("webhookUrl").map(String::as_str),
        Some("https://example.com/hook")
    );

    zb.get_inbox_placement_test("a b/c").unwrap();
    assert!(only(&fake).path.ends_with("/api/ipt/test/a%20b%2Fc"));

    zb.delete_inbox_placement_test("t1").unwrap();
    assert_eq!(only(&fake).method, "DELETE");

    zb.set_inbox_placement_webhook("https://example.com/hook")
        .unwrap();
    let request = only(&fake);
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.fields.get("webhookUrl").map(String::as_str),
        Some("https://example.com/hook")
    );
}

#[test]
fn inbox_placement_list_sends_only_the_filters_that_are_set() {
    let (port, fake) = start_fake();
    let zb = client(port, "ipt-key");

    zb.list_inbox_placement_tests(&Default::default()).unwrap();
    let request = only(&fake);
    assert!(!request.fields.contains_key("status"));
    assert!(!request.fields.contains_key("page"));

    let options = ListInboxPlacementTestsOptions {
        status: Some("finished".to_string()),
        created_after: Some("2026-01-01".to_string()),
        page: Some(2),
        page_size: Some(50),
        ..Default::default()
    };
    zb.list_inbox_placement_tests(&options).unwrap();
    let request = only(&fake);
    assert_eq!(
        request.fields.get("status").map(String::as_str),
        Some("finished")
    );
    assert_eq!(
        request.fields.get("createdAfter").map(String::as_str),
        Some("2026-01-01")
    );
    assert_eq!(request.fields.get("page").map(String::as_str), Some("2"));
    assert_eq!(
        request.fields.get("page_size").map(String::as_str),
        Some("50")
    );
}

#[test]
fn inbox_placement_rejects_invalid_arguments_before_a_request() {
    let (port, fake) = start_fake();
    let zb = client(port, "ipt-key");

    assert!(zb
        .create_inbox_placement_test("", &Default::default())
        .is_err());
    assert!(zb
        .create_inbox_placement_test(&"x".repeat(65), &Default::default())
        .is_err());
    assert!(zb
        .create_inbox_placement_test(
            "t",
            &CreateInboxPlacementTestOptions {
                test_type: Some(4),
                ..Default::default()
            }
        )
        .is_err());
    assert!(zb
        .create_inbox_placement_test(
            "t",
            &CreateInboxPlacementTestOptions {
                test_type: Some(1),
                ..Default::default()
            }
        )
        .is_err());
    assert!(zb
        .create_inbox_placement_test(
            "t",
            &CreateInboxPlacementTestOptions {
                providers: Some(vec![InboxPlacementProviderSelection {
                    id: 1,
                    max_seeds: Some(0),
                }]),
                ..Default::default()
            }
        )
        .is_err());
    assert!(zb
        .list_inbox_placement_tests(&ListInboxPlacementTestsOptions {
            status: Some("queued".to_string()),
            ..Default::default()
        })
        .is_err());
    assert!(zb
        .list_inbox_placement_tests(&ListInboxPlacementTestsOptions {
            page_size: Some(101),
            ..Default::default()
        })
        .is_err());
    assert!(zb.get_inbox_placement_test("  ").is_err());
    assert!(zb.delete_inbox_placement_test("").is_err());
    assert!(zb.set_inbox_placement_webhook("  ").is_err());
    assert_eq!(fake.len(), 0);
}

#[test]
fn inbox_placement_detail_error_becomes_the_message() {
    let (port, _fake) = start_fake();
    let zb = client(port, "ipt-detail-key");

    let error = zb.get_inbox_placement_providers().unwrap_err();
    assert!(error.to_string().contains("Inbox placement test not found"));
    assert_short(&error);
}

#[test]
fn list_evaluator_uses_its_own_base_url_and_encodes_file_id() {
    let (port, fake) = start_fake();
    let mut zb = client(port, "le-key");

    zb.get_list_evaluator_result("a b/c").unwrap();
    assert!(only(&fake).path.ends_with("/listevaluator/a%20b%2Fc/"));

    zb.url_provider.list_evaluator_url = format!("http://127.0.0.1:{port}/custom/listevaluator");
    zb.get_list_evaluator_result("fid").unwrap();
    assert!(only(&fake).path.ends_with("/custom/listevaluator/fid/"));
}

fn api_error_message<T: std::fmt::Debug>(name: &str, result: Result<T, ZBError>) -> String {
    match result {
        Err(ZBError::ApiError { message, .. }) => message,
        other => panic!("{name}: expected an ApiError, got {other:?}"),
    }
}

/// An invalid or drained key answers with an HTTP 200 and an error-only body.
/// Every call must fail with the API's own message instead of returning an
/// empty result or a parse error.
#[test]
fn a_dead_key_is_an_api_error_on_every_call() {
    let (port, _fake) = start_fake();
    let zb = client(port, "dead-key-user");
    let drained = "Invalid API key or your account ran out of credits";
    assert_eq!(api_error_message("validate", zb.validate_email("a@example.com")), drained);
    assert_eq!(api_error_message("activity", zb.get_activity_data("a@example.com")), drained);
    assert_eq!(api_error_message("api usage", zb.get_api_usage_overall()), "Invalid API key");
    assert_eq!(api_error_message("score", zb.score_email("a@example.com")), drained);
    assert_eq!(api_error_message("find_email_v2", zb.find_email_v2().first_name("john").domain("example.com").call()), drained);
    assert_eq!(api_error_message("domain_search_v2", zb.domain_search_v2().domain("example.com").call()), drained);
    #[allow(deprecated)]
    {
        assert_eq!(api_error_message("find_email", zb.find_email("example.com", "john", "", "")), drained);
        assert_eq!(api_error_message("domain_search", zb.domain_search("example.com")), drained);
    }
    assert_eq!(api_error_message("add_filter", zb.add_filter("allow", "email", "x@example.com")), drained);
    assert_eq!(api_error_message("delete_filter", zb.delete_filter("allow", "email", "x@example.com")), drained);
    assert_eq!(api_error_message("list_filters GET", zb.list_filters(false)), drained);
    assert_eq!(api_error_message("list_filters POST", zb.list_filters(true)), drained);
    // -1 is the documented balance of an invalid key, not an error.
    assert_eq!(zb.get_credits().unwrap(), -1);
}

/// The error-body checks must not turn a real result into a failure.
#[test]
fn a_bare_message_is_still_a_result_where_the_api_means_one() {
    let (port, _fake) = start_fake();
    let zb = client(port, "greylist-pending-user");
    let pending = zb.validate_email("a@example.com").expect("a pending greylist validate is a result");
    assert_eq!(
        pending.message.as_deref(),
        Some("Greylist validation still in progress. Please try again later.")
    );

    let zb = client(port, "filter-message-user");
    let added = zb.add_filter("allow", "email", "x@example.com").expect("a filter success is a result");
    assert_eq!(added["Message"], "Filter has been added.");
}

// -- https on every request path -------------------------------------------
//
// `url_provider` is public, so a caller can point any base URL at plain
// http://. Only the exact loopback hosts may use it; anything else must fail
// before a connection is opened. The client below resolves a non-loopback
// name to the local fake, so a request that slipped through would land there
// with the API key in cleartext -- and the fake would record it.

const PLAIN_HOST: &str = "plain-http.zb-test.invalid";

fn plaintext_client(port: u16, key: &str) -> ZeroBounce {
    let mut zb = client(port, key);
    let root = format!("http://{PLAIN_HOST}:{port}");
    zb.base_url = format!("{root}/api/v2");
    zb.url_provider.url = format!("{root}/api/v2");
    zb.url_provider.bulk_url = format!("{root}/bulk/v2");
    zb.url_provider.list_evaluator_url = format!("{root}/bulk/v2/listevaluator");
    zb.url_provider.inbox_placement_url = format!("{root}/api/ipt");
    zb.client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .resolve(
            PLAIN_HOST,
            std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        )
        .build()
        .unwrap();
    zb
}

fn assert_refused_plaintext<T: std::fmt::Debug>(name: &str, result: Result<T, ZBError>) {
    match result {
        Err(ZBError::ExplicitError(message)) => {
            assert!(message.contains("https://"), "{name}: {message}");
        }
        other => panic!("{name}: expected the https rule to refuse the request, got {other:?}"),
    }
}

/// The fake must have seen nothing: no request, so no key on the wire.
fn assert_nothing_sent(fake: &Fake, name: &str) {
    let leaked = fake.take();
    // The message says whether a key was on the wire, never what it was.
    assert!(
        leaked.is_empty(),
        "{name}: {} request(s) reached the server over plain http, the first to {} (key sent: {})",
        leaked.len(),
        leaked[0].path,
        !leaked[0].api_key.is_empty()
    );
}

/// Calls every public request method of `ZeroBounce` against the plain-http
/// client and returns the names it called, so the guard below can hold them
/// against the methods `src/` defines. Named after the `pub fn`; the v2
/// builders are swept through their `call()`.
#[allow(deprecated)] // find_email and domain_search are deprecated, not gone
fn sweep_every_request_method(zb: &ZeroBounce) -> Vec<&'static str> {
    let mut swept = Vec::new();
    macro_rules! sweep {
        ($name:literal, $call:expr) => {{
            assert_refused_plaintext($name, $call);
            swept.push($name);
        }};
    }
    let day = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let file = ZBFile::from_content(b"email\na@example.com\n".to_vec());
    let getfile_options = ZBGetFileOptions {
        download_type: Some(zb_download_type::COMBINED.to_string()),
        activity_data: Some(true),
    };
    let finder_options = ZBEmailFinderFileOptions {
        first_name_column: Some(2),
        ..Default::default()
    };
    let post = ZBValidateOptions {
        use_post: true,
        ..Default::default()
    };
    let done = "https://example.com/done";

    // Main API: account, scoring, validation, finder and domain search.
    sweep!("get_credits", zb.get_credits());
    sweep!("get_api_usage", zb.get_api_usage(day, day));
    sweep!("get_api_usage_overall", zb.get_api_usage_overall());
    sweep!("get_activity_data", zb.get_activity_data("a@example.com"));
    sweep!("score_email", zb.score_email("a@example.com"));
    sweep!("score_email_post", zb.score_email_post("a@example.com"));
    sweep!("validate_email", zb.validate_email("a@example.com"));
    sweep!("validate_email_and_ip", zb.validate_email_and_ip("a@example.com", "1.1.1.1"));
    sweep!(
        "validate_email_with_options",
        zb.validate_email_with_options("a@example.com", "", &ZBValidateOptions::default())
    );
    sweep!("validate_email_with_options", zb.validate_email_with_options("a@example.com", "", &post));
    sweep!("batch_validate", zb.batch_validate(vec![("a@example.com".into(), String::new())]));
    sweep!(
        "batch_validate_with_options",
        zb.batch_validate_with_options(vec![("a@example.com".into(), String::new())], &post)
    );
    sweep!("find_email", zb.find_email("example.com", "john", "", "doe"));
    sweep!("find_email_v2", zb.find_email_v2().first_name("john").domain("example.com").call());
    sweep!("domain_search", zb.domain_search("example.com"));
    sweep!("domain_search_v2", zb.domain_search_v2().domain("example.com").call());

    // Bulk validation and AI scoring files.
    sweep!("bulk_validation_file_submit", zb.bulk_validation_file_submit(&file));
    sweep!("bulk_validation_file_status_check", zb.bulk_validation_file_status_check("fid"));
    sweep!("bulk_validation_result_fetch", zb.bulk_validation_result_fetch("fid"));
    sweep!(
        "bulk_validation_result_fetch_with_options",
        zb.bulk_validation_result_fetch_with_options("fid", &getfile_options)
    );
    sweep!("bulk_validation_result_delete", zb.bulk_validation_result_delete("fid"));
    sweep!("ai_scoring_file_submit", zb.ai_scoring_file_submit(&file));
    sweep!("ai_scoring_file_status_check", zb.ai_scoring_file_status_check("fid"));
    sweep!("ai_scoring_result_fetch", zb.ai_scoring_result_fetch("fid"));
    sweep!(
        "ai_scoring_result_fetch_with_options",
        zb.ai_scoring_result_fetch_with_options("fid", &getfile_options)
    );
    sweep!("ai_scoring_result_delete", zb.ai_scoring_result_delete("fid"));

    // Bulk email finder, domain search and list evaluator (multipart uploads).
    sweep!(
        "send_email_finder_file",
        zb.send_email_finder_file(1, &finder_options, "leads.csv", &b"example.com,john\n"[..])
    );
    sweep!("email_finder_file_status", zb.email_finder_file_status("fid"));
    sweep!("email_finder_result_fetch", zb.email_finder_result_fetch("fid"));
    sweep!("email_finder_result_delete", zb.email_finder_result_delete("fid"));
    sweep!(
        "send_domain_search_file",
        zb.send_domain_search_file(1, Some(true), "domains.csv", &b"example.com\n"[..])
    );
    sweep!("domain_search_file_status", zb.domain_search_file_status("fid"));
    sweep!("domain_search_result_fetch", zb.domain_search_result_fetch("fid"));
    sweep!("domain_search_result_delete", zb.domain_search_result_delete("fid"));
    sweep!("send_evaluated_file", zb.send_evaluated_file("fid"));
    sweep!(
        "send_list_evaluator_file",
        zb.send_list_evaluator_file(1, "list.csv", &b"a@example.com\n"[..])
    );
    sweep!("get_list_evaluator_result", zb.get_list_evaluator_result("fid"));

    // Filters.
    sweep!("add_filter", zb.add_filter("allow", "email", "x@example.com"));
    sweep!("list_filters", zb.list_filters(false));
    sweep!("list_filters", zb.list_filters(true));
    sweep!("delete_filter", zb.delete_filter("allow", "email", "x@example.com"));

    // S3 and SFTP/FTP file jobs.
    sweep!("list_s3_files", zb.list_s3_files(None, None));
    sweep!("validate_s3_file", zb.validate_s3_file("f.csv", true, None, done));
    sweep!("score_s3_file", zb.score_s3_file("f.csv", true, None, done));
    sweep!("email_finder_s3_file", zb.email_finder_s3_file("f.csv", true, None, done));
    sweep!("domain_search_s3_file", zb.domain_search_s3_file("f.csv", true, None, done));
    sweep!("export_s3_file", zb.export_s3_file("fid", None));
    sweep!("validate_sftp_file", zb.validate_sftp_file("f.csv", true, None, done));
    sweep!("score_sftp_file", zb.score_sftp_file("f.csv", true, None, done));
    sweep!("email_finder_sftp_file", zb.email_finder_sftp_file("f.csv", true, None, done));
    sweep!("domain_search_sftp_file", zb.domain_search_sftp_file("f.csv", true, None, done));
    sweep!("export_sftp_file", zb.export_sftp_file("fid", None));

    // Inbox Placement Tester (Bearer key).
    sweep!("get_inbox_placement_providers", zb.get_inbox_placement_providers());
    sweep!("get_inbox_placement_test_types", zb.get_inbox_placement_test_types());
    sweep!(
        "create_inbox_placement_test",
        zb.create_inbox_placement_test("t", &CreateInboxPlacementTestOptions::default())
    );
    sweep!("get_inbox_placement_test", zb.get_inbox_placement_test("1"));
    sweep!(
        "list_inbox_placement_tests",
        zb.list_inbox_placement_tests(&ListInboxPlacementTestsOptions::default())
    );
    sweep!("delete_inbox_placement_test", zb.delete_inbox_placement_test("1"));
    sweep!("get_inbox_placement_webhook", zb.get_inbox_placement_webhook());
    sweep!("set_inbox_placement_webhook", zb.set_inbox_placement_webhook("https://example.com/hook"));

    swept
}

/// `pub fn`s of `ZeroBounce` that send no request.
const NOT_REQUEST_METHODS: [&str; 2] = ["new", "with_base_url"];

/// The name of every `pub fn` in an `impl ZeroBounce` block under `src/`,
/// read from the source, so a method added later is in this list without
/// anyone remembering to add it.
fn public_request_methods() -> BTreeSet<String> {
    fn rust_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    // Cargo sets CARGO_MANIFEST_DIR when it runs the test too; reading it at
    // run time keeps the guard on the tree being tested if the target
    // directory is shared between checkouts.
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string());
    let mut files = Vec::new();
    rust_files(&std::path::Path::new(&root).join("src"), &mut files);
    assert!(!files.is_empty(), "no Rust sources under {root}/src");

    let mut names = BTreeSet::new();
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        let mut in_impl = false;
        for line in source.lines() {
            // An inherent impl starts in column 0 and ends at the first "}"
            // in column 0. A trait impl (`impl X for ZeroBounce`) is skipped.
            let head = line.trim_end().trim_end_matches('{').trim_end();
            if line.starts_with("impl") && head.ends_with(" ZeroBounce") && !head.contains(" for ") {
                in_impl = true;
                continue;
            }
            if in_impl && line.starts_with('}') {
                in_impl = false;
                continue;
            }
            let item = line.trim_start();
            let signature = item
                .strip_prefix("pub fn ")
                .or_else(|| item.strip_prefix("pub async fn "));
            if let (true, Some(signature)) = (in_impl, signature) {
                let name: String = signature
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !NOT_REQUEST_METHODS.contains(&name.as_str()) {
                    names.insert(name);
                }
            }
        }
    }
    names
}

/// Every public request method refuses a plain-http URL before anything is
/// sent, and the guard fails when `src/` gains a request method the sweep
/// does not call (or the sweep names one that no longer exists).
#[test]
fn plain_http_is_refused_on_every_public_request_method() {
    let (port, fake) = start_fake();
    let zb = plaintext_client(port, "plain-sweep-key");
    let swept: BTreeSet<String> = sweep_every_request_method(&zb)
        .into_iter()
        .map(String::from)
        .collect();
    assert_nothing_sent(&fake, "plain-http sweep");

    let defined = public_request_methods();
    let missing: Vec<&String> = defined.difference(&swept).collect();
    let unknown: Vec<&String> = swept.difference(&defined).collect();
    assert!(
        missing.is_empty(),
        "public request methods the plain-http sweep does not call: {missing:?}"
    );
    assert!(
        unknown.is_empty(),
        "swept names with no `pub fn` in an `impl ZeroBounce` block: {unknown:?}"
    );
}

/// The plaintext client itself works: the same requests reach the fake once
/// the override is an allowed loopback URL. Without this, the tests above
/// could pass because the name never resolved rather than because the rule
/// held.
#[test]
fn the_plaintext_fixture_does_reach_the_fake() {
    let (port, fake) = start_fake();
    let mut zb = plaintext_client(port, "plain-control-key");
    zb.url_provider.inbox_placement_url = format!("http://127.0.0.1:{port}/api/ipt");
    zb.get_inbox_placement_providers().unwrap();
    assert_eq!(only(&fake).authorization, "Bearer plain-control-key");

    // And the resolve override is live: with the rule bypassed by hand, a raw
    // request to the plain-http name lands on the fake.
    zb.client
        .get(format!("http://{PLAIN_HOST}:{port}/api/ipt/providers"))
        .send()
        .unwrap();
    assert_eq!(fake.len(), 1);
}

// -- status_code() wherever a response arrived -------------------------------

#[test]
fn getfile_errors_carry_the_http_status() {
    let (port, _fake) = start_fake();

    let error = client(port, "html-403-file").bulk_validation_result_fetch("fid").unwrap_err();
    assert_eq!(error.status_code(), Some(403), "{error:?}");
    assert!(error.to_string().contains("Access denied"));
    assert_short(&error);

    let error = client(port, "cf-json-file").ai_scoring_result_fetch("fid").unwrap_err();
    assert_eq!(error.status_code(), Some(403), "{error:?}");
    assert!(error.to_string().contains("Access denied"));

    let error = client(port, "html-200-file").email_finder_result_fetch("fid").unwrap_err();
    assert_eq!(error.status_code(), Some(200), "{error:?}");
    assert!(error.to_string().contains("Access denied"));

    let error = client(port, "double-error-file").domain_search_result_fetch("fid").unwrap_err();
    assert_eq!(error.status_code(), Some(400), "{error:?}");
    assert_eq!(error.to_string(), "api_key is invalid");
}

#[test]
fn a_batch_account_level_error_carries_the_http_status() {
    let (port, _fake) = start_fake();
    let error = client(port, "batch-account-user")
        .batch_validate(vec![("a@example.com".into(), String::new())])
        .unwrap_err();
    // The API reports it with an HTTP 200.
    assert_eq!(error.status_code(), Some(200), "{error:?}");
    match error {
        ZBError::ApiError { message, .. } => {
            assert!(message.contains("Invalid API Key"), "{message}");
        }
        other => panic!("expected an ApiError, got {other:?}"),
    }
}

/// Errors raised before any request have no status.
#[test]
fn errors_without_a_response_have_no_status() {
    let error = ZeroBounce::new("").validate_email("a@example.com").unwrap_err();
    assert_eq!(error.status_code(), None);
    let error = ZeroBounce::with_base_url("key", "http://api.zerobounce.net/v2/").err().unwrap();
    assert_eq!(error.status_code(), None);
}

/// The built-in base URLs end in `/v2/` and every endpoint starts with `/`, so
/// requests went to `/v2//validate`. Built-in and custom URLs alike are now
/// joined with one slash.
#[test]
fn base_urls_are_joined_with_one_slash() {
    use zero_bounce::utility::{ENDPOINT_FILE_SEND, ENDPOINT_VALIDATE};
    let cases: Vec<(ZeroBounce, &str)> = vec![
        (ZeroBounce::new("k"), "https://api.zerobounce.net/v2/validate"),
        (ZeroBounce::with_base_url("k", ApiBaseUrl::Default).unwrap(), "https://api.zerobounce.net/v2/validate"),
        (ZeroBounce::with_base_url("k", ApiBaseUrl::USA).unwrap(), "https://api-us.zerobounce.net/v2/validate"),
        (ZeroBounce::with_base_url("k", ApiBaseUrl::EU).unwrap(), "https://api-eu.zerobounce.net/v2/validate"),
        (ZeroBounce::with_base_url("k", "https://custom.example/v2/").unwrap(), "https://custom.example/v2/validate"),
        (ZeroBounce::with_base_url("k", "https://custom.example/v2").unwrap(), "https://custom.example/v2/validate"),
        (ZeroBounce::with_base_url("k", "https://custom.example/v2//").unwrap(), "https://custom.example/v2/validate"),
    ];
    for (zb, want) in cases {
        assert_eq!(zb.url_provider.url_of(ENDPOINT_VALIDATE), want);
    }

    let mut zb = ZeroBounce::new("k");
    assert_eq!(zb.url_provider.bulk_url_of(ENDPOINT_FILE_SEND), "https://bulkapi.zerobounce.net/v2/sendfile");
    assert_eq!(zb.url_provider.inbox_placement_url_of("/providers"), "https://bulkapi.zerobounce.net/api/ipt/providers");
    zb.url_provider.url = "https://custom.example/v2/".into();
    zb.url_provider.bulk_url = "https://bulk.example/v2/".into();
    zb.url_provider.list_evaluator_url = "https://bulk.example/v2/listevaluator/".into();
    zb.url_provider.inbox_placement_url = "https://bulk.example/api/ipt/".into();
    assert_eq!(zb.url_provider.url_of(ENDPOINT_VALIDATE), "https://custom.example/v2/validate");
    assert_eq!(zb.url_provider.bulk_url_of(ENDPOINT_FILE_SEND), "https://bulk.example/v2/sendfile");
    assert_eq!(zb.url_provider.list_evaluator_url_of("/fid/"), "https://bulk.example/v2/listevaluator/fid/");
    assert_eq!(zb.url_provider.inbox_placement_url_of("/providers"), "https://bulk.example/api/ipt/providers");
}

/// The same on the wire, for each kind of base URL a request is built from.
#[test]
fn requests_reach_paths_without_a_double_slash() {
    let (port, fake) = start_fake();
    let mut zb = ZeroBounce::with_base_url("slash-key", format!("http://127.0.0.1:{port}/api/v2/")).unwrap();
    zb.url_provider.bulk_url = format!("http://127.0.0.1:{port}/bulk/v2/");
    zb.url_provider.list_evaluator_url = format!("http://127.0.0.1:{port}/bulk/v2/listevaluator/");
    zb.url_provider.inbox_placement_url = format!("http://127.0.0.1:{port}/api/ipt/");

    zb.get_credits().unwrap();
    assert_eq!(only(&fake).path, "/api/v2/getcredits");
    zb.validate_email("a@example.com").unwrap();
    assert_eq!(only(&fake).path, "/api/v2/validate");
    zb.bulk_validation_file_status_check("fid").unwrap();
    assert_eq!(only(&fake).path, "/bulk/v2/filestatus");
    zb.get_list_evaluator_result("fid").unwrap();
    assert_eq!(only(&fake).path, "/bulk/v2/listevaluator/fid/");
    zb.get_inbox_placement_providers().unwrap();
    assert!(!only(&fake).path.contains("//"));
}

/// The main client must not follow a redirect: the key would go with it. Only
/// the Warmup client had a test, so `Policy::limited(10)` here passed the suite.
/// This uses the client `with_base_url` builds, not the test helper's.
#[test]
fn the_main_client_refuses_redirects() {
    let (port, fake) = start_fake();
    let mut zb = ZeroBounce::with_base_url("redirect-key", format!("http://127.0.0.1:{port}/api/v2/")).unwrap();
    zb.url_provider.bulk_url = format!("http://127.0.0.1:{port}/bulk/v2");
    zb.url_provider.inbox_placement_url = format!("http://127.0.0.1:{port}/api/ipt");
    type Call<'a> = Box<dyn Fn() -> Result<(), ZBError> + 'a>;
    let calls: Vec<(&str, Call)> = vec![
        ("get_credits", Box::new(|| zb.get_credits().map(|_| ()))),
        ("validate_email", Box::new(|| zb.validate_email("a@example.com").map(|_| ()))),
        ("batch_validate", Box::new(|| zb.batch_validate(vec![("a@example.com".into(), String::new())]).map(|_| ()))),
        ("bulk_validation_file_status_check", Box::new(|| zb.bulk_validation_file_status_check("fid").map(|_| ()))),
        ("bulk_validation_result_fetch", Box::new(|| zb.bulk_validation_result_fetch("fid").map(|_| ()))),
        ("get_inbox_placement_providers", Box::new(|| zb.get_inbox_placement_providers().map(|_| ()))),
    ];
    for (name, call) in calls {
        let error = call().expect_err(name);
        assert_eq!(error.status_code(), Some(302), "{name}: {error:?}");
        assert_eq!(
            error.to_string(),
            "ZeroBounce returned a redirect, which this client does not follow.",
            "{name}"
        );
        let seen = fake.take();
        assert_eq!(seen.len(), 1, "{name}: the redirect was followed to {:?}", seen.last().map(|r| &r.path));
    }
}

/// The Warmup client already refused such a key; the main client sent it.
#[test]
fn a_key_with_whitespace_or_control_characters_is_rejected_before_a_request() {
    let (port, fake) = start_fake();
    for key in [" key", "key ", "\tkey", "key\n", "ke\u{0}y", "ke\u{7f}y", "ke\ry"] {
        let zb = client(port, key);
        let results: Vec<(&str, Result<(), ZBError>)> = vec![
            ("get_credits", zb.get_credits().map(|_| ())),
            ("validate_email", zb.validate_email("a@example.com").map(|_| ())),
            ("batch_validate", zb.batch_validate(vec![("a@example.com".into(), String::new())]).map(|_| ())),
            ("bulk_validation_file_status_check", zb.bulk_validation_file_status_check("fid").map(|_| ())),
            ("get_inbox_placement_providers", zb.get_inbox_placement_providers().map(|_| ())),
        ];
        for (name, result) in results {
            match result {
                Err(ZBError::ExplicitError(message)) => assert!(
                    message.contains("surrounding whitespace or control characters"),
                    "{name}: {message}"
                ),
                other => panic!("{name}: expected the key to be rejected, got {other:?}"),
            }
        }
        assert_eq!(fake.len(), 0, "a request was sent");
    }
    client(port, "plain-key").get_credits().unwrap();
    assert_eq!(fake.take().len(), 1);
}

/// Debug output names the type and its fields, with the key redacted.
#[test]
fn debug_output_redacts_the_key() {
    let zb = ZeroBounce::new("sk-debug-secret");
    for printed in [format!("{zb:?}"), format!("{zb:#?}")] {
        assert!(!printed.contains("sk-debug-secret"), "{printed}");
        assert!(printed.contains("ZeroBounce"), "{printed}");
        assert!(printed.contains("***"), "{printed}");
        assert!(printed.contains("https://api.zerobounce.net/v2/"), "{printed}");
    }
}

// -- multi-byte characters at the HTML sniff's cut --------------------------
//
// The HTML sniff looked at the first 2000 bytes with `&body[..2000]`, which
// panicked when a multi-byte character straddled byte 2000. Every JSON
// response and every getfile download goes through it.

/// `prefix` ASCII bytes, so the two-byte 'é' after it spans bytes 1999-2000.
fn straddling_the_cut(prefix: &str) -> String {
    let mut text = String::from(prefix);
    while text.len() < 1999 {
        text.push('a');
    }
    text.push('é');
    assert!(!text.is_char_boundary(2000));
    text
}

#[test]
fn a_validate_body_with_a_multi_byte_character_at_byte_2000_is_parsed() {
    let mut server = mockito::Server::new();
    let body = format!(
        "{}\"}}",
        straddling_the_cut(r#"{"address":"a@example.com","status":"valid","sub_status":"","city":""#)
    );
    let _mock = server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();
    let zb = ZeroBounce::with_base_url("multibyte-key", format!("{}/v2/", server.url())).unwrap();
    let validation = zb.validate_email("a@example.com").unwrap();
    assert!(validation.city.unwrap_or_default().ends_with('é'));
}

#[test]
fn a_getfile_csv_with_a_multi_byte_character_at_byte_2000_is_returned() {
    let mut server = mockito::Server::new();
    let csv = format!("{}\n", straddling_the_cut("email,name\na@example.com,"));
    let _mock = server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/octet-stream")
        .with_body(csv.clone())
        .create();
    let mut zb = ZeroBounce::with_base_url("multibyte-key", format!("{}/v2/", server.url())).unwrap();
    zb.url_provider.bulk_url = format!("{}/v2/", server.url());
    let ZBBulkResponse::Content(content) = zb.bulk_validation_result_fetch("fid").unwrap() else {
        panic!("expected the file content");
    };
    assert_eq!(&content[..], csv.as_bytes());
}

// -- the key in transport errors -------------------------------------------

/// A transport error quotes the request URL, which carries the key as
/// `api_key`. The `From<reqwest::Error>` conversion redacts it; nothing
/// tested that before.
#[test]
fn a_transport_error_redacts_the_key_in_its_url() {
    // Port 1 on loopback refuses the connection, so the request fails in
    // transport with its URL attached.
    let zb = ZeroBounce::with_base_url("transport-secret-key", "http://127.0.0.1:1/v2/").unwrap();
    let error = zb.get_credits().unwrap_err();
    let ZBError::RequestError(inner) = &error else {
        panic!("expected a transport error, got {error:?}");
    };
    let url = inner.url().expect("the error carries the request URL").as_str();
    assert!(url.contains("api_key=REDACTED"), "{url}");
    for printed in [error.to_string(), format!("{error:?}"), format!("{error:#?}")] {
        assert!(!printed.contains("transport-secret-key"), "the key leaked: {printed}");
        assert!(printed.contains("REDACTED"), "{printed}");
    }
}
