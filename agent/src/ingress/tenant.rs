use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantError {
    AccessDenied { requester: String, target: String },
}

impl std::error::Error for TenantError {}

impl fmt::Display for TenantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied { requester, target } => {
                write!(f, "Cross-tenant access denied: tenant '{}' cannot access resources of tenant '{}'", requester, target)
            }
        }
    }
}

pub fn verify_tenant_access(requester: &TenantId, target: &TenantId) -> Result<(), TenantError> {
    if requester == target {
        Ok(())
    } else {
        Err(TenantError::AccessDenied {
            requester: requester.to_string(),
            target: target.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_tenant_access_is_denied() {
        let tenant_a = TenantId::new("tenant-acme");
        let tenant_b = TenantId::new("tenant-globex");

        // Same tenant access is allowed
        let same_res = verify_tenant_access(&tenant_a, &tenant_a);
        assert!(same_res.is_ok());

        // Cross-tenant access MUST be denied
        let cross_res = verify_tenant_access(&tenant_a, &tenant_b);
        assert!(cross_res.is_err());
        assert_eq!(
            cross_res,
            Err(TenantError::AccessDenied {
                requester: "tenant-acme".to_string(),
                target: "tenant-globex".to_string(),
            })
        );
    }
}
