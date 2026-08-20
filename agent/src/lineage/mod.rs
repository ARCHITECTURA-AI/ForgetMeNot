use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageId(String);

impl LineageId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LineageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageRecord {
    pub lineage_id: LineageId,
    pub parent_request_id: Option<String>,
    pub child_request_id: Option<String>,
    pub correlation_id: String,
    pub session_token: String,
    pub subject_token: String,
}

impl LineageRecord {
    #[must_use]
    pub fn new(
        lineage_id: impl Into<String>,
        correlation_id: impl Into<String>,
        session_token: impl Into<String>,
        subject_token: impl Into<String>,
    ) -> Self {
        Self {
            lineage_id: LineageId::new(lineage_id),
            parent_request_id: None,
            child_request_id: None,
            correlation_id: correlation_id.into(),
            session_token: session_token.into(),
            subject_token: subject_token.into(),
        }
    }

    #[must_use]
    pub fn create_child(
        &mut self,
        child_lineage_id: impl Into<String>,
        child_request_id: impl Into<String>,
    ) -> Self {
        let child_req = child_request_id.into();
        self.child_request_id = Some(child_req.clone());

        Self {
            lineage_id: LineageId::new(child_lineage_id),
            parent_request_id: Some(self.lineage_id.to_string()),
            child_request_id: None,
            correlation_id: self.correlation_id.clone(),
            session_token: self.session_token.clone(),
            subject_token: self.subject_token.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_preserves_parent_child_relationship_and_unique_ids() {
        let mut parent = LineageRecord::new("lin-parent-001", "corr-12345", "sess-999", "subj-888");

        let child = parent.create_child("lin-child-002", "req-child-001");

        // 1. Unique lineage identifiers
        assert_ne!(parent.lineage_id, child.lineage_id);
        assert_eq!(parent.lineage_id.as_str(), "lin-parent-001");
        assert_eq!(child.lineage_id.as_str(), "lin-child-002");

        // 2. Parent-child relationship preservation
        assert_eq!(child.parent_request_id, Some("lin-parent-001".to_string()));
        assert_eq!(parent.child_request_id, Some("req-child-001".to_string()));

        // 3. Correlation context preservation
        assert_eq!(child.correlation_id, parent.correlation_id);
        assert_eq!(child.session_token, parent.session_token);
        assert_eq!(child.subject_token, parent.subject_token);
    }
}
