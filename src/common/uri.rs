use regex::Regex;
use url::Url;

// Redact secret values in URIs. Handles known keys in query and fragment.
// Keys (case-insensitive): psk, token, pass, password, secret, key
pub fn redact_uri_secrets(input: &str) -> String {
    // Try parsing as URL first
    if let Ok(mut url) = Url::parse(input) {
        // Query params
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| {
                if is_secret_key(&k) {
                    (k.to_string(), "***".to_string())
                } else {
                    (k.to_string(), v.to_string())
                }
            })
            .collect();
        if !pairs.is_empty() {
            url.query_pairs_mut().clear().extend_pairs(pairs.iter().map(|(k, v)| (&k[..], &v[..])));
        }
        // Fragment: sometimes fragments carry key=value pairs
        if let Some(frag) = url.fragment() {
            let red = redact_kv_like(frag);
            let _ = url.set_fragment(Some(&red));
        }
        return url.to_string();
    }

    // Fallback: apply regex-based redaction on raw string (covers non-URL inputs like srt://@...)
    redact_kv_like(input)
}

fn is_secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    matches!(k.as_str(), "psk" | "token" | "pass" | "password" | "secret" | "key")
}

fn redact_kv_like(s: &str) -> String {
    // Build a regex that matches key=value in query or fragment, with optional URL encoding
    // We keep it simple: (?i)(psk|token|pass|password|secret|key)=([^&#]*)
    // Also handle percent-encoded key names by decoding a copy for detection would be heavy; simpler heuristic works well.
    let re = Regex::new(r"(?i)(psk|token|pass|password|secret|key)=([^&#]*)").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let key = &caps[1];
        format!("{}=***", key)
    }).into_owned()
}

#[cfg(test)]
mod tests {
    use super::redact_uri_secrets;

    #[test]
    fn redact_srt_pass() {
        let uri = "srt://127.0.0.1:9000?mode=caller&pass=hello";
        let red = redact_uri_secrets(uri);
        assert!(!red.contains("pass=hello"));
        assert!(red.contains("pass=***"));
    }

    #[test]
    fn redact_rist_psk_env() {
        let uri = "rist://@:10000?mode=listener&psk=env:FOO";
        let red = redact_uri_secrets(uri);
        assert!(!red.contains("psk=env:FOO"));
        assert!(red.contains("psk=***"));
    }

    #[test]
    fn redact_token_urlencoded() {
        let uri = "srt://host?token=abc%20123&mode=caller";
        let red = redact_uri_secrets(uri);
        assert!(!red.contains("token=abc%20123"));
        assert!(red.contains("token=***"));
    }

    #[test]
    fn redact_fragment() {
        let uri = "srt://host/path#secret=shh&x=1";
        let red = redact_uri_secrets(uri);
        assert!(red.contains("secret=***"));
        assert!(!red.contains("secret=shh"));
    }
}
