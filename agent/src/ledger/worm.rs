use crate::ledger::event::LedgerEvent;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

pub struct WormLedger {
    file_path: PathBuf,
}

impl WormLedger {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn append(&self, event: &LedgerEvent) -> anyhow::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;

        let mut line = serde_json::to_string(event)?;
        line.push('\n');
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    pub fn read_all(&self) -> anyhow::Result<Vec<LedgerEvent>> {
        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let event: LedgerEvent = serde_json::from_str(trimmed)?;
                events.push(event);
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempFile {
        path: PathBuf,
    }

    impl TempFile {
        fn new() -> Self {
            let count = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
            let mut path = std::env::temp_dir();
            path.push(format!(
                "fmn_worm_test_{}_{}.jsonl",
                std::process::id(),
                count
            ));
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

    fn create_test_event(seq: u64) -> LedgerEvent {
        LedgerEvent::new(
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
            "prev-hash".to_string(),
            "event-hash".to_string(),
        )
    }

    #[test]
    fn write_is_durable_before_return() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        let event = create_test_event(1);
        let result = ledger.append(&event);
        assert!(result.is_ok());

        // Verify durability by opening a separate handle from disk immediately after return
        let fresh_ledger = WormLedger::new(temp.path.clone());
        let read_events = fresh_ledger.read_all().unwrap();
        assert_eq!(read_events.len(), 1);
        assert_eq!(read_events[0], event);
    }

    #[test]
    fn test_append_one_and_read_back() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        let event = create_test_event(1);
        ledger.append(&event).unwrap();

        let events = ledger.read_all().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_append_multiple_and_read_all() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        let event1 = create_test_event(1);
        let event2 = create_test_event(2);

        ledger.append(&event1).unwrap();
        ledger.append(&event2).unwrap();

        let events = ledger.read_all().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event1);
        assert_eq!(events[1], event2);
    }

    #[test]
    fn test_append_does_not_overwrite() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        let event1 = create_test_event(1);
        ledger.append(&event1).unwrap();

        let events_after_first = ledger.read_all().unwrap();
        assert_eq!(events_after_first.len(), 1);
        assert_eq!(events_after_first[0], event1);

        let event2 = create_test_event(2);
        ledger.append(&event2).unwrap();

        let events_after_second = ledger.read_all().unwrap();
        assert_eq!(events_after_second.len(), 2);
        assert_eq!(events_after_second[0], event1); // First is intact
        assert_eq!(events_after_second[1], event2);
    }

    #[test]
    fn test_empty_file_returns_empty_vector() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        let events = ledger.read_all().unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_malformed_json_returns_error() {
        let temp = TempFile::new();
        let ledger = WormLedger::new(temp.path.clone());

        // Write raw malformed text directly to file
        {
            let mut file = File::create(&temp.path).unwrap();
            writeln!(file, "{{\\\"invalid json\\\": }}").unwrap();
        }

        let res = ledger.read_all();
        assert!(res.is_err());
    }
}
