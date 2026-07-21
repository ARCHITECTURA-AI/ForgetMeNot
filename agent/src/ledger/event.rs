use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LedgerEvent {
    pub event_id: String,
    pub sequence_no: u64,
    pub timestamp: String,
    pub org_id: String,
    pub subject_token: String,
    pub session_token: String,
    pub event_type: String,
    pub entity_id: Option<String>,
    pub request_id: Option<String>,
    pub fmn_mode: String,
    pub model_version: Option<String>,
    pub action_taken: String,
    pub fn_rate_estimate: f64,
    pub inference_hash: String,
    pub prev_event_hash: String,
    pub event_hash: String,
}

impl LedgerEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: String,
        sequence_no: u64,
        timestamp: String,
        org_id: String,
        subject_token: String,
        session_token: String,
        event_type: String,
        entity_id: Option<String>,
        request_id: Option<String>,
        fmn_mode: String,
        model_version: Option<String>,
        action_taken: String,
        fn_rate_estimate: f64,
        inference_hash: String,
        prev_event_hash: String,
        event_hash: String,
    ) -> Self {
        Self {
            event_id,
            sequence_no,
            timestamp,
            org_id,
            subject_token,
            session_token,
            event_type,
            entity_id,
            request_id,
            fmn_mode,
            model_version,
            action_taken,
            fn_rate_estimate,
            inference_hash,
            prev_event_hash,
            event_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::chain::compute_event_hash;

    #[test]
    fn event_hash_is_deterministic() {
        let event = LedgerEvent::new(
            "event-123".to_string(),
            42,
            "2026-06-24T09:00:00Z".to_string(),
            "org-456".to_string(),
            "subj-789".to_string(),
            "sess-abc".to_string(),
            "INFERENCE_CLEAN".to_string(),
            Some("ent-1".to_string()),
            Some("req-2".to_string()),
            "enforcement".to_string(),
            Some("gpt-4o".to_string()),
            "PASS".to_string(),
            0.05,
            "inf-hash".to_string(),
            "prev-hash".to_string(),
            "current-hash".to_string(),
        );

        let hash1 = compute_event_hash(&event);
        let hash2 = compute_event_hash(&event);

        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_ledger_event_creation() {
        let event = LedgerEvent::new(
            "event-123".to_string(),
            42,
            "2026-06-24T09:00:00Z".to_string(),
            "org-456".to_string(),
            "subj-789".to_string(),
            "sess-abc".to_string(),
            "INFERENCE_CLEAN".to_string(),
            Some("ent-1".to_string()),
            Some("req-2".to_string()),
            "enforcement".to_string(),
            Some("gpt-4o".to_string()),
            "PASS".to_string(),
            0.05,
            "inf-hash".to_string(),
            "prev-hash".to_string(),
            "current-hash".to_string(),
        );

        assert_eq!(event.event_id, "event-123");
        assert_eq!(event.sequence_no, 42);
        assert_eq!(event.timestamp, "2026-06-24T09:00:00Z");
        assert_eq!(event.org_id, "org-456");
        assert_eq!(event.subject_token, "subj-789");
        assert_eq!(event.session_token, "sess-abc");
        assert_eq!(event.event_type, "INFERENCE_CLEAN");
        assert_eq!(event.entity_id, Some("ent-1".to_string()));
        assert_eq!(event.request_id, Some("req-2".to_string()));
        assert_eq!(event.fmn_mode, "enforcement");
        assert_eq!(event.model_version, Some("gpt-4o".to_string()));
        assert_eq!(event.action_taken, "PASS");
        assert_eq!(event.fn_rate_estimate, 0.05);
        assert_eq!(event.inference_hash, "inf-hash");
        assert_eq!(event.prev_event_hash, "prev-hash");
        assert_eq!(event.event_hash, "current-hash");
    }

    #[test]
    fn test_ledger_event_serialization_deserialization() {
        let event = LedgerEvent::new(
            "event-123".to_string(),
            42,
            "2026-06-24T09:00:00Z".to_string(),
            "org-456".to_string(),
            "subj-789".to_string(),
            "sess-abc".to_string(),
            "INFERENCE_CLEAN".to_string(),
            None,
            None,
            "shadow".to_string(),
            None,
            "REDACT".to_string(),
            0.01,
            "inf-hash".to_string(),
            "prev-hash".to_string(),
            "current-hash".to_string(),
        );

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: LedgerEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event, deserialized);
    }
}
