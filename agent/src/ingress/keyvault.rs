use std::fmt;

pub struct KeyVault {
    provider_key: String,
}

impl KeyVault {
    #[must_use]
    pub fn new(provider_key: impl Into<String>) -> Self {
        Self {
            provider_key: provider_key.into(),
        }
    }

    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.provider_key
    }
}

impl fmt::Debug for KeyVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("KeyVault").field(&"[REDACTED]").finish()
    }
}

impl fmt::Display for KeyVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_key_is_never_exposed() {
        let raw_secret = "sk-proj-super-secret-openai-api-key-12345";
        let vault = KeyVault::new(raw_secret);

        // 1. Debug formatting MUST redact secret
        let debug_str = format!("{:?}", vault);
        assert!(!debug_str.contains(raw_secret));
        assert!(debug_str.contains("[REDACTED]"));

        // 2. Display formatting MUST redact secret
        let display_str = format!("{}", vault);
        assert!(!display_str.contains(raw_secret));
        assert_eq!(display_str, "[REDACTED]");

        // 3. Secret is only accessible via explicit internal accessor
        assert_eq!(vault.expose_secret(), raw_secret);
    }
}
