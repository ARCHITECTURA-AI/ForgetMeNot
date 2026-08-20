use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationError {
    MissingKey,
    InvalidKey,
}

impl std::error::Error for AuthenticationError {}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey => write!(f, "Missing API key in authorization header"),
            Self::InvalidKey => write!(f, "Invalid ForgetMeNot API key"),
        }
    }
}

pub fn validate_fmn_key(
    provided_key: Option<&str>,
    expected_key: &str,
) -> Result<(), AuthenticationError> {
    let key = match provided_key {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => return Err(AuthenticationError::MissingKey),
    };

    // Strip optional "Bearer " prefix if present
    let token = key.strip_prefix("Bearer ").unwrap_or(key).trim();

    if token.is_empty() {
        return Err(AuthenticationError::MissingKey);
    }

    if token == expected_key {
        Ok(())
    } else {
        Err(AuthenticationError::InvalidKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_KEY: &str = "fmn-secret-api-key-12345";

    #[test]
    fn missing_key_is_rejected() {
        // None provided
        let res_none = validate_fmn_key(None, EXPECTED_KEY);
        assert_eq!(res_none, Err(AuthenticationError::MissingKey));

        // Empty string provided
        let res_empty = validate_fmn_key(Some(""), EXPECTED_KEY);
        assert_eq!(res_empty, Err(AuthenticationError::MissingKey));

        // Whitespace string provided
        let res_ws = validate_fmn_key(Some("   "), EXPECTED_KEY);
        assert_eq!(res_ws, Err(AuthenticationError::MissingKey));
    }

    #[test]
    fn invalid_key_is_rejected() {
        let res_invalid = validate_fmn_key(Some("wrong-key-999"), EXPECTED_KEY);
        assert_eq!(res_invalid, Err(AuthenticationError::InvalidKey));

        let res_invalid_bearer = validate_fmn_key(Some("Bearer wrong-key"), EXPECTED_KEY);
        assert_eq!(res_invalid_bearer, Err(AuthenticationError::InvalidKey));
    }

    #[test]
    fn test_valid_key_is_accepted() {
        let res_valid = validate_fmn_key(Some(EXPECTED_KEY), EXPECTED_KEY);
        assert!(res_valid.is_ok());

        let res_valid_bearer =
            validate_fmn_key(Some(&format!("Bearer {}", EXPECTED_KEY)), EXPECTED_KEY);
        assert!(res_valid_bearer.is_ok());
    }
}
