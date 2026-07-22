#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationReport {
    pub upstream_count: u64,
    pub proxied_count: u64,
    pub delta: u64,
    pub bypass_detected: bool,
}

impl ReconciliationReport {
    #[must_use]
    pub fn new(upstream_count: u64, proxied_count: u64) -> Self {
        let delta = upstream_count.saturating_sub(proxied_count);
        let bypass_detected = delta > 0;

        Self {
            upstream_count,
            proxied_count,
            delta,
            bypass_detected,
        }
    }
}

pub fn reconcile_call_counts(upstream_count: u64, proxied_count: u64) -> ReconciliationReport {
    ReconciliationReport::new(upstream_count, proxied_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_bypass_is_detected() {
        let upstream_count = 100;
        let proxied_count = 95;

        let report = reconcile_call_counts(upstream_count, proxied_count);

        assert!(report.bypass_detected);
        assert_eq!(report.delta, 5);
        assert_eq!(report.upstream_count, 100);
        assert_eq!(report.proxied_count, 95);
    }

    #[test]
    fn test_no_bypass_when_counts_match() {
        let report = reconcile_call_counts(100, 100);

        assert!(!report.bypass_detected);
        assert_eq!(report.delta, 0);
    }
}
