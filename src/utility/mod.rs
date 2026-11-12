use std::io::Error as IOError;
use std::io::Read;

pub mod bulk_get_file;
pub mod http_error;
// Test fixtures. Not part of the public API; the integration tests include the
// same file (tests/mod.rs).
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod mock_constants;
pub mod structures;

pub use bulk_get_file::{format_get_file_error_message, get_file_json_indicates_error};

/// A key with surrounding whitespace or control characters would be quoted,
/// unredacted, in the HTTP library's own error, so it is never sent.
pub(crate) fn check_key_text(api_key: &str) -> ZBResult<()> {
    if api_key != api_key.trim() || api_key.chars().any(|c| c.is_control()) {
        return Err(ZBError::explicit(
            "api_key has surrounding whitespace or control characters",
        ));
    }
    Ok(())
}

/// What Debug output shows in place of an API key.
pub(crate) fn redacted_key(api_key: &str) -> &'static str {
    if api_key.is_empty() {
        ""
    } else {
        "***"
    }
}

/// `text` percent-encoded the way a query or form encodes it (space as `+`)
/// and strictly (path escaping, space as `%20`).
fn url_forms(text: &str) -> [String; 2] {
    let mut query = String::new();
    let mut strict = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                query.push(byte as char);
                strict.push(byte as char);
            }
            b'*' => {
                query.push('*');
                strict.push_str("%2A");
            }
            b' ' => {
                query.push('+');
                strict.push_str("%20");
            }
            _ => {
                query.push_str(&format!("%{byte:02X}"));
                strict.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    [query, strict]
}

/// `text` as it sits inside a JSON string, with `/` as is or as `\/`.
fn json_escaped(text: &str, escape_slash: bool) -> String {
    let quoted = serde_json::to_string(text).unwrap_or_default();
    let inner = quoted
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(text);
    if escape_slash {
        inner.replace('/', "\\/")
    } else {
        inner.to_string()
    }
}

/// Every form a message can carry `secret` in, composed from the encoders up
/// to depth 2: {as sent, JSON-escaped, JSON-escaped with `\/` for `/`} x {as
/// is, percent-encoded the way a query or form encodes it, strictly (path
/// escaping)}, and each JSON form escaped twice (a JSON error body quoting
/// the JSON request body). Longest first, so a shorter form is never masked
/// inside a longer one and part of the longer one left behind.
fn secret_forms(secret: &str) -> Vec<String> {
    let mut forms = vec![secret.to_string()];
    forms.extend(url_forms(secret));
    for escape_slash in [false, true] {
        let escaped = json_escaped(secret, escape_slash);
        forms.extend(url_forms(&escaped));
        forms.push(json_escaped(&escaped, escape_slash));
        forms.push(escaped);
    }
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    forms
}

/// Mask every `api_key=` value in a message, whichever key it holds.
fn scrub_query_string_values(text: &str) -> String {
    // ASCII lowercasing keeps every byte offset, so indexes found in `lower`
    // are char boundaries in `text` too.
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    while let Some(found) = lower[from..].find("api_key=") {
        let value_start = from + found + "api_key=".len();
        out.push_str(&text[from..value_start]);
        let value_end = text[value_start..]
            .find(|c: char| c == '&' || c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>'))
            .map_or(text.len(), |offset| value_start + offset);
        if value_end > value_start {
            out.push_str("***");
        }
        from = value_end;
    }
    out.push_str(&text[from..]);
    out
}

/// Replace every form of each secret in `text` with `***`, and every
/// `api_key=` value. Both API clients pass each message built from a
/// response body through it, so a body that echoes the key (an error quoting
/// the request URL, `{"error": "Invalid API key <key>"}`) does not hand the
/// key to the caller.
///
/// Redaction is closed under encoding: a message that still holds a secret in
/// any decoded view ([`secret_survives_decoding`]) is replaced whole by
/// [`CREDENTIAL_OMITTED_MESSAGE`].
pub(crate) fn redact_secrets(text: &str, secrets: &[&str]) -> String {
    let mut value = text.to_string();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        for form in secret_forms(secret) {
            if !form.is_empty() {
                value = value.replace(&form, "***");
            }
        }
    }
    let value = scrub_query_string_values(&value);
    if secret_survives_decoding(&value, secrets) {
        return CREDENTIAL_OMITTED_MESSAGE.to_string();
    }
    value
}

/// What replaces a message that still held a secret after masking.
pub(crate) const CREDENTIAL_OMITTED_MESSAGE: &str =
    "ZeroBounce's response was omitted from this error because it contained a credential.";

/// Whether any view of `text`, decoded up to three layers deep in any order
/// (percent-decoding, percent-decoding with `+` as a space, JSON-unescaping
/// with `\/` and `\uXXXX`), contains a secret. `secret_forms` masks the
/// spellings this crate can generate; this catches every other one (lowercase
/// percent escapes, a third layer, another JSON escaper).
fn secret_survives_decoding(text: &str, secrets: &[&str]) -> bool {
    let secrets: Vec<&str> = secrets.iter().copied().filter(|secret| !secret.is_empty()).collect();
    if secrets.is_empty() {
        return false;
    }
    let decoders: [fn(&str) -> String; 3] = [
        |text| percent_decoded(text, false),
        |text| percent_decoded(text, true),
        json_unescaped,
    ];
    let mut seen = std::collections::HashSet::new();
    seen.insert(text.to_string());
    let mut views = vec![text.to_string()];
    for layer in 0..=3 {
        let mut next = Vec::new();
        for view in &views {
            if secrets.iter().any(|secret| view.contains(secret)) {
                return true;
            }
            if layer == 3 {
                continue;
            }
            for decode in decoders {
                let decoded = decode(view);
                if seen.insert(decoded.clone()) {
                    next.push(decoded);
                }
            }
        }
        views = next;
    }
    false
}

/// `text` with each valid `%XX` escape decoded (and `+` as a space when
/// asked); anything else is kept as it is. Bytes that do not form UTF-8 are
/// replaced, as they cannot be part of a secret.
fn percent_decoded(text: &str, plus_is_space: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((high * 16 + low) as u8);
                i += 3;
                continue;
            }
        }
        out.push(if byte == b'+' && plus_is_space { b' ' } else { byte });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `text` with each valid JSON string escape decoded (`\"`, `\\`, `\/`,
/// `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX` and surrogate pairs); anything
/// else is kept as it is.
fn json_unescaped(text: &str) -> String {
    fn hex4(chars: &[char], at: usize) -> Option<u32> {
        let digits: String = chars.get(at..at + 4)?.iter().collect();
        if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(&digits, 16).ok()
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' || i + 1 >= chars.len() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let simple = match chars[i + 1] {
            '"' => Some('"'),
            '\\' => Some('\\'),
            '/' => Some('/'),
            'b' => Some('\u{8}'),
            'f' => Some('\u{c}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            _ => None,
        };
        if let Some(decoded) = simple {
            out.push(decoded);
            i += 2;
            continue;
        }
        if chars[i + 1] == 'u' {
            if let Some(code) = hex4(&chars, i + 2) {
                // A surrogate pair (\ud83d\ude00) is one character.
                if (0xD800..0xDC00).contains(&code)
                    && chars.get(i + 6) == Some(&'\\')
                    && chars.get(i + 7) == Some(&'u')
                {
                    if let Some(low) = hex4(&chars, i + 8).filter(|low| (0xDC00..0xE000).contains(low)) {
                        let combined = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                        out.push(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                        i += 12;
                        continue;
                    }
                }
                out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                i += 6;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// The HTTP client both API clients use: explicit timeout, redirects never
/// followed (the API key travels with every request, and a redirect could
/// send it to another host). If it cannot be built there is no fallback: a
/// default client would follow redirects.
pub(crate) fn http_client() -> ZBResult<reqwest::blocking::Client> {
    http_client_with_timeout(scaled_for_tests(REQUEST_TIMEOUT))
}

#[cfg(test)]
thread_local! {
    /// This crate's unit tests divide the timeout [`http_client`] builds in
    /// by this, so a test can prove that the clients the public constructors
    /// build carry [`REQUEST_TIMEOUT`] without waiting two minutes.
    static TIMEOUT_DIVISOR: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
}

#[cfg(test)]
thread_local! {
    /// The timeout of the last client [`http_client_with_timeout`] built on
    /// this thread, so a unit test can pin the exact value a constructor uses.
    static LAST_TIMEOUT: std::cell::Cell<Option<std::time::Duration>> = const { std::cell::Cell::new(None) };
}

/// `timeout` unchanged, except in this crate's unit tests, which may shrink
/// it ([`TIMEOUT_DIVISOR`]).
fn scaled_for_tests(timeout: std::time::Duration) -> std::time::Duration {
    #[cfg(test)]
    let timeout = timeout / TIMEOUT_DIVISOR.with(|divisor| divisor.get());
    timeout
}

/// How long one request may take, from connecting to the last byte of the
/// body.
pub(crate) const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// [`http_client`] with another timeout, so a test can prove the timeout holds
/// without waiting two minutes.
fn http_client_with_timeout(
    timeout: std::time::Duration,
) -> ZBResult<reqwest::blocking::Client> {
    #[cfg(test)]
    LAST_TIMEOUT.with(|last| last.set(Some(timeout)));
    client_or_error(
        reqwest::blocking::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build(),
    )
}

fn client_or_error(
    built: reqwest::Result<reqwest::blocking::Client>,
) -> ZBResult<reqwest::blocking::Client> {
    built.map_err(|error| ZBError::ExplicitError(format!("could not build the HTTP client: {error}")))
}

#[cfg(test)]
mod redaction_tests {
    use super::redact_secrets;

    /// Redaction is closed under encoding: every sequence of up to three
    /// encodings of a secret (URL with %20 or +, JSON with or without \/),
    /// then a seeded random sample of them with lowercase percent escapes and
    /// random surrounding text, leaves no decoded view holding the secret. A
    /// spelling `secret_forms` does not generate drops the whole message.
    #[test]
    fn redaction_is_closed_under_encoding() {
        let secret = r#"a b"c/d+e%f&g=h<i>j'k\l"#;
        // URL with %20, URL with +, JSON, JSON with \/.
        let encoders: [fn(&str) -> String; 4] = [
            |text| super::url_forms(text)[1].clone(),
            |text| super::url_forms(text)[0].clone(),
            |text| super::json_escaped(text, false),
            |text| super::json_escaped(text, true),
        ];
        let mut sequences: Vec<Vec<usize>> = vec![vec![]];
        for depth in 1..=3 {
            let mut next = Vec::new();
            for prefix in sequences.iter().filter(|s| s.len() == depth - 1) {
                for encoder in 0..encoders.len() {
                    let mut sequence = prefix.clone();
                    sequence.push(encoder);
                    next.push(sequence);
                }
            }
            sequences.extend(next);
        }
        assert_eq!(sequences.len(), 85);
        // A small LCG, seeded, for the random sample.
        let mut state: u64 = 20261112;
        let mut random = |below: usize| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((state >> 33) as usize) % below
        };
        // (encoder sequence, lowercase it, text around it)
        type Case = (Vec<usize>, bool, String);
        let mut cases: Vec<Case> =
            sequences.iter().map(|sequence| (sequence.clone(), false, String::new())).collect();
        for _ in 0..500 {
            let sequence = (0..random(4)).map(|_| random(encoders.len())).collect();
            cases.push((sequence, random(2) == 0, format!("{{\"x\":{}}} \"q\"", random(1000))));
        }
        let mut omitted = 0;
        for (sequence, lowercase, around) in cases {
            let mut encoded = secret.to_string();
            for &encoder in &sequence {
                encoded = encoders[encoder](&encoded);
            }
            if lowercase {
                encoded = encoded.to_lowercase();
            }
            let message = format!("echo {around}[{encoded}] end");
            for secrets in [vec![secret], vec!["other-key", secret], vec![secret, "smtp-user"]] {
                let got = redact_secrets(&message, &secrets);
                if got == super::CREDENTIAL_OMITTED_MESSAGE {
                    omitted += 1;
                }
                let mut views = vec![got.clone()];
                for _ in 0..3 {
                    let mut next = Vec::new();
                    for view in &views {
                        next.push(super::percent_decoded(view, false));
                        next.push(super::percent_decoded(view, true));
                        next.push(super::json_unescaped(view));
                    }
                    views.extend(next);
                }
                assert!(
                    views.iter().all(|view| !view.contains(secret)),
                    "{sequence:?} lowercase={lowercase}: {got:?}"
                );
            }
        }
        assert!(omitted > 0, "no message needed the decode check");
        // A message with no secret in it is kept as it is.
        assert_eq!(
            redact_secrets(r"Invalid email %2F \u0041", &[secret]),
            r"Invalid email %2F \u0041"
        );
    }

    /// Every form of a key, and any api_key value, whichever key it holds.
    #[test]
    fn every_form_of_a_secret_is_masked() {
        let text = r"raw a/b+c d*e, query a%2Fb%2Bc+d*e, path a%2Fb%2Bc%20d%2Ae, json a\/b+c d*e, url https://x.test/v2?api_key=other-key&email=a";
        assert_eq!(
            redact_secrets(text, &["a/b+c d*e"]),
            "raw ***, query ***, path ***, json ***, url https://x.test/v2?api_key=***&email=a"
        );
        assert_eq!(redact_secrets("API_KEY=x", &[]), "API_KEY=***");
        assert_eq!(redact_secrets("no secret here", &["", "absent"]), "no secret here");
    }

    /// Each form on its own, spelled out, for a secret holding every
    /// character the encodings treat differently, so dropping any one form
    /// fails: as sent, query and strict percent-encoding, JSON-escaped with
    /// and without `\/`, and the query and strict encoding of each JSON form
    /// (a server echoing a URL-encoded JSON request body). serde_json leaves
    /// `<`, `>`, `&` and `'` as they are, so there is no HTML-escaped form.
    #[test]
    fn every_encoding_of_a_secret_is_masked_on_its_own() {
        let secret = r#"a b"c/d+e%f&g=h<i>j'k\l"#;
        let forms = [
            ("raw", r#"a b"c/d+e%f&g=h<i>j'k\l"#),
            ("query", "a+b%22c%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5Cl"),
            ("strict", "a%20b%22c%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5Cl"),
            ("json", r#"a b\"c/d+e%f&g=h<i>j'k\\l"#),
            ("json \\/", r#"a b\"c\/d+e%f&g=h<i>j'k\\l"#),
            ("query json", "a+b%5C%22c%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5C%5Cl"),
            ("strict json", "a%20b%5C%22c%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5C%5Cl"),
            ("query json \\/", "a+b%5C%22c%5C%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5C%5Cl"),
            ("strict json \\/", "a%20b%5C%22c%5C%2Fd%2Be%25f%26g%3Dh%3Ci%3Ej%27k%5C%5Cl"),
        ];
        for (name, text) in forms {
            assert_eq!(
                redact_secrets(&format!("echo [{text}] end"), &[secret]),
                "echo [***] end",
                "the {name} form was left"
            );
        }
        // The raw form of `"k` sits inside its JSON form `\"k`: masking the
        // shorter form first would leave a stray backslash behind.
        assert_eq!(
            redact_secrets(r#"{"error":"bad key \"k"}"#, &[r#""k"#]),
            r#"{"error":"bad key ***"}"#
        );
    }

    /// Every spelling of a secret, composed here from the encoders
    /// themselves, is masked whole: {raw, JSON, JSON with `\/`} x {as is,
    /// query-encoded with `+`, with `%20`}, and each JSON form escaped twice.
    /// The secret holds every character an encoding changes, so the spellings
    /// all differ and dropping any one generator from `secret_forms` fails.
    #[test]
    fn every_composed_spelling_of_a_secret_is_masked_whole() {
        type Encoder = Box<dyn Fn(&str) -> String>;
        let secret = r#"a b"c/d+e%f&g=h<i>j'k\l"#;
        let json = |text: &str, slash: bool| {
            let quoted = serde_json::to_string(text).unwrap();
            let inner = &quoted[1..quoted.len() - 1];
            if slash {
                inner.replace('/', "\\/")
            } else {
                inner.to_string()
            }
        };
        let query = |text: &str| {
            let mut url = reqwest::Url::parse("https://x.test/").unwrap();
            url.query_pairs_mut().append_pair("k", text);
            url.query().unwrap()["k=".len()..].to_string()
        };
        let escapers: Vec<(&str, Encoder)> = vec![
            ("raw", Box::new(|text| text.to_string())),
            ("json", Box::new(move |text| json(text, false))),
            ("json \\/", Box::new(move |text| json(text, true))),
        ];
        let url_encoders: Vec<(&str, Encoder)> = vec![
            ("", Box::new(|text| text.to_string())),
            (", query +", Box::new(move |text| query(text))),
            (", query %20", Box::new(move |text| query(text).replace('+', "%20"))),
        ];
        let mut spellings = std::collections::BTreeMap::new();
        for (escape_name, escape) in &escapers {
            for (url_name, encode) in &url_encoders {
                spellings.insert(format!("{escape_name}{url_name}"), encode(&escape(secret)));
            }
            if *escape_name != "raw" {
                spellings.insert(format!("{escape_name}, twice"), escape(&escape(secret)));
            }
        }
        assert_eq!(spellings.len(), 3 * 3 + 2);
        let distinct: std::collections::HashSet<&String> = spellings.values().collect();
        assert_eq!(distinct.len(), spellings.len(), "{spellings:?}");
        for (name, text) in &spellings {
            assert_eq!(redact_secrets(&format!("<{text}>"), &[secret]), "<***>", "the {name} spelling was left");
        }
    }
}

#[cfg(test)]
mod http_client_tests {
    use super::{client_or_error, http_client_with_timeout, ZBError, LAST_TIMEOUT, REQUEST_TIMEOUT, TIMEOUT_DIVISOR};
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[test]
    fn the_request_timeout_is_two_minutes() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(120));
    }

    /// The Warmup client's default timeout is exactly REQUEST_TIMEOUT: the
    /// behavioural test below only bounds it to within a few tenths.
    #[test]
    fn the_warmup_client_is_built_with_exactly_the_request_timeout() {
        LAST_TIMEOUT.with(|last| last.set(None));
        let _warmup = crate::ZeroBounceWarmup::new("timeout-key").unwrap();
        assert_eq!(LAST_TIMEOUT.with(|last| last.get()), Some(Duration::from_secs(120)));
    }

    /// A local server that accepts every request, reads it and never answers.
    fn never_answering_server() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let mut held = Vec::new();
            for mut stream in listener.incoming().flatten() {
                let mut buffer = [0u8; 4096];
                let _ = stream.read(&mut buffer);
                held.push(stream);
            }
        });
        port
    }

    /// A server that accepts the request and never answers. Without a
    /// timeout the call would hang for ever; the test gives up after 20
    /// seconds and fails instead.
    #[test]
    fn a_server_that_never_answers_times_out() {
        let port = never_answering_server();
        let client = http_client_with_timeout(Duration::from_millis(300)).unwrap();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = client.get(format!("http://127.0.0.1:{port}/v2/getcredits")).send();
            let _ = sender.send((result.map(|response| response.status()), started.elapsed()));
        });
        let (result, elapsed) = receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("the request was still waiting after 20 s: the client has no timeout");
        let error = result.expect_err("a server that never answered gave a response");
        assert!(error.is_timeout(), "{error}");
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    }

    /// The clients `ZeroBounce::new`, `ZeroBounce::with_base_url` and
    /// `ZeroBounceWarmup::with_base_url` (which `ZeroBounceWarmup::new` calls)
    /// build give up after REQUEST_TIMEOUT. The test shrinks that 100-fold
    /// (to 1.2 s), so a client built with any other timeout (or none) fails
    /// here: too soon, too late, or still waiting after 20 s.
    #[test]
    fn the_public_constructors_build_clients_with_the_request_timeout() {
        const DIVISOR: u32 = 100;
        let port = never_answering_server();
        let base = format!("http://127.0.0.1:{port}");
        TIMEOUT_DIVISOR.with(|divisor| divisor.set(DIVISOR));
        let mut from_new = crate::ZeroBounce::new("timeout-key");
        from_new.url_provider.url = format!("{base}/v2");
        let from_base_url = crate::ZeroBounce::with_base_url("timeout-key", format!("{base}/v2")).unwrap();
        let warmup = crate::ZeroBounceWarmup::with_base_url("timeout-key", &base).unwrap();
        TIMEOUT_DIVISOR.with(|divisor| divisor.set(1));

        type Call = Box<dyn FnOnce() -> Result<String, ZBError> + Send>;
        let calls: Vec<(&str, Call)> = vec![
            ("ZeroBounce::new", Box::new(move || from_new.get_credits().map(|v| format!("{v:?}")))),
            ("ZeroBounce::with_base_url", Box::new(move || from_base_url.get_credits().map(|v| format!("{v:?}")))),
            ("ZeroBounceWarmup::with_base_url", Box::new(move || warmup.get_seeds().map(|v| format!("{v:?}")))),
        ];
        let expected = REQUEST_TIMEOUT / DIVISOR;
        for (name, call) in calls {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let started = Instant::now();
                let result = call();
                let _ = sender.send((result, started.elapsed()));
            });
            let (result, elapsed) = receiver
                .recv_timeout(Duration::from_secs(20))
                .unwrap_or_else(|_| panic!("{name}: still waiting after 20 s, want a timeout after {expected:?}"));
            assert!(result.is_err(), "{name}: a server that never answered gave {result:?}");
            assert!(
                elapsed >= expected * 9 / 10 && elapsed <= expected * 3 / 2,
                "{name}: gave up after {elapsed:?}, want about {expected:?}"
            );
        }
    }

    /// Both API clients get their reqwest client from `http_client`, so the
    /// timeout and the redirect policy reach every request. A client built
    /// anywhere else in `src/` would skip them.
    #[test]
    fn every_http_client_is_built_by_http_client() {
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
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_files(&src, &mut files);
        assert!(files.len() > 5, "no Rust sources under {}", src.display());
        let mut found = Vec::new();
        for file in files {
            let source = std::fs::read_to_string(&file).unwrap();
            // This file is the one place a client is built (and its tests
            // build one more to get a builder error).
            if file.ends_with("utility/mod.rs") {
                continue;
            }
            for (number, line) in source.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for pattern in [
                    "Client::new(",
                    "Client::builder(",
                    "Client::default(",
                    "ClientBuilder::new(",
                    "blocking::get(",
                ] {
                    if code.contains(pattern) {
                        found.push(format!("{}:{}: {code}", file.display(), number + 1));
                    }
                }
            }
        }
        assert!(found.is_empty(), "HTTP clients built outside http_client: {found:#?}");
    }

    /// A builder failure used to fall back to `Client::default()`, which
    /// follows redirects. It is now an error.
    #[test]
    fn a_client_that_cannot_be_built_is_an_error_not_a_default_client() {
        let builder_error = reqwest::blocking::Client::new()
            .get("not a url")
            .build()
            .unwrap_err();
        match client_or_error(Err(builder_error)) {
            Err(ZBError::ExplicitError(message)) => {
                assert!(message.starts_with("could not build the HTTP client"), "{message}")
            }
            other => panic!("expected an error, got {other:?}"),
        }
    }
}

pub const CONTENT_TYPE_JSON: &str = "application/json";
pub const CONTENT_TYPE_STREAM: &str = "application/octet-stream";
/// Cap JSON API responses so error pages cannot exhaust memory (RS-05).
pub const MAX_JSON_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
/// Cap bulk getfile downloads (still large enough for typical CSV exports).
pub const MAX_GETFILE_BYTES: usize = 256 * 1024 * 1024;

/// Stop reading after one byte past the cap, so an oversized response cannot
/// consume unbounded memory before the caller checks its length.
pub(crate) fn read_limited(
    mut reader: impl Read,
    max_bytes: usize,
    message: &str,
) -> ZBResult<Vec<u8>> {
    let mut body = Vec::new();
    reader.by_ref().take(max_bytes as u64 + 1).read_to_end(&mut body)?;
    if body.len() > max_bytes {
        return Err(ZBError::explicit(message));
    }
    Ok(body)
}

#[cfg(test)]
mod response_limit_tests {
    use super::read_limited;
    use std::io::{self, Read};

    struct CountingReader {
        remaining: usize,
        read: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = buf.len().min(self.remaining);
            buf[..n].fill(b'x');
            self.remaining -= n;
            self.read += n;
            Ok(n)
        }
    }

    #[test]
    fn stops_at_one_byte_past_limit() {
        let mut source = CountingReader {
            remaining: 10_000,
            read: 0,
        };
        let error = read_limited(&mut source, 32, "too large").unwrap_err();
        assert_eq!(error.to_string(), "too large");
        assert_eq!(source.read, 33);
    }

    #[test]
    fn accepts_body_at_limit() {
        let body = read_limited(&b"exact"[..], 5, "too large").unwrap();
        assert_eq!(body, b"exact");
    }
}

/// Enum representing the available API base URLs
///
/// `#[non_exhaustive]`: a region added later is not a breaking change, so a
/// `match` on it needs a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApiBaseUrl {
    /// Default API URL: https://api.zerobounce.net/v2/
    Default,
    /// USA API URL: https://api-us.zerobounce.net/v2/
    USA,
    /// EU API URL: https://api-eu.zerobounce.net/v2/
    EU,
}

impl ApiBaseUrl {
    /// Get the URL string for this API base URL
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiBaseUrl::Default => "https://api.zerobounce.net/v2/",
            ApiBaseUrl::USA => "https://api-us.zerobounce.net/v2/",
            ApiBaseUrl::EU => "https://api-eu.zerobounce.net/v2/",
        }
    }
}

impl From<ApiBaseUrl> for String {
    fn from(url: ApiBaseUrl) -> Self {
        url.as_str().to_string()
    }
}

impl From<&ApiBaseUrl> for String {
    fn from(url: &ApiBaseUrl) -> Self {
        url.as_str().to_string()
    }
}

/// Percent-encodes a value so it is one path segment.
///
/// Only the unreserved set from RFC 3986 is left alone, so `/`, spaces and the
/// rest are escaped and a file id cannot break out of its segment. Written here
/// rather than pulling in a crate for four lines.
pub fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

pub const BULK_URI: &str = "https://bulkapi.zerobounce.net/v2";
/// The list evaluator and the Inbox Placement Tester are not under the bulk /v2 path.
pub const LIST_EVALUATOR_URI: &str = "https://bulkapi.zerobounce.net/v2/listevaluator";
pub const INBOX_PLACEMENT_URI: &str = "https://bulkapi.zerobounce.net/api/ipt";
pub const ENDPOINT_CREDITS: &str = "/getcredits";
pub const ENDPOINT_ACTIVITY_DATA: &str = "/activity";
pub const ENDPOINT_SCORING: &str = "/scoring";
pub const ENDPOINT_VALIDATE: &str = "/validate";
pub const ENDPOINT_API_USAGE: &str = "/getapiusage";
pub const ENDPOINT_BATCH_VALIDATE: &str = "/validatebatch";
pub const ENDPOINT_FILE_SEND: &str = "/sendfile";
pub const ENDPOINT_FILE_STATUS: &str = "/filestatus";
pub const ENDPOINT_FILE_RESULT: &str = "/getfile";
pub const ENDPOINT_FILE_DELETE: &str = "/deletefile";
pub const ENDPOINT_SCORING_SEND: &str = "/scoring/sendfile";
pub const ENDPOINT_SCORING_STATUS: &str = "/scoring/filestatus";
pub const ENDPOINT_SCORING_RESULT: &str = "/scoring/getfile";
pub const ENDPOINT_SCORING_DELETE: &str = "/scoring/deletefile";
pub const ENDPOINT_EMAIL_FINDER: &str = "/guessformat";

// validation statuses
pub const S_VALID: &str = "valid";
pub const S_INVALID: &str = "invalid";
pub const S_CATCH_ALL: &str = "catch-all";
pub const S_UNKNOWN: &str = "unknown";
pub const S_SPAMTRAP: &str = "spamtrap";
pub const S_ABUSE: &str = "abuse";
pub const S_DO_NOT_MAIL: &str = "do_not_mail";

// validation sub statuses
pub const SS_ANTISPAM_SYSTEM: &str = "antispam_system";
pub const SS_GREYLISTED: &str = "greylisted";
pub const SS_MAIL_SERVER_TEMPORARY_ERROR: &str = "mail_server_temporary_error";
pub const SS_FORCIBLE_DISCONNECT: &str = "forcible_disconnect";
pub const SS_MAIL_SERVER_DID_NOT_RESPOND: &str = "mail_server_did_not_respond";
pub const SS_TIMEOUT_EXCEEDED: &str = "timeout_exceeded";
pub const SS_FAILED_SMTP_CONNECTION: &str = "failed_smtp_connection";
pub const SS_MAILBOX_QUOTA_EXCEEDED: &str = "mailbox_quota_exceeded";
pub const SS_EXCEPTION_OCCURRED: &str = "exception_occurred";
pub const SS_POSSIBLE_TRAP: &str = "possible_trap";
pub const SS_ROLE_BASED: &str = "role_based";
pub const SS_GLOBAL_SUPPRESSION: &str = "global_suppression";
pub const SS_MAILBOX_NOT_FOUND: &str = "mailbox_not_found";
pub const SS_NO_DNS_ENTRIES: &str = "no_dns_entries";
pub const SS_FAILED_SYNTAX_CHECK: &str = "failed_syntax_check";
pub const SS_POSSIBLE_TYPO: &str = "possible_typo";
pub const SS_UNROUTABLE_IP_ADDRESS: &str = "unroutable_ip_address";
pub const SS_LEADING_PERIOD_REMOVED: &str = "leading_period_removed";
pub const SS_DOES_NOT_ACCEPT_MAIL: &str = "does_not_accept_mail";
pub const SS_ALIAS_ADDRESS: &str = "alias_address";
pub const SS_ROLE_BASED_CATCH_ALL: &str = "role_based_catch_all";
pub const SS_DISPOSABLE: &str = "disposable";
pub const SS_TOXIC: &str = "toxic";
pub const SS_ACCEPT_ALL: &str = "accept_all";
pub const SS_AI_AGENT_MAILBOX: &str = "ai_agent_mailbox";

/// Every error the SDK returns.
///
/// `#[non_exhaustive]`: a variant added later is not a breaking change, so a
/// `match` on it needs a `_` arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum ZBError {
    /// An error the SDK raised itself: an invalid argument or URL caught before
    /// a request, or a 2xx response body it could not read.
    ExplicitError(String),
    /// The API answered and the answer is a failure: a non-2xx status, an HTML
    /// error page, an error body on an HTTP 200, a bulk `getfile` error, or a
    /// `batch_validate` account-level error. `status` is the HTTP status of that
    /// response, so callers can tell a 403 from Cloudflare apart from a 400/404
    /// from the API and a 429 from the Warmup rate limit. Both the validation
    /// client and the Warmup client return it.
    ApiError { status: u16, message: String },
    /// A 2xx body that could not be read as the expected type. The inner
    /// error names only the kind of error and its line and column, never the
    /// body's content (serde's own message quotes the value it rejected).
    JsonError(serde_json::Error),
    IntParseError(std::num::ParseIntError),
    RequestError(reqwest::Error),
    IOError(IOError),
}

pub type ZBResult<T> = Result<T, ZBError>;

impl ZBError {
    pub fn explicit(string: &str) -> ZBError {
        ZBError::ExplicitError(string.to_string())
    }

    pub fn api(status: u16, message: impl Into<String>) -> ZBError {
        ZBError::ApiError {
            status,
            message: message.into(),
        }
    }

    /// This error with every secret masked in an API error's message
    /// ([`redact_secrets`]). Other errors carry no response body.
    pub(crate) fn redacted(self, secrets: &[&str]) -> ZBError {
        match self {
            ZBError::ApiError { status, message } => ZBError::ApiError {
                status,
                message: redact_secrets(&message, secrets),
            },
            other => other,
        }
    }

    /// The HTTP status of a failed API response: `Some` for every
    /// [`ZBError::ApiError`], including a failure the API reports with an HTTP
    /// 200. `None` when no failed response is involved: an argument or URL
    /// rejected before a request, a transport error, or a 2xx body that could
    /// not be parsed.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            ZBError::ApiError { status, .. } => Some(*status),
            _ => None,
        }
    }
}

impl std::fmt::Display for ZBError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZBError::ExplicitError(message) => write!(formatter, "{message}"),
            ZBError::ApiError { message, .. } => write!(formatter, "{message}"),
            ZBError::JsonError(_) => {
                write!(
                    formatter,
                    "ZeroBounce returned a response that is not valid JSON."
                )
            }
            ZBError::IntParseError(_) => write!(
                formatter,
                "ZeroBounce returned a response that could not be read."
            ),
            ZBError::RequestError(error) => write!(formatter, "{error}"),
            ZBError::IOError(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ZBError {}

// Implementation made in order to automatically convert errors
// generated by the reqwuest library into a ZBError instance.
//
// This automatic conversion is expected when using the client
// and issuing http requests.
fn redact_api_key(url: &mut reqwest::Url) {
    if !url.query_pairs().any(|(key, _)| key == "api_key") {
        return;
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| {
            let value = if key == "api_key" {
                "REDACTED".to_string()
            } else {
                value.into_owned()
            };
            (key.into_owned(), value)
        })
        .collect();
    url.query_pairs_mut().clear().extend_pairs(pairs);
}

impl From<reqwest::Error> for ZBError {
    fn from(mut value: reqwest::Error) -> ZBError {
        if let Some(url) = value.url_mut() {
            redact_api_key(url);
        }
        ZBError::RequestError(value)
    }
}

// Implementation made in order to automatically convert errors
// generated by the serde_json library into a ZBError instance.
//
// This automatic conversion is expected when deserializing.
//
// serde's own message quotes the value it could not read (`invalid type:
// string "<value>", expected u64`, `unknown variant `<value>``), so a 2xx body
// that echoed the API key in a wrongly typed field handed the key on in the
// `JsonError`'s Debug output and in the error it carries. The error kept here
// says only what kind of error it was and where: no body content.
impl From<serde_json::Error> for ZBError {
    fn from(value: serde_json::Error) -> ZBError {
        use serde::de::Error as _;
        let kind = match value.classify() {
            serde_json::error::Category::Io => "I/O error",
            serde_json::error::Category::Syntax => "invalid JSON",
            serde_json::error::Category::Data => "unexpected JSON data",
            serde_json::error::Category::Eof => "unexpected end of JSON",
        };
        ZBError::JsonError(serde_json::Error::custom(format!(
            "{kind} at line {} column {}",
            value.line(),
            value.column()
        )))
    }
}

impl From<IOError> for ZBError {
    fn from(value: IOError) -> Self {
        ZBError::IOError(value)
    }
}

#[cfg(test)]
mod json_error_tests {
    use super::ZBError;

    /// serde quotes the value it rejected; the error kept drops it and keeps
    /// the position.
    #[test]
    fn a_json_error_keeps_the_position_and_drops_the_value() {
        let parsed: Result<u64, _> = serde_json::from_str("\n  \"secret-value\"");
        let error = ZBError::from(parsed.unwrap_err());
        let ZBError::JsonError(inner) = &error else {
            panic!("{error:?}");
        };
        assert_eq!((inner.line(), inner.column()), (2, 16));
        assert_eq!(inner.to_string(), "unexpected JSON data at line 2 column 16");
        for text in [error.to_string(), format!("{error:?}"), format!("{inner:?}")] {
            assert!(!text.contains("secret-value"), "{text}");
        }
        let syntax: Result<serde_json::Value, _> = serde_json::from_str("[1 secret]");
        let error = ZBError::from(syntax.unwrap_err());
        assert!(!format!("{error:?}").contains("secret"), "{error:?}");
        assert!(format!("{error:?}").contains("invalid JSON"), "{error:?}");
    }
}
