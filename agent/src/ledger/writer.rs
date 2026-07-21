use crate::ledger::event::LedgerEvent;
use crate::ledger::worm::WormLedger;
use crate::ledger::chain::verify_event_hash;

pub struct LedgerWriter {
    worm: WormLedger,
}
#[must_use]
impl LedgerWriter {
    pub fn new(worm: WormLedger) -> Self {
        Self { worm }
    }

    pub fn append(&self, event: LedgerEvent) -> anyhow::Result<()> {
        let existing = self.worm.read_all()?;

        if existing.is_empty() {
            if event.sequence_no != 1 {
                return Err(anyhow::anyhow!(
                    "Genesis event must have sequence number 1"
                ));
            }
        } else if let Some(last_event) = existing.last() {
            if event.sequence_no != last_event.sequence_no + 1 {
                return Err(anyhow::anyhow!(
                    "Invalid sequence number: expected {}, got {}",
                    last_event.sequence_no + 1,
                    event.sequence_no
                ));
            }
            if event.prev_event_hash != last_event.event_hash {
                return Err(anyhow::anyhow!("Invalid previous event hash link"));
            }
        }

        if !verify_event_hash(&event) {
            return Err(anyhow::anyhow!("Ledger event hash verification failed"));
        }

        self.worm.append(&event)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::chain::compute_event_hash;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new() -> Self {
            let count = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut path = std::env::temp_dir();
            path.push(format!("fmn_writer_refac_test_{}_{}.jsonl", std::process::id(), count));
            Self { path }
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            if self.path.exists() {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    fn create_test_event(seq: u64, prev_hash: &str, is_valid_hash: bool) -> LedgerEvent {
        let mut event = LedgerEvent::new(
            format!("event-id-{}", seq),
            seq,
            "2026-06-24T10:00:00Z".to_string(),
            "org-123".to_string(),
            "subj-456".to_string(),
            "sess-789".to_string(),
            "INFERENCE_CLEAN".to_string(),
            None,
            None,
            "shadow".to_string(),
            None,
            "PASS".to_string(),
            0.0,
            "inf-hash".to_string(),
            prev_hash.to_string(),
            "".to_string(),
        );

        if is_valid_hash {
            event.event_hash = compute_event_hash(&event);
        } else {
            event.event_hash = "invalid-hash-string".to_string();
        }

        event
    }

    #[test]
    fn failed_write_returns_error_not_silent_pass() {
        let invalid_path = PathBuf::from("/non_existent_directory_12345/fmn.ledger");
        let worm = WormLedger::new(invalid_path);
        let writer = LedgerWriter::new(worm);

        let event = create_test_event(1, "genesis_prev_hash", true);
        let res = writer.append(event);

        assert!(res.is_err());
    }

    #[test]
    fn test_ledger_writer_genesis_event() {
        let temp = TempFile::new();
        let worm = WormLedger::new(temp.path.clone());
        let writer = LedgerWriter::new(worm);

        let genesis = create_test_event(1, "genesis_prev_hash", true);
        let res = writer.append(genesis.clone());
        assert!(res.is_ok());

        // Verify it is indeed written
        let read_worm = WormLedger::new(temp.path.clone());
        let events = read_worm.read_all().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], genesis);
    }

    #[test]
    fn test_ledger_writer_valid_append() {
        let temp = TempFile::new();
        let worm = WormLedger::new(temp.path.clone());
        let writer = LedgerWriter::new(worm);

        // 1. Append Genesis
        let genesis = create_test_event(1, "genesis_prev_hash", true);
        writer.append(genesis.clone()).unwrap();

        // 2. Append Next Event
        let second = create_test_event(2, &genesis.event_hash, true);
        let res = writer.append(second.clone());
        assert!(res.is_ok());

        // Check file has both events
        let read_worm = WormLedger::new(temp.path.clone());
        let events = read_worm.read_all().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], genesis);
        assert_eq!(events[1], second);
    }

    #[test]
    fn test_ledger_writer_invalid_hash() {
        let temp = TempFile::new();
        let worm = WormLedger::new(temp.path.clone());
        let writer = LedgerWriter::new(worm);

        let genesis = create_test_event(1, "genesis_prev_hash", false);
        let res = writer.append(genesis);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("hash verification failed"));
    }

    #[test]
    fn test_ledger_writer_invalid_sequence_number() {
        let temp = TempFile::new();
        let worm = WormLedger::new(temp.path.clone());
        let writer = LedgerWriter::new(worm);

        // 1. Append Genesis
        let genesis = create_test_event(1, "genesis_prev_hash", true);
        writer.append(genesis.clone()).unwrap();

        // 2. Append Event with sequence jump (3 instead of 2)
        let second = create_test_event(3, &genesis.event_hash, true);
        let res = writer.append(second);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Invalid sequence number"));
    }

    #[test]
    fn test_ledger_writer_invalid_prev_event_hash() {
        let temp = TempFile::new();
        let worm = WormLedger::new(temp.path.clone());
        let writer = LedgerWriter::new(worm);

        // 1. Append Genesis
        let genesis = create_test_event(1, "genesis_prev_hash", true);
        writer.append(genesis.clone()).unwrap();

        // 2. Append Event with invalid previous hash
        let second = create_test_event(2, "some_incorrect_hash", true);
        let res = writer.append(second);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Invalid previous event hash link"));
    }
}
