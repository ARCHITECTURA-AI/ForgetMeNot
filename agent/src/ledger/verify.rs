use crate::ledger::chain::{verify_chain, verify_event_hash};
use crate::ledger::event::LedgerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct VerificationResult {
    pub valid: bool,
    pub total_events: usize,
    pub errors: Vec<String>,
}
#[must_use]
pub fn verify_ledger(events: &[LedgerEvent]) -> VerificationResult {
    let mut errors = Vec::new();

    if events.is_empty() {
        return VerificationResult {
            valid: true,
            total_events: 0,
            errors: Vec::new(),
        };
    }

    // 1. Verify every event hash
    for event in events {
        if !verify_event_hash(event) {
            errors.push(format!(
                "Invalid event hash at sequence {}",
                event.sequence_no
            ));
        }
    }

    // 2. Verify sequence continuity
    if events[0].sequence_no != 1 {
        errors.push(format!(
            "Sequence gap between 0 and {}",
            events[0].sequence_no
        ));
    }
    for i in 1..events.len() {
        let prev = events[i - 1].sequence_no;
        let curr = events[i].sequence_no;
        if curr != prev + 1 {
            errors.push(format!("Sequence gap between {} and {}", prev, curr));
        }
    }

    // 3. Verify chain continuity
    for i in 1..events.len() {
        let previous = &events[i - 1];
        let current = &events[i];
        if !verify_chain(previous, current) {
            errors.push(format!(
                "Broken chain between {} and {}",
                previous.sequence_no, current.sequence_no
            ));
        }
    }

    VerificationResult {
        valid: errors.is_empty(),
        total_events: events.len(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::chain::compute_event_hash;

    fn create_test_event(seq: u64, prev_hash: &str, is_valid_hash: bool) -> LedgerEvent {
        let mut event = LedgerEvent::new(
            format!("event-{}", seq),
            seq,
            "2026-06-24T09:00:00Z".to_string(),
            "org-123".to_string(),
            "subj-456".to_string(),
            "sess-789".to_string(),
            "INFERENCE_CLEAN".to_string(),
            None,
            None,
            "shadow".to_string(),
            None,
            "PASS".to_string(),
            0.05,
            "inf-hash".to_string(),
            prev_hash.to_string(),
            "".to_string(),
        );

        if is_valid_hash {
            event.event_hash = compute_event_hash(&event);
        } else {
            event.event_hash = "invalid-hash".to_string();
        }

        event
    }

    #[test]
    fn tampering_breaks_chain_and_names_index() {
        let mut events = Vec::new();
        let mut prev_hash = "genesis".to_string();
        for seq in 1..=10 {
            let event = create_test_event(seq, &prev_hash, true);
            prev_hash = event.event_hash.clone();
            events.push(event);
        }

        // Mutate event #5 (at sequence 5) in the event chain
        events[4].event_type = "TAMPERED_EVENT".to_string();

        let result = verify_ledger(&events);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("sequence 5") || e.contains("between 5 and 6")));
    }

    #[test]
    fn test_empty_ledger() {
        let result = verify_ledger(&[]);
        assert!(result.valid);
        assert_eq!(result.total_events, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_single_valid_event() {
        let event = create_test_event(1, "genesis", true);
        let result = verify_ledger(&[event]);
        assert!(result.valid);
        assert_eq!(result.total_events, 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_multiple_valid_events() {
        let event1 = create_test_event(1, "genesis", true);
        let event2 = create_test_event(2, &event1.event_hash, true);
        let event3 = create_test_event(3, &event2.event_hash, true);

        let result = verify_ledger(&[event1, event2, event3]);
        assert!(result.valid);
        assert_eq!(result.total_events, 3);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_hash() {
        let event1 = create_test_event(1, "genesis", false); // invalid hash
        let result = verify_ledger(&[event1]);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Invalid event hash at sequence 1");
    }

    #[test]
    fn test_sequence_gap() {
        let event1 = create_test_event(1, "genesis", true);
        let event2 = create_test_event(3, &event1.event_hash, true); // seq gap: 1 -> 3

        let result = verify_ledger(&[event1, event2]);
        assert!(!result.valid);
        // Sequence gap triggers two failures:
        // 1. Sequence gap error
        // 2. Broken chain error (since verify_chain checks sequence increment)
        assert!(result
            .errors
            .iter()
            .any(|e| e == "Sequence gap between 1 and 3"));
        assert!(result
            .errors
            .iter()
            .any(|e| e == "Broken chain between 1 and 3"));
    }

    #[test]
    fn test_broken_prev_event_hash() {
        let event1 = create_test_event(1, "genesis", true);
        let mut event2 = create_test_event(2, "wrong_prev_hash", true); // broken link
        event2.event_hash = compute_event_hash(&event2); // self hash is valid

        let result = verify_ledger(&[event1, event2]);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Broken chain between 1 and 2");
    }

    #[test]
    fn test_multiple_errors_collected() {
        // Event 1 has invalid self-hash
        let event1 = create_test_event(1, "genesis", false);
        // Event 2 has sequence gap (3 instead of 2) and broken prev_event_hash link
        let mut event2 = create_test_event(3, "broken_prev", true);
        event2.event_hash = compute_event_hash(&event2); // self hash is valid

        let result = verify_ledger(&[event1, event2]);
        assert!(!result.valid);

        // We expect:
        // 1. Invalid event hash at sequence 1
        // 2. Sequence gap between 1 and 3
        // 3. Broken chain between 1 and 3
        assert!(result
            .errors
            .iter()
            .any(|e| e == "Invalid event hash at sequence 1"));
        assert!(result
            .errors
            .iter()
            .any(|e| e == "Sequence gap between 1 and 3"));
        assert!(result
            .errors
            .iter()
            .any(|e| e == "Broken chain between 1 and 3"));
    }
}
