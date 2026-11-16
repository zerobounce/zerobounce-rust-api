//! Transport limits and URL refusals, against a raw one-response server that
//! writes canned bytes: a chunked body cut mid-stream, an oversize error body,
//! and base URLs that would send a request somewhere else.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use zero_bounce::{ZBError, ZeroBounce, ZeroBounceWarmup};

const KEY: &str = "transport-dummy-key-0123";

/// Serves `response` to every connection, closing it after the write.
/// Returns the port and the number of connections accepted.
fn serve(response: Vec<u8>) -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            seen.fetch_add(1, Ordering::SeqCst);
            let response = response.clone();
            thread::spawn(move || {
                let mut buffer = Vec::new();
                let mut temp = [0u8; 4096];
                while !buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut temp) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => buffer.extend_from_slice(&temp[..n]),
                    }
                }
                let _ = stream.write_all(&response);
            });
        }
    });
    (port, count)
}

/// One chunk holding `body`, then the connection closes: with `finished` the
/// terminating zero chunk is sent, without it the body is cut mid-stream.
fn chunked(content_type: &str, body: &str, finished: bool) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n",
        body.len()
    );
    if finished {
        response.push_str("0\r\n\r\n");
    }
    response.into_bytes()
}

fn sized(status: u16, content_type: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn assert_cut_error_without_key(name: &str, error: ZBError) {
    let text = format!("{error} {error:?}");
    assert!(!text.contains(KEY), "{name}: {text}");
}

const VALIDATE_JSON: &str = r#"{"address":"a@example.com","status":"valid"}"#;

#[test]
fn a_chunked_body_cut_mid_stream_is_an_error_on_validate_getfile_and_warmup() {
    // Control: the same chunked body, finished, is a success.
    let (port, _) = serve(chunked("application/json", VALIDATE_JSON, true));
    let zb = ZeroBounce::with_base_url(KEY, format!("http://127.0.0.1:{port}/v2")).unwrap();
    assert!(zb.validate_email("a@example.com").is_ok());

    let (port, _) = serve(chunked("application/json", VALIDATE_JSON, false));
    let zb = ZeroBounce::with_base_url(KEY, format!("http://127.0.0.1:{port}/v2")).unwrap();
    assert_cut_error_without_key("validate", zb.validate_email("a@example.com").unwrap_err());

    let (port, _) = serve(chunked(
        "text/csv",
        "email,status\r\na@example.com,valid\r\n",
        false,
    ));
    let mut zb = ZeroBounce::with_base_url(KEY, format!("http://127.0.0.1:{port}/v2")).unwrap();
    zb.url_provider.bulk_url = format!("http://127.0.0.1:{port}/v2");
    assert_cut_error_without_key(
        "getfile",
        zb.bulk_validation_result_fetch("fid").unwrap_err(),
    );

    let (port, _) = serve(chunked("application/json", r#"{"count":1}"#, false));
    let warmup = ZeroBounceWarmup::with_base_url(KEY, &format!("http://127.0.0.1:{port}")).unwrap();
    assert_cut_error_without_key("warmup", warmup.get_seed_count().unwrap_err());
}

/// 17 MiB, over the 16 MiB error-body cap and far under the 256 MiB file cap.
fn big_body() -> String {
    "x".repeat(17 << 20)
}

#[test]
fn an_oversize_error_body_gets_the_error_body_cap_not_the_file_cap() {
    let (port, _) = serve(sized(500, "text/csv", &big_body()));
    let mut zb = ZeroBounce::with_base_url(KEY, format!("http://127.0.0.1:{port}/v2")).unwrap();
    zb.url_provider.bulk_url = format!("http://127.0.0.1:{port}/v2");
    let error = zb.bulk_validation_result_fetch("fid").unwrap_err();
    assert_eq!(error.status_code(), Some(500), "{error:?}");
    assert_eq!(error.to_string(), "getfile response exceeds size limit");

    let warmup = ZeroBounceWarmup::with_base_url(KEY, &format!("http://127.0.0.1:{port}")).unwrap();
    let error = warmup.get_seed_count().unwrap_err();
    assert_eq!(error.status_code(), Some(500), "{error:?}");
    assert_eq!(error.to_string(), "response body exceeds size limit");
}

const BAD_BASES: [&str; 8] = [
    "https://api.example.com/v2?x=1",
    "https://api.example.com/v2?",
    "https://api.example.com/v2#f",
    "https://api.example.com/v2 ",
    " https://api.example.com/v2",
    "https://api.example.com/v\t2",
    "https://api.example.com/v2\u{200B}",
    "https://api.example.com\\v2",
];

#[test]
fn a_base_url_with_a_query_a_fragment_or_whitespace_is_refused_at_construction() {
    for base in BAD_BASES {
        let error = ZeroBounce::with_base_url(KEY, base).unwrap_err();
        assert!(
            error.to_string().contains("query, a fragment"),
            "{base:?}: {error}"
        );
        let error = ZeroBounceWarmup::with_base_url(KEY, base).unwrap_err();
        assert!(
            error.to_string().contains("query, a fragment"),
            "{base:?}: {error}"
        );
    }
    // A plain http host is still the scheme refusal.
    assert!(ZeroBounceWarmup::with_base_url(KEY, "http://example.com")
        .unwrap_err()
        .to_string()
        .contains("https://"));
}

#[test]
fn the_warmup_client_keeps_the_specific_userinfo_message() {
    let error = ZeroBounceWarmup::with_base_url(KEY, "https://api.zerobounce.net@evil.example/")
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "request URL must not contain a username or password"
    );
}

#[test]
fn a_provider_url_set_after_construction_is_refused_at_send_time() {
    let (port, count) = serve(sized(200, "application/json", r#"{"Credits":"1"}"#));
    for base in [
        format!("http://127.0.0.1:{port}/v2?x=1"),
        format!("http://127.0.0.1:{port}/v2?"),
        format!("http://127.0.0.1:{port}/v2#f"),
        format!("http://127.0.0.1:{port}/v2 "),
    ] {
        let mut zb = ZeroBounce::with_base_url(KEY, format!("http://127.0.0.1:{port}/v2")).unwrap();
        zb.url_provider.url = base.clone();
        let error = zb.get_credits().unwrap_err();
        assert!(
            error.to_string().contains("query, a fragment"),
            "{base:?}: {error}"
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0, "a request was sent");
}
