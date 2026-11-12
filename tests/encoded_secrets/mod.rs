//! Shared by the offline suites: the encodings a server may wrap a secret in,
//! every sequence of up to three of them, and the decoded views of a message
//! (made here, independently of the crate's own decoder).

#![allow(dead_code)]

/// A secret holding every character an encoding changes: space " / \ + % &
/// = < > '.
pub const SECRET: &str = r#"a b"c/d+e%f&g=h<i>j'k\l"#;

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

/// The four encodings: URL with %20, URL with +, JSON, JSON with \/.
pub const ENCODINGS: [&str; 4] = ["url%20", "url+", "json", "json\\/"];

pub fn encode(name: &str, text: &str) -> String {
    match name {
        "url%20" => percent_encode(text, false),
        "url+" => percent_encode(text, true),
        "json" => json_encode(text, false),
        _ => json_encode(text, true),
    }
}

/// Every sequence of up to three encodings, the empty one first: 1 + 4 + 16
/// + 64 = 85.
pub fn sequences() -> Vec<Vec<&'static str>> {
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

/// `text` and every decoding of it up to three layers deep (percent, percent
/// with + as a space, JSON string), each applied to the whole text and to the
/// bracketed echo inside it.
pub fn decoded_views(text: &str) -> Vec<String> {
    let decoders: [fn(&str) -> Option<String>; 3] = [
        |s| strict_percent_decode(s, false),
        |s| strict_percent_decode(s, true),
        |s| serde_json::from_str::<String>(&format!("\"{s}\"")).ok(),
    ];
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
                        if out != candidate {
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

/// The first text with a decoded view that holds `secret`.
pub fn leaking<'a>(texts: &'a [String], secret: &str) -> Option<&'a String> {
    texts
        .iter()
        .find(|text| decoded_views(text).iter().any(|view| view.contains(secret)))
}
