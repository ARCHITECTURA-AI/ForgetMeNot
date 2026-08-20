use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

fn try_decode_base64(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() % 4 != 0 {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return None;
    }
    if let Ok(bytes) = BASE64.decode(trimmed) {
        if let Ok(s) = String::from_utf8(bytes) {
            if !s.is_empty() && s != trimmed {
                return Some(s);
            }
        }
    }
    None
}

fn try_decode_hex(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() % 2 != 0 {
        return None;
    }
    if let Ok(bytes) = hex::decode(trimmed) {
        if let Ok(s) = String::from_utf8(bytes) {
            if !s.is_empty() && s != trimmed {
                return Some(s);
            }
        }
    }
    None
}

/// Decode ROT13-encoded text. Exposed as a standalone function because
/// ROT13 has no structural marker and cannot be auto-detected — every
/// alphabetic string produces a different ROT13 output.
#[must_use]
pub fn decode_rot13(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a'..='m' | 'A'..='M' => ((c as u8) + 13) as char,
            'n'..='z' | 'N'..='Z' => ((c as u8) - 13) as char,
            _ => c,
        })
        .collect()
}

/// Attempt to decode text from Base64 or Hex. If recognised, returns
/// the decoded representation; otherwise returns the original unchanged.
///
/// ROT13 is intentionally excluded from auto-detection (use [`decode_rot13`]
/// explicitly) because it has no structural marker.
#[must_use]
pub fn decode_text(input: &str) -> String {
    if let Some(b64) = try_decode_base64(input) {
        return b64;
    }
    if let Some(hex_decoded) = try_decode_hex(input) {
        return hex_decoded;
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64_then_exposes_email() {
        // Base64 encoding of "john.smith@acme.com" is "am9obi5zbWl0aEBhY21lLmNvbQ=="
        let encoded = "am9obi5zbWl0aEBhY21lLmNvbQ==";
        let decoded = decode_text(encoded);

        assert_eq!(decoded, "john.smith@acme.com");
    }

    #[test]
    fn ignores_non_encoded_text() {
        let plain_text = "Hello, this is regular unencoded text.";
        let result = decode_text(plain_text);

        assert_eq!(result, plain_text);
    }
}
