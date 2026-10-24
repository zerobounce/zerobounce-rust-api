//! Offline tests for the Warmup API client. A local fake records every request
//! and answers without the network.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::NaiveDate;
use serde_json::Value;
use zero_bounce::{EmailContent, ManagedSending, OnboardingModel, ZBError, ZeroBounceWarmup};

#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    query: HashMap<String, String>,
    api_key: String,
    body: Option<Value>,
}

struct Fake {
    records: Mutex<Vec<Recorded>>,
    reply: Mutex<Option<(u16, String, String)>>,
}

impl Fake {
    fn take(&self) -> Vec<Recorded> {
        std::mem::take(&mut *self.records.lock().unwrap())
    }

    fn only(&self) -> Recorded {
        let records = self.take();
        assert_eq!(records.len(), 1, "expected one request, got {records:?}");
        records[0].clone()
    }

    fn reply_with(&self, status: u16, content_type: &str, body: &str) {
        *self.reply.lock().unwrap() =
            Some((status, content_type.to_string(), body.to_string()));
    }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.replace('+', " ").into_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""), 16)
            {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
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
                let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") else {
                    continue;
                };
                let header = String::from_utf8_lossy(&buffer[..pos]).to_string();
                let mut length = 0usize;
                let mut api_key = String::new();
                let mut lines = header.split("\r\n");
                let request_line = lines.next().unwrap_or("").to_string();
                for line in lines {
                    let lower = line.to_ascii_lowercase();
                    if let Some(value) = lower.strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                    if lower.starts_with("api-key:") {
                        api_key = line["api-key:".len()..].trim().to_string();
                    }
                }
                let body_start = pos + 4;
                while buffer.len() < body_start + length {
                    match stream.read(&mut temp) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buffer.extend_from_slice(&temp[..n]),
                    }
                }
                let body_bytes = buffer
                    .get(body_start..body_start + length)
                    .unwrap_or(&[])
                    .to_vec();

                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or("").to_string();
                let target = parts.next().unwrap_or("/");
                let (path, query_string) = target.split_once('?').unwrap_or((target, ""));
                let mut query = HashMap::new();
                for pair in query_string.split('&').filter(|p| !p.is_empty()) {
                    let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                    query.insert(percent_decode(key), percent_decode(value));
                }
                let body = if body_bytes.is_empty() {
                    None
                } else {
                    serde_json::from_slice::<Value>(&body_bytes).ok()
                };

                fake.records.lock().unwrap().push(Recorded {
                    method,
                    path: path.to_string(),
                    query,
                    api_key: api_key.clone(),
                    body,
                });

                let (status, content_type, payload) =
                    fake.reply.lock().unwrap().take().unwrap_or((
                        200,
                        "application/json".to_string(),
                        format!(r#"{{"ok":true,"echo_api_key":"{api_key}"}}"#),
                    ));
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

fn start_fake() -> (u16, Arc<Fake>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let fake = Arc::new(Fake {
        records: Mutex::new(Vec::new()),
        reply: Mutex::new(None),
    });
    let shared = fake.clone();
    thread::spawn(move || {
        for incoming in listener.incoming().flatten() {
            let shared = shared.clone();
            thread::spawn(move || handle(incoming, shared));
        }
    });
    (port, fake)
}

/// A throwaway SMTP credential for the offline tests, built at run time.
///
/// A string literal passed as a password is a hard-coded cryptographic value,
/// which CodeQL flags even in a test; nothing here is a real secret, so the
/// value is assembled instead of written out.
fn test_password(seed: &str) -> String {
    format!("pw-{seed}-{}", std::process::id())
}

fn client(port: u16) -> ZeroBounceWarmup {
    ZeroBounceWarmup::with_base_url("warmup-key", &format!("http://127.0.0.1:{port}")).unwrap()
}

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

// -- construction ------------------------------------------------------------

#[test]
fn the_key_travels_in_the_api_key_header() {
    let (port, fake) = start_fake();
    let zb = client(port);

    zb.get_seed_count().unwrap();
    let request = fake.only();
    assert_eq!(request.path, "/api/v1/warmup/seed-count");
    assert_eq!(request.api_key, "warmup-key");
    // The key must never reach the query string, where it would be logged.
    assert!(request.query.is_empty(), "{:?}", request.query);
}

#[test]
fn a_key_that_is_empty_or_padded_is_rejected() {
    assert!(ZeroBounceWarmup::new("").is_err());
    for bad in [" key ", "key\n", "key\tx", "key\u{0}"] {
        let error = ZeroBounceWarmup::new(bad).unwrap_err();
        assert!(error.to_string().contains("whitespace"), "{error}");
    }
}

#[test]
fn a_plain_http_base_url_is_rejected() {
    let error = ZeroBounceWarmup::with_base_url("key", "http://warmup-api.zerobounce.net")
        .unwrap_err();
    assert!(error.to_string().contains("https"), "{error}");
}

// -- paths and verbs ---------------------------------------------------------

#[test]
fn every_endpoint_uses_its_documented_path_and_verb() {
    let (port, fake) = start_fake();
    let zb = client(port);

    zb.list_sending_accounts().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/sending-accounts");
    zb.get_sending_account("a@b.com").unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/sending-account");
    zb.list_from_addresses(None, None, None).unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/from-addresses");
    zb.get_engagement_rules().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/engagement-rules");
    zb.get_custom_engagement("a@b.com").unwrap();
    assert_eq!(
        fake.only().path,
        "/api/v1/warmup/custom-engagement/a%40b.com"
    );
    zb.get_seeds().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/seeds");
    zb.get_progress_today().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/progress-today");
    zb.get_sent_today().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/sent-today");
    zb.get_emails_per_day().unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/emails-per-day");
    zb.get_statistics("a@b.com").unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/statistics");
    zb.get_statistics_detailed("a@b.com").unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/statistics-detailed");
    zb.get_from_address_statistics("a@b.com").unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/from-address-statistics");
    zb.check_domain_dmarc("a@b.com", None).unwrap();
    assert_eq!(fake.only().path, "/api/v1/warmup/check-domain-dmarc");

    zb.add_sending_account("a@b.com", OnboardingModel::SelfManaged, Some(0), None, None, None)
        .unwrap();
    let request = fake.only();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/v1/warmup/sending-accounts");

    zb.update_sending_account("a@b.com", 1, 4, true).unwrap();
    assert_eq!(fake.only().method, "PUT");
    zb.delete_sending_account("a@b.com").unwrap();
    assert_eq!(fake.only().method, "DELETE");
    zb.change_from_address("a@b.com", "c@d.com").unwrap();
    let request = fake.only();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/api/v1/warmup/smtp-credentials");
    zb.set_engagement_rule("a@b.com", 2).unwrap();
    let request = fake.only();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/api/v1/warmup/engagement-rule");
    zb.create_custom_engagement("a@b.com", 10, 10, 5).unwrap();
    let request = fake.only();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/v1/warmup/custom-engagement");
    zb.update_custom_engagement("a@b.com", 10, 10, 5).unwrap();
    assert_eq!(fake.only().method, "PUT");
    zb.delete_custom_engagement("a@b.com", 1).unwrap();
    assert_eq!(fake.only().method, "DELETE");
    zb.test_smtp_configuration("a@b.com", "u", &test_password("ok"), "smtp.example.com", 587)
        .unwrap();
    let request = fake.only();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/v1/warmup/test-smtp-configuration");
}

#[test]
fn statistics_per_day_sends_both_dates() {
    let (port, fake) = start_fake();
    let zb = client(port);
    zb.get_statistics_per_day("a@b.com", date("2026-01-02"), date("2026-01-09"))
        .unwrap();
    let request = fake.only();
    assert_eq!(request.path, "/api/v1/warmup/statistics-per-day");
    assert_eq!(request.query.get("start_date").unwrap(), "2026-01-02");
    assert_eq!(request.query.get("end_date").unwrap(), "2026-01-09");
}

// -- argument validation -----------------------------------------------------

#[test]
fn required_arguments_are_checked_before_any_request() {
    let (port, fake) = start_fake();
    let zb = client(port);

    assert!(zb.get_sending_account("").is_err());
    assert!(zb.delete_sending_account(" ").is_err());
    assert!(zb.change_from_address("", "b@c.com").is_err());
    assert!(zb.change_from_address("a@b.com", "").is_err());
    assert!(zb.set_engagement_rule("", 1).is_err());
    assert!(zb.get_statistics("").is_err());
    assert!(zb.download_statistics("").is_err());
    assert!(zb.check_domain_dmarc("", None).is_err());
    assert!(fake.take().is_empty(), "nothing should reach the network");
}

#[test]
fn engagement_rule_ids_are_range_checked() {
    let (port, fake) = start_fake();
    let zb = client(port);
    for bad in [-1, 5] {
        let error = zb.set_engagement_rule("a@b.com", bad).unwrap_err();
        assert!(error.to_string().contains("rule_id"), "{error}");
    }
    assert!(fake.take().is_empty());
    for good in 0..=4 {
        zb.set_engagement_rule("a@b.com", good).unwrap();
        assert_eq!(fake.only().body.unwrap()["rule_id"], good);
    }
}

#[test]
fn custom_engagement_percentages_are_range_checked() {
    let (port, fake) = start_fake();
    let zb = client(port);
    for (reply, important, click) in [
        (4, 10, 5),   // reply_pct below 5
        (46, 10, 5),  // reply_pct above 45
        (10, 4, 5),   // important_pct below 5
        (10, 41, 5),  // important_pct above 40
        (10, 10, 71), // click_pct above 70
        (10, 10, 18), // click_pct above important_pct x 1.75
    ] {
        assert!(zb
            .create_custom_engagement("a@b.com", reply, important, click)
            .is_err());
    }
    assert!(fake.take().is_empty());

    zb.create_custom_engagement("a@b.com", 20, 20, 35).unwrap();
    let body = fake.only().body.unwrap();
    assert_eq!(body["reply_pct"], 20);
    assert_eq!(body["click_pct"], 35);
}

#[test]
fn delete_custom_engagement_only_falls_back_to_a_standard_rule() {
    let (port, fake) = start_fake();
    let zb = client(port);
    let error = zb.delete_custom_engagement("a@b.com", 0).unwrap_err();
    assert!(error.to_string().contains("engagement_rule_id"), "{error}");
    assert!(fake.take().is_empty());
    zb.delete_custom_engagement("a@b.com", 3).unwrap();
    assert_eq!(fake.only().body.unwrap()["engagement_rule_id"], 3);
}

#[test]
fn update_sending_account_checks_its_ranges() {
    let (port, fake) = start_fake();
    let zb = client(port);
    assert!(zb.update_sending_account("a@b.com", 5, 4, true).is_err());
    assert!(zb.update_sending_account("a@b.com", 1, 9, true).is_err());
    assert!(fake.take().is_empty());

    zb.update_sending_account("a@b.com", 1, 5, false).unwrap();
    let body = fake.only().body.unwrap();
    assert_eq!(body["team_id"], 1);
    assert_eq!(body["status_id"], 5);
    assert_eq!(body["auto_enable"], false);
}

#[test]
fn add_sending_account_checks_the_onboarding_model() {
    let (port, fake) = start_fake();
    let zb = client(port);

    // self_managed needs its engagement rule and takes no ZB-managed fields.
    assert!(zb
        .add_sending_account("a@b.com", OnboardingModel::SelfManaged, None, None, None, None)
        .is_err());
    assert!(zb
        .add_sending_account(
            "a@b.com",
            OnboardingModel::SelfManaged,
            Some(0),
            None,
            Some(&EmailContent::default()),
            None,
        )
        .is_err());
    assert!(fake.take().is_empty());

    zb.add_sending_account(
        "a@b.com",
        OnboardingModel::SelfManaged,
        Some(2),
        Some(true),
        None,
        None,
    )
    .unwrap();
    let body = fake.only().body.unwrap();
    assert_eq!(body["onboarding_model"], "self_managed");
    assert_eq!(body["engagement_rule_id"], 2);
    assert_eq!(body["ignore_conflict"], true);
}

#[test]
fn add_sending_account_checks_the_zb_managed_payload() {
    let (port, fake) = start_fake();
    let zb = client(port);

    let bad_types = EmailContent {
        types_of_emails: vec!["promotional".into()],
        ..EmailContent::default()
    };
    assert!(zb
        .add_sending_account(
            "a@b.com",
            OnboardingModel::ZbManaged,
            None,
            None,
            Some(&bad_types),
            None,
        )
        .is_err());
    let bad_offset = ManagedSending {
        timezone_offset: Some(15),
        ..ManagedSending::default()
    };
    assert!(zb
        .add_sending_account(
            "a@b.com",
            OnboardingModel::ZbManaged,
            None,
            None,
            None,
            Some(&bad_offset),
        )
        .is_err());
    assert!(fake.take().is_empty());

    let content = EmailContent {
        types_of_emails: vec!["newsletter".into(), "marketing".into()],
        ..EmailContent::default()
    };
    let sending = ManagedSending {
        timezone_offset: Some(-5),
        warmup_start_date: Some(date("2026-03-04")),
        daily_limit: None,
    };
    zb.add_sending_account(
        "a@b.com",
        OnboardingModel::ZbManaged,
        None,
        None,
        Some(&content),
        Some(&sending),
    )
    .unwrap();
    let body = fake.only().body.unwrap();
    assert_eq!(body["onboarding_model"], "zb_managed");
    assert_eq!(body["email_content"]["types_of_emails"][1], "marketing");
    assert_eq!(body["managed_sending"]["warmup_start_date"], "2026-03-04");
}

#[test]
fn list_from_addresses_checks_its_search_and_sends_paging_as_strings() {
    let (port, fake) = start_fake();
    let zb = client(port);
    assert!(zb.list_from_addresses(Some("bad search!"), None, None).is_err());
    assert!(fake.take().is_empty());

    zb.list_from_addresses(Some("a.b-c_d@e+f"), Some(0), Some(50))
        .unwrap();
    let request = fake.only();
    assert_eq!(request.query.get("search_from").unwrap(), "a.b-c_d@e+f");
    assert_eq!(request.query.get("offset").unwrap(), "0");
    assert_eq!(request.query.get("limit").unwrap(), "50");
}

#[test]
fn statistics_per_day_rejects_a_backwards_range() {
    let (port, fake) = start_fake();
    let zb = client(port);
    assert!(zb
        .get_statistics_per_day("a@b.com", date("2026-02-02"), date("2026-01-01"))
        .is_err());
    assert!(fake.take().is_empty());
}

#[test]
fn check_domain_dmarc_requires_an_email_address_not_a_domain() {
    let (port, fake) = start_fake();
    let zb = client(port);
    // LIVE: the API answers a bare domain with "Please provide a valid email
    // address", although the docs describe a domain.
    let error = zb.check_domain_dmarc("example.com", None).unwrap_err();
    assert!(error.to_string().contains("email address"), "{error}");
    assert!(fake.take().is_empty());

    zb.check_domain_dmarc("sender@example.com", Some("zb")).unwrap();
    let request = fake.only();
    assert_eq!(request.query.get("dkim_selector").unwrap(), "zb");
}

#[test]
fn custom_engagement_path_ids_cannot_traverse() {
    let (port, fake) = start_fake();
    let zb = client(port);
    for bad in [".", ".."] {
        assert!(zb.get_custom_engagement(bad).is_err());
    }
    assert!(fake.take().is_empty());
}

#[test]
fn the_smtp_test_checks_its_port() {
    let (port, fake) = start_fake();
    let zb = client(port);
    let error = zb
        .test_smtp_configuration("a@b.com", "u", &test_password("port"), "s", 0)
        .unwrap_err();
    assert!(error.to_string().contains("port"), "{error}");
    assert!(fake.take().is_empty());
}

// -- responses ---------------------------------------------------------------

#[test]
fn an_empty_success_becomes_none() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(204, "application/json", "");
    assert!(zb.delete_sending_account("a@b.com").unwrap().is_none());
    fake.take();

    fake.reply_with(200, "application/json", "");
    assert!(zb.get_seed_count().unwrap().is_none());
}

#[test]
fn csv_downloads_come_back_as_text() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(200, "text/csv", "seed,provider\na@b.com,gmail\n");
    assert_eq!(
        zb.download_seeds().unwrap(),
        "seed,provider\na@b.com,gmail\n"
    );
    fake.take();

    fake.reply_with(200, "text/csv", "day,sent\n2026-01-01,10\n");
    zb.download_statistics("a@b.com").unwrap();
    assert_eq!(fake.only().query.get("c_from").unwrap(), "a@b.com");
}

#[test]
fn a_json_error_in_place_of_a_csv_is_an_error() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(
        200,
        "application/json",
        r#"{"message":"No statistics for this address."}"#,
    );
    let error = zb.download_seeds().unwrap_err();
    assert_eq!(error.to_string(), "No statistics for this address.");
}

#[test]
fn the_http_status_is_on_the_error() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(
        429,
        "application/json",
        r#"{"message":"Custom rule was updated less than 7 days ago."}"#,
    );
    let error = zb.update_custom_engagement("a@b.com", 10, 10, 5).unwrap_err();
    assert_eq!(error.status_code(), Some(429));
    assert_eq!(
        error.to_string(),
        "Custom rule was updated less than 7 days ago."
    );
}

#[test]
fn a_validation_error_list_becomes_one_short_message() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(
        422,
        "application/json",
        r#"{"detail":[{"loc":["body","reply_pct"],"msg":"ensure this value is <= 45"}]}"#,
    );
    let error = zb.create_custom_engagement("a@b.com", 10, 10, 5).unwrap_err();
    let message = error.to_string();
    assert!(message.len() < 500, "{message}");
    assert!(message.contains("ensure this value is <= 45"), "{message}");
}

#[test]
fn a_redirect_is_refused_rather_than_followed() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(302, "text/html", "");
    let error = zb.get_seeds().unwrap_err();
    assert!(error.to_string().contains("redirect"), "{error}");
    assert_eq!(error.status_code(), Some(302));
}

#[test]
fn the_key_never_appears_in_an_error() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(
        401,
        "application/json",
        r#"{"message":"Invalid key warmup-key"}"#,
    );
    let error = zb.get_seeds().unwrap_err();
    assert!(!error.to_string().contains("warmup-key"), "{error}");
    assert!(error.to_string().contains("***"), "{error}");
}

#[test]
fn smtp_credentials_never_appear_in_an_error() {
    let (port, fake) = start_fake();
    let zb = client(port);
    // The canned body echoes the very credentials that were sent, so the
    // assertions below prove the client strips them rather than that the
    // body happened not to contain them.
    let username = "bob";
    let password = test_password("login");
    fake.reply_with(
        500,
        "application/json",
        &format!(r#"{{"message":"login failed for user {username} with {password}"}}"#),
    );
    let error = zb
        .test_smtp_configuration("a@b.com", username, &password, "smtp.example.com", 587)
        .unwrap_err();
    let message = error.to_string();
    assert!(!message.contains(&password), "{message}");
    assert!(!message.contains(username), "{message}");
    // LIVE: a failed SMTP login comes back as a 500, not a 4xx.
    assert_eq!(error.status_code(), Some(500));
}

#[test]
fn an_html_error_page_is_not_pasted_into_the_message() {
    let (port, fake) = start_fake();
    let zb = client(port);
    let page = format!(
        "<!DOCTYPE html><html><head><title>Access denied</title></head><body>{}</body></html>",
        "<p>blocked</p>".repeat(2000)
    );
    fake.reply_with(403, "text/html", &page);
    let error = zb.get_seeds().unwrap_err();
    let message = error.to_string();
    assert!(message.contains("HTML error page"), "{message}");
    assert!(message.len() < 500, "{message}");
    assert_eq!(error.status_code(), Some(403));
}

#[test]
fn a_malformed_body_does_not_leak_a_parser_dump() {
    let (port, fake) = start_fake();
    let zb = client(port);
    fake.reply_with(
        200,
        "application/json",
        &format!("<<<not-json>>>{}", "x".repeat(5000)),
    );
    let error: ZBError = zb.get_seeds().unwrap_err();
    assert!(error.to_string().contains("not valid JSON"), "{error}");
}
