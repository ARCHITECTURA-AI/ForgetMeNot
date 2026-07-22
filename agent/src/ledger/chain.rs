use crate::ledger::event::LedgerEvent;
use sha2::{Digest, Sha256};
#[must_use]
pub fn compute_event_hash(event: &LedgerEvent) -> String {
    let data = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        event.event_id,
        event.sequence_no,
        event.timestamp,
        event.org_id,
        event.subject_token,
        event.session_token,
        event.event_type,
        event.entity_id.as_deref().unwrap_or(""),
        event.request_id.as_deref().unwrap_or(""),
        event.fmn_mode,
        event.model_version.as_deref().unwrap_or(""),
        event.action_taken,
        event.fn_rate_estimate,
        event.inference_hash,
        event.prev_event_hash
    );

    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}
#[must_use]
pub fn verify_event_hash(event: &LedgerEvent) -> bool {
    compute_event_hash(event) == event.event_hash
}
#[must_use]
pub fn verify_chain(previous: &LedgerEvent, current: &LedgerEvent) -> bool {
    current.prev_event_hash == previous.event_hash
        && current.sequence_no == previous.sequence_no + 1
        && verify_event_hash(previous)
        && verify_event_hash(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(seq: u64, prev_hash: &str) -> LedgerEvent {
        let mut event = LedgerEvent::new(
            format!("event-{}", seq),
            seq,
            "2026-06-24T09:00:00Z".to_string(),
            "org-123".to_string(),
            "subj-456".to_string(),
            "sess-789".to_string(),
            "INFERENCE_CLEAN".to_string(),
            Some("ent-abc".to_string()),
            Some("req-xyz".to_string()),
            "enforcement".to_string(),
            Some("gpt-4o".to_string()),
            "PASS".to_string(),
            0.05,
            "inf-hash-123".to_string(),
            prev_hash.to_string(),
            "".to_string(),
        );
        event.event_hash = compute_event_hash(&event);
        event
    }

    #[test]
    fn each_event_links_to_previous() {
        let prev = create_test_event(1, "genesis_hash");
        let current = create_test_event(2, &prev.event_hash);

        assert_eq!(current.prev_event_hash, prev.event_hash);
        assert!(verify_chain(&prev, &current));
    }

    #[test]
    fn test_valid_event_hash() {
        let event = create_test_event(1, "genesis_hash");
        assert!(verify_event_hash(&event));
    }

    #[test]
    fn test_invalid_event_hash() {
        let mut event = create_test_event(1, "genesis_hash");
        event.event_hash = "wrong_hash".to_string();
        assert!(!verify_event_hash(&event));
    }

    #[test]
    fn test_valid_chain() {
        let prev = create_test_event(1, "genesis_hash");
        let current = create_test_event(2, &prev.event_hash);
        assert!(verify_chain(&prev, &current));
    }

    #[test]
    fn test_broken_prev_event_hash() {
        let prev = create_test_event(1, "genesis_hash");
        let mut current = create_test_event(2, "wrong_prev_hash");
        // Recompute the event hash for current so it has a valid self-hash,
        // but the link to previous is broken.
        current.event_hash = compute_event_hash(&current);

        assert!(!verify_chain(&prev, &current));
    }

    #[test]
    fn test_broken_sequence_number() {
        let prev = create_test_event(1, "genesis_hash");
        let mut current = create_test_event(3, &prev.event_hash); // gap: 1 -> 3
        current.event_hash = compute_event_hash(&current);

        assert!(!verify_chain(&prev, &current));
    }

    #[test]
    fn test_tampered_event_content() {
        let mut prev = create_test_event(1, "genesis_hash");
        let current = create_test_event(2, &prev.event_hash);

        // Tamper with prev content, but don't re-compute its event_hash
        prev.event_type = "INFERENCE_REDACTED".to_string();

        assert!(!verify_chain(&prev, &current));
    }
}
