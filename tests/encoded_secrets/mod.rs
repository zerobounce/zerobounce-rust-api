//! Shared by the offline suites: the encodings a server may wrap a secret in,
//! every sequence of up to three of them, and the decoded views of a message
//! (made here, independently of the crate's own decoder). The quote spellings
//! (`\'`, `''`, `\x27`, `\X27`, HTML entities in either case) and an
//! upper-cased echo are swept in sequences of up to two, for the API key, the
//! Warmup key and the SMTP password alike.

#![allow(dead_code)]

/// A secret holding every character an encoding changes (space " % & ' + /
/// < = > \) between letters: `a b"c%d&e'f+g/h<i=j>k\l`. Built at run time
/// from the ASCII table rather than written as a literal, so no hard-coded
/// value flows into the SMTP password the Warmup suite sends.
pub fn secret() -> String {
    let changed_by_an_encoding = |c: &char| {
        matches!(c, ' ' | '"' | '%' | '&' | '\'' | '+' | '/' | '<' | '=' | '>' | '\\')
    };
    let specials: Vec<char> = (0u32..128).filter_map(char::from_u32).filter(changed_by_an_encoding).collect();
    let letters: Vec<char> = (0u32..128)
        .filter_map(char::from_u32)
        .filter(char::is_ascii_lowercase)
        .take(specials.len() + 1)
        .collect();
    let mut secret = String::new();
    for (index, letter) in letters.iter().enumerate() {
        secret.push(*letter);
        if let Some(special) = specials.get(index) {
            secret.push(*special);
        }
    }
    secret
}

fn percent_encode(text: &str, space_as_plus: bool) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            b' ' if space_as_plus => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn json_encode(text: &str, escape_slash: bool) -> String {
    let quoted = serde_json::to_string(text).unwrap();
    let inner = quoted[1..quoted.len() - 1].to_string();
    if escape_slash {
        inner.replace('/', "\\/")
    } else {
        inner
    }
}

/// The four URL and JSON encodings: URL with %20, URL with +, JSON, JSON with \/.
pub const ENCODINGS: [&str; 4] = ["url%20", "url+", "json", "json\\/"];

pub fn encode(name: &str, text: &str) -> String {
    match name {
        "url%20" => percent_encode(text, false),
        "url+" => percent_encode(text, true),
        "json" => json_encode(text, false),
        "json\\/" => json_encode(text, true),
        "upper" => text.to_uppercase(),
        "backslash-quote" => text.replace('\'', "\\'"),
        "doubled-quote" => text.replace('\'', "''"),
        "hex-x27" => text.replace('\'', "\\x27"),
        "hex-X27" => text.replace('\'', "\\X27"),
        "html-apos" => text.replace('\'', "&apos;"),
        "html-APOS" => text.replace('\'', "&APOS;"),
        "html-39" => text.replace('\'', "&#39;"),
        "html-x27" => text.replace('\'', "&#x27;"),
        other => panic!("unknown encoding {other}"),
    }
}

/// Echoes that are not URL or JSON: an upper-cased copy, and a quote written
/// as a backslash and quote, doubled, a `\x27` or `\X27` byte, or an HTML
/// reference in lower, upper or numeric form. Only the quote is rewritten.
pub const QUOTE_ENCODINGS: [&str; 9] = [
    "upper",
    "backslash-quote",
    "doubled-quote",
    "hex-x27",
    "hex-X27",
    "html-apos",
    "html-APOS",
    "html-39",
    "html-x27",
];

/// The 85 sequences of up to three of the four URL and JSON encodings, then
/// every sequence of one or two encodings from all thirteen that uses at least
/// one of [`QUOTE_ENCODINGS`]: 9 + (13 x 13 - 4 x 4) = 162, 247 in all.
pub fn sequences() -> &'static [Vec<&'static str>] {
    static ALL: std::sync::OnceLock<Vec<Vec<&'static str>>> = std::sync::OnceLock::new();
    ALL.get_or_init(build_sequences)
}

fn build_sequences() -> Vec<Vec<&'static str>> {
    let mut all = base_sequences();
    let everything: Vec<&'static str> = ENCODINGS.iter().chain(QUOTE_ENCODINGS.iter()).copied().collect();
    let uses_quote_encoding = |sequence: &Vec<&'static str>| sequence.iter().any(|name| QUOTE_ENCODINGS.contains(name));
    let mut frontier: Vec<Vec<&'static str>> = vec![Vec::new()];
    for _ in 0..2 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for name in &everything {
                let mut sequence = prefix.clone();
                sequence.push(*name);
                next.push(sequence);
            }
        }
        all.extend(next.iter().filter(|sequence| uses_quote_encoding(sequence)).cloned());
        frontier = next;
    }
    assert_eq!(all.len(), 85 + 9 + 13 * 13 - 4 * 4);
    all
}

/// Every sequence of up to three of the four URL and JSON encodings, the empty
/// one first: 1 + 4 + 16 + 64 = 85.
fn base_sequences() -> Vec<Vec<&'static str>> {
    let mut all = vec![Vec::new()];
    let mut frontier: Vec<Vec<&'static str>> = vec![Vec::new()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for name in ENCODINGS {
                let mut sequence = prefix.clone();
                sequence.push(name);
                next.push(sequence);
            }
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }
    assert_eq!(all.len(), 85);
    all
}

pub fn apply(sequence: &[&str], text: &str) -> String {
    sequence.iter().fold(text.to_string(), |text, name| encode(name, &text))
}

fn strict_percent_decode(text: &str, plus_is_space: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// `\xHH` and `\XHH` as that byte; None when there is none or the result is
/// not UTF-8.
fn hex_byte_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut found = false;
    while i < bytes.len() {
        if bytes[i] == b'\\' && matches!(bytes.get(i + 1), Some(b'x' | b'X')) {
            if let Some(hex) = bytes.get(i + 2..i + 4).and_then(|hex| std::str::from_utf8(hex).ok()) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 4;
                    found = true;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    if found {
        String::from_utf8(out).ok()
    } else {
        None
    }
}

/// `&name;` and `&#NN;` / `&#xHH;` references, the name in any case.
fn html_decode(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = text;
    let mut found = false;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').filter(|end| *end <= 8).and_then(|end| {
            let name = rest[1..end].to_lowercase();
            let character = match name.as_str() {
                "apos" => Some('\''),
                "quot" => Some('"'),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                _ => name
                    .strip_prefix("#x")
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|number| number.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((character, end + 1))
        });
        match decoded {
            Some((character, used)) => {
                out.push(character);
                rest = &rest[used..];
                found = true;
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    found.then_some(out)
}

/// `text` and every decoding of it up to three layers deep (percent, percent
/// with + as a space, JSON string, backslash-quote, doubled quote, hex byte,
/// HTML reference), each applied to the whole text and to the bracketed echo
/// inside it.
pub fn decoded_views(text: &str) -> Vec<String> {
    let decoders: [fn(&str) -> Option<String>; 7] = [
        |s| strict_percent_decode(s, false),
        |s| strict_percent_decode(s, true),
        |s| serde_json::from_str::<String>(&format!("\"{s}\"")).ok(),
        |s| s.contains("\\'").then(|| s.replace("\\'", "'")),
        |s| s.contains("''").then(|| s.replace("''", "'")),
        hex_byte_decode,
        html_decode,
    ];
    let mut seen = std::collections::HashSet::new();
    seen.insert(text.to_string());
    let mut views = vec![text.to_string()];
    let mut frontier = vec![text.to_string()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for view in &frontier {
            let mut candidates = vec![view.clone()];
            if let (Some(start), Some(end)) = (view.find('['), view.rfind(']')) {
                if end > start {
                    candidates.push(view[start + 1..end].to_string());
                }
            }
            for candidate in candidates {
                for decode in decoders {
                    if let Some(out) = decode(&candidate) {
                        if out != candidate && seen.insert(out.clone()) {
                            next.push(out);
                        }
                    }
                }
            }
        }
        views.extend(next.iter().cloned());
        frontier = next;
    }
    views
}

/// The first text with a decoded view that holds `secret`, in any case.
pub fn leaking<'a>(texts: &'a [String], secret: &str) -> Option<&'a String> {
    let secret = secret.to_lowercase();
    texts
        .iter()
        .find(|text| decoded_views(text).iter().any(|view| view.to_lowercase().contains(&secret)))
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn there_are_247_sequences_and_each_quote_encoding_is_in_some() {
        let all = sequences();
        assert_eq!(all.len(), 247);
        for name in QUOTE_ENCODINGS {
            assert!(all.iter().any(|sequence| sequence == &[name]), "{name}");
        }
    }

    /// The checker must see the secret behind every spelling, or the sweeps
    /// that use it would pass on a leak: the key, the Warmup key and the SMTP
    /// password all go through `leaking`.
    #[test]
    fn leaking_sees_the_secret_behind_every_sequence() {
        let secret = secret();
        for sequence in sequences() {
            let echo = format!("Bad [{}] end", apply(sequence, &secret));
            assert!(leaking(std::slice::from_ref(&echo), &secret).is_some(), "{sequence:?}: {echo}");
        }
        assert!(leaking(&["Bad [***] end".to_string()], &secret).is_none());
    }

    /// Both clients refuse a key with a Unicode space or an invisible
    /// character anywhere in it, before any request is made (the base URL
    /// here is never contacted), and keep allowing an inner ASCII space.
    #[test]
    fn both_clients_refuse_a_key_with_a_unicode_space_or_an_invisible_character() {
        let base = "http://127.0.0.1:9";
        for c in ['\u{a0}', '\u{2028}', '\u{3000}', '\u{200B}', '\u{FEFF}', '\u{180E}', '\u{FE0F}', '\u{E0020}', '\t'] {
            let key = format!("ab{c}cd");
            let error = zero_bounce::ZeroBounceWarmup::with_base_url(&key, base).expect_err(&format!("warmup U+{:04X}", c as u32));
            assert!(error.to_string().contains("surrounding whitespace or control characters"), "{error}");
            let zb = zero_bounce::ZeroBounce::with_base_url(&key, base).unwrap();
            let error = zb.get_credits().expect_err(&format!("main U+{:04X}", c as u32));
            assert!(error.to_string().contains("surrounding whitespace or control characters"), "{error}");
        }
        assert!(zero_bounce::ZeroBounceWarmup::with_base_url("ab cd", base).is_ok());
    }

    #[test]
    fn an_upper_cased_echo_is_a_leak() {
        let secret = secret();
        assert!(leaking(&[secret.to_uppercase()], &secret).is_some());
        assert!(leaking(&[apply(&["upper", "url%20"], &secret)], &secret).is_some());
        assert!(leaking(&[apply(&["hex-X27", "upper"], &secret)], &secret).is_some());
        assert!(leaking(&[apply(&["html-APOS", "upper"], &secret)], &secret).is_some());
    }
}
