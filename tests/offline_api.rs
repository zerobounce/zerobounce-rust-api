use std::collections::HashMap;
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
use zero_bounce::utility::BULK_URI;
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
    if key.starts_with("double-error-") {
        return (
            400,
            "text/plain",
            "Error: Error: api_key is invalid".to_string(),
        );
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
                    let response = format!(
                        "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        for incoming in listener.incoming() {
            if let Ok(stream) = incoming {
                let shared = shared.clone();
                thread::spawn(move || handle(stream, shared));
            }
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
    assert_eq!(env!("CARGO_PKG_VERSION"), "2.1.3");
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

    let mut options = ZBValidateOptions::default();
    options.activity_data = Some(false);
    options.verify_plus = Some(true);
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

    let mut posted = ZBValidateOptions::default();
    posted.use_post = true;
    posted.timeout = Some(15);
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

    let mut batch_options = ZBValidateOptions::default();
    batch_options.timeout = Some(30);
    batch_options.activity_data = Some(false);
    batch_options.verify_plus = Some(true);
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
    let mut bad = ZBValidateOptions::default();
    bad.timeout = Some(2);
    assert!(zb
        .validate_email_with_options("a@example.com", "", &bad)
        .unwrap_err()
        .to_string()
        .contains("timeout"));
    let mut batch = ZBValidateOptions::default();
    batch.timeout = Some(9);
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
        let port = port;
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
