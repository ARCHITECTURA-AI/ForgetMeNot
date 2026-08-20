use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FmnMode {
    Shadow,
    Enforcement,
}

impl Default for FmnMode {
    fn default() -> Self {
        Self::Shadow
    }
}

impl fmt::Display for FmnMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shadow => write!(f, "shadow"),
            Self::Enforcement => write!(f, "enforcement"),
        }
    }
}

impl FromStr for FmnMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "shadow" => Ok(Self::Shadow),
            "enforcement" => Ok(Self::Enforcement),
            _ => Err(anyhow::anyhow!("Invalid FMN mode: {}", s)),
        }
    }
}

impl FmnMode {
    #[must_use]
    pub fn is_enforcement(&self) -> bool {
        matches!(self, Self::Enforcement)
    }

    #[must_use]
    pub fn process_output(&self, original: &str, redacted: &str) -> String {
        if self.is_enforcement() {
            redacted.to_string()
        } else {
            original.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_shadow() {
        let mode = FmnMode::default();
        assert_eq!(mode, FmnMode::Shadow);
        assert_eq!(mode.to_string(), "shadow");
    }

    #[test]
    fn enforcement_mode_alters_output() {
        let shadow_mode = FmnMode::Shadow;
        let enforcement_mode = FmnMode::Enforcement;

        let raw_output = "Hello john.smith@acme.com";
        let redacted_output = "Hello [REDACTED]";

        // Shadow mode preserves raw output
        assert_eq!(
            shadow_mode.process_output(raw_output, redacted_output),
            raw_output
        );

        // Enforcement mode alters/redacts output
        assert_eq!(
            enforcement_mode.process_output(raw_output, redacted_output),
            redacted_output
        );
    }

    #[test]
    fn test_from_str() {
        assert_eq!(FmnMode::from_str("shadow").unwrap(), FmnMode::Shadow);
        assert_eq!(
            FmnMode::from_str("enforcement").unwrap(),
            FmnMode::Enforcement
        );
        assert!(FmnMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(FmnMode::Shadow.to_string(), "shadow");
        assert_eq!(FmnMode::Enforcement.to_string(), "enforcement");
    }
}
