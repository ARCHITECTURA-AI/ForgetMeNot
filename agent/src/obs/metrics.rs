//! # In-Process Metrics — F2, F21 / T-OBS-3, T-ING-14
//!
//! Provides [`AgentMetrics`], a set of lock-free, monotonically-increasing
//! counters that track agent-level operational events.
//!
//! ## Metrics implemented
//!
//! | Name | Description | TDD reference |
//! |------|-------------|---------------|
//! | `fmn_proxied_calls` | Calls that entered the proxy and were forwarded | T-ING-14 / F2 |
//! | `fmn_inferences_served` | Inferences for which a final decision was reached | INV-3 |
//! | `fmn_blocks_total` | Decisions that were BLOCK | T-OBS-3 / F21 |
//! | `fmn_redactions_total` | Decisions that were REDACT | T-OBS-3 / F21 |
//!
//! ## Privacy guarantee (T-OBS-3 / F21)
//!
//! Counter **names** are generic operational labels — they contain no entity
//! IDs, no hashes, no subject tokens, no email addresses, no request bodies,
//! and no other plaintext PII.  Counter **values** are plain unsigned integers.
//! No sensitive data is stored, serialised, or emitted by this module.
//!
//! ## What this module does NOT do
//!
//! * It does **not** export metrics over the network.
//! * It does **not** implement a Prometheus scrape endpoint.
//! * It does **not** use OpenTelemetry or any other exporter crate.
//! * It does **not** perform detection, redaction, proxying, or ledger writes.

use std::sync::atomic::{AtomicU64, Ordering};

// ── AgentMetrics ──────────────────────────────────────────────────────────────

/// A collection of monotonically-increasing, lock-free counters for the
/// ForgetMeNot agent.
///
/// All reads and writes use [`Ordering::Relaxed`].  Counters are
/// monotonically increasing and never reset during the lifetime of the
/// struct — this matches the semantics of an append-only compliance ledger.
///
/// # Thread safety
///
/// [`AgentMetrics`] may be shared across threads via [`std::sync::Arc`].
/// Increment operations are atomic and require no external locking.
///
/// # Privacy
///
/// No counter name, label, or value contains plaintext PII (T-OBS-3, F21).
pub struct AgentMetrics {
    /// Total number of inference requests that reached the proxy and were
    /// forwarded to the upstream provider.
    ///
    /// Mapped to the `fmn_proxied_calls` counter referenced by T-ING-14 / F2.
    fmn_proxied_calls: AtomicU64,

    /// Total number of inferences for which a final compliance decision was
    /// produced and a ledger event written.
    ///
    /// The invariant `fmn_inferences_served == ledger_event_count` must hold
    /// per tenant (INV-3).
    fmn_inferences_served: AtomicU64,

    /// Total number of decisions that resulted in a BLOCK action.
    ///
    /// This counter never encodes *why* a block occurred or *which* entity
    /// triggered it (T-OBS-3, F21).
    fmn_blocks_total: AtomicU64,

    /// Total number of decisions that resulted in a REDACT action.
    ///
    /// As with blocks, no entity information is encoded in the count.
    fmn_redactions_total: AtomicU64,
}

impl AgentMetrics {
    /// Construct a new [`AgentMetrics`] with all counters initialised to zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fmn_proxied_calls: AtomicU64::new(0),
            fmn_inferences_served: AtomicU64::new(0),
            fmn_blocks_total: AtomicU64::new(0),
            fmn_redactions_total: AtomicU64::new(0),
        }
    }

    // ── Increment operations ──────────────────────────────────────────────────

    /// Increment `fmn_proxied_calls` by one.
    ///
    /// Call this each time a request is accepted and forwarded to the upstream
    /// provider (T-ING-14 / F2).
    #[inline]
    pub fn inc_proxied_calls(&self) {
        self.fmn_proxied_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment `fmn_inferences_served` by one.
    ///
    /// Call this each time a final compliance decision is produced and durably
    /// written to the ledger (INV-3).
    #[inline]
    pub fn inc_inferences_served(&self) {
        self.fmn_inferences_served.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment `fmn_blocks_total` by one.
    ///
    /// Call this when the compliance engine emits a BLOCK decision.
    #[inline]
    pub fn inc_blocks(&self) {
        self.fmn_blocks_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment `fmn_redactions_total` by one.
    ///
    /// Call this when the compliance engine emits a REDACT decision.
    #[inline]
    pub fn inc_redactions(&self) {
        self.fmn_redactions_total.fetch_add(1, Ordering::Relaxed);
    }

    // ── Read operations ───────────────────────────────────────────────────────

    /// Return the current value of `fmn_proxied_calls`.
    #[must_use]
    #[inline]
    pub fn proxied_calls(&self) -> u64 {
        self.fmn_proxied_calls.load(Ordering::Relaxed)
    }

    /// Return the current value of `fmn_inferences_served`.
    #[must_use]
    #[inline]
    pub fn inferences_served(&self) -> u64 {
        self.fmn_inferences_served.load(Ordering::Relaxed)
    }

    /// Return the current value of `fmn_blocks_total`.
    #[must_use]
    #[inline]
    pub fn blocks_total(&self) -> u64 {
        self.fmn_blocks_total.load(Ordering::Relaxed)
    }

    /// Return the current value of `fmn_redactions_total`.
    #[must_use]
    #[inline]
    pub fn redactions_total(&self) -> u64 {
        self.fmn_redactions_total.load(Ordering::Relaxed)
    }

    /// Return a point-in-time snapshot of all counters.
    ///
    /// Because each counter is read independently, the snapshot is **not**
    /// globally consistent under concurrent mutation — use it for
    /// informational purposes only, never for invariant enforcement.
    ///
    /// The snapshot carries **no PII** (T-OBS-3, F21).
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            fmn_proxied_calls: self.proxied_calls(),
            fmn_inferences_served: self.inferences_served(),
            fmn_blocks_total: self.blocks_total(),
            fmn_redactions_total: self.redactions_total(),
        }
    }
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ── MetricsSnapshot ───────────────────────────────────────────────────────────

/// A plain-data point-in-time snapshot of all [`AgentMetrics`] counters.
///
/// All fields are `u64` counts.  No PII is stored (T-OBS-3, F21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsSnapshot {
    /// Value of `fmn_proxied_calls` at snapshot time.
    pub fmn_proxied_calls: u64,
    /// Value of `fmn_inferences_served` at snapshot time.
    pub fmn_inferences_served: u64,
    /// Value of `fmn_blocks_total` at snapshot time.
    pub fmn_blocks_total: u64,
    /// Value of `fmn_redactions_total` at snapshot time.
    pub fmn_redactions_total: u64,
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── T-ING-14 ─────────────────────────────────────────────────────────────
    //
    // "proxied_calls_are_counted": each proxied call increments
    // `fmn_proxied_calls`. (F2)

    /// T-ING-14 — `proxied_calls_are_counted`
    ///
    /// Incrementing once per "proxied call" must be reflected in the counter
    /// value.  N increments → counter reads N.
    #[test]
    fn proxied_calls_are_counted() {
        // Arrange
        let m = AgentMetrics::new();

        // Act — simulate 5 proxied calls
        m.inc_proxied_calls();
        m.inc_proxied_calls();
        m.inc_proxied_calls();
        m.inc_proxied_calls();
        m.inc_proxied_calls();

        // Assert
        assert_eq!(
            m.proxied_calls(),
            5,
            "fmn_proxied_calls must equal the number of inc_proxied_calls() calls"
        );
    }

    // ── T-OBS-3 ──────────────────────────────────────────────────────────────
    //
    // "metrics_have_no_pii": exported metrics carry hashed IDs only. (F21)
    //
    // The test verifies that the snapshot — the only data structure this
    // module exposes to callers — contains only u64 counts and no string
    // fields that could accidentally carry plaintext PII.

    /// T-OBS-3 — `metrics_have_no_pii`
    ///
    /// After recording blocks and redactions, the snapshot must contain only
    /// numeric counts — no email addresses, entity IDs, tokens, or any other
    /// sensitive plaintext.
    #[test]
    fn metrics_have_no_pii() {
        // Arrange — simulate a mix of events
        let m = AgentMetrics::new();
        m.inc_proxied_calls();
        m.inc_proxied_calls();
        m.inc_inferences_served();
        m.inc_inferences_served();
        m.inc_blocks();
        m.inc_redactions();

        // Act
        let snap = m.snapshot();

        // Assert — values are correct unsigned integers (no PII)
        assert_eq!(snap.fmn_proxied_calls, 2);
        assert_eq!(snap.fmn_inferences_served, 2);
        assert_eq!(snap.fmn_blocks_total, 1);
        assert_eq!(snap.fmn_redactions_total, 1);

        // Structural assertion: MetricsSnapshot has no String fields that
        // could carry PII.  Verified at compile-time by the struct definition
        // (all fields are u64), but we make the intent explicit here.
        let _ = format!("{snap:?}"); // must not contain any sensitive string
        assert!(!format!("{snap:?}").contains('@'));
        assert!(!format!("{snap:?}").contains("sk-"));
        assert!(!format!("{snap:?}").contains("Bearer"));
    }

    // ── Additional robustness tests ───────────────────────────────────────────

    /// All counters start at zero after construction.
    #[test]
    fn new_metrics_start_at_zero() {
        let m = AgentMetrics::new();
        assert_eq!(m.proxied_calls(), 0);
        assert_eq!(m.inferences_served(), 0);
        assert_eq!(m.blocks_total(), 0);
        assert_eq!(m.redactions_total(), 0);
    }

    /// Default construction is equivalent to `AgentMetrics::new()`.
    #[test]
    fn default_produces_zero_counters() {
        let m = AgentMetrics::default();
        assert_eq!(m.proxied_calls(), 0);
        assert_eq!(m.inferences_served(), 0);
        assert_eq!(m.blocks_total(), 0);
        assert_eq!(m.redactions_total(), 0);
    }

    /// Each counter is independent — incrementing one must not affect others.
    #[test]
    fn counters_are_independent() {
        let m = AgentMetrics::new();
        m.inc_blocks();
        m.inc_blocks();

        assert_eq!(m.blocks_total(), 2);
        // All other counters unchanged
        assert_eq!(m.proxied_calls(), 0);
        assert_eq!(m.inferences_served(), 0);
        assert_eq!(m.redactions_total(), 0);
    }

    /// `snapshot()` captures values at call time.
    #[test]
    fn snapshot_reflects_current_counts() {
        let m = AgentMetrics::new();
        m.inc_proxied_calls();
        m.inc_inferences_served();

        let snap = m.snapshot();
        assert_eq!(snap.fmn_proxied_calls, 1);
        assert_eq!(snap.fmn_inferences_served, 1);
        assert_eq!(snap.fmn_blocks_total, 0);
        assert_eq!(snap.fmn_redactions_total, 0);
    }

    /// Increments are deterministic: same sequence of calls always yields
    /// the same counter value.
    #[test]
    fn increment_is_deterministic() {
        let m1 = AgentMetrics::new();
        let m2 = AgentMetrics::new();

        for _ in 0..7 {
            m1.inc_proxied_calls();
            m2.inc_proxied_calls();
        }

        assert_eq!(m1.proxied_calls(), m2.proxied_calls());
    }
}
