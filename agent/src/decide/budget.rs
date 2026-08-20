//! # Decision Budget — F-budget / T-DEC-6
//!
//! Tracks the **processing budget** consumed by the agent across a single
//! request (or session).  The budget is an abstract unit ceiling; callers
//! charge units against it and check whether it is exhausted.
//!
//! ## Primary use-case — accumulated medium-confidence hit escalation
//!
//! T-DEC-6: N medium-confidence contextual hits about one entity across a
//! session each contribute one unit.  When the accumulated count reaches the
//! ceiling, the session budget is exhausted and the caller must escalate to
//! [`crate::decide::decision::Decision::Block`] even though no individual hit
//! would trigger a block on its own.
//!
//! ## What this module does NOT do
//!
//! * It does not call any tokenizer or OpenAI API.
//! * It does not perform billing or quota enforcement against an external
//!   service.
//! * It does not perform any network I/O.
//! * It does not write logs or touch storage.

// ── ProcessingBudget ──────────────────────────────────────────────────────────

/// A bounded counter that tracks how many processing units have been consumed
/// out of a fixed ceiling.
///
/// Units are intentionally abstract — callers decide what one unit represents
/// (e.g. one medium-confidence detection hit, one buffered token chunk, one
/// scan invocation).  This keeps the accounting layer decoupled from the
/// detection layer.
///
/// # Invariants
///
/// * `consumed <= ceiling` at all times.
/// * `ceiling` is fixed at construction and never changes.
/// * All operations are infallible and side-effect free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingBudget {
    /// Maximum units allowed before the budget is considered exhausted.
    ceiling: u64,
    /// Units consumed so far.  Always `<= ceiling`.
    consumed: u64,
}

impl ProcessingBudget {
    /// Create a new [`ProcessingBudget`] with the given `ceiling`.
    ///
    /// `consumed` starts at zero.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `ceiling` is zero — a zero-ceiling budget
    /// is immediately exhausted and serves no useful purpose.
    #[must_use]
    pub fn new(ceiling: u64) -> Self {
        debug_assert!(ceiling > 0, "ProcessingBudget: ceiling must be > 0");
        Self {
            ceiling,
            consumed: 0,
        }
    }

    // ── Mutating operations ───────────────────────────────────────────────────

    /// Charge `units` against the budget.
    ///
    /// `consumed` increases by `units`, saturating at `ceiling` so that it
    /// never exceeds the ceiling regardless of how many units are charged.
    ///
    /// Returns the number of units **actually charged** (may be less than
    /// `units` if the budget was nearly exhausted).
    pub fn consume(&mut self, units: u64) -> u64 {
        let available = self.ceiling.saturating_sub(self.consumed);
        let charged = units.min(available);
        self.consumed = self.consumed.saturating_add(charged);
        charged
    }

    // ── Query operations ──────────────────────────────────────────────────────

    /// Returns the number of units remaining before the budget is exhausted.
    ///
    /// Always in the range `[0, ceiling]`.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.ceiling.saturating_sub(self.consumed)
    }

    /// Returns `true` when no units remain — i.e. `consumed >= ceiling`.
    ///
    /// When `exhausted()` is `true`, the caller **must** treat subsequent
    /// decisions as [`crate::decide::decision::Decision::Block`] (T-DEC-6).
    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.consumed >= self.ceiling
    }

    /// Returns the fixed ceiling this budget was constructed with.
    #[must_use]
    pub fn ceiling(&self) -> u64 {
        self.ceiling
    }

    /// Returns the total units consumed so far.
    #[must_use]
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Reset `consumed` to zero, restoring the full budget.
    ///
    /// Useful when a budget instance is reused across multiple requests in a
    /// session pool — call `reset()` at the start of each new request.
    pub fn reset(&mut self) {
        self.consumed = 0;
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn new_budget_starts_fully_available() {
        let b = ProcessingBudget::new(10);

        assert_eq!(b.remaining(), 10);
        assert_eq!(b.consumed(), 0);
        assert!(!b.exhausted());
    }

    #[test]
    fn ceiling_is_returned_correctly() {
        let b = ProcessingBudget::new(42);
        assert_eq!(b.ceiling(), 42);
    }

    // ── Representative behaviour 1: consuming updates remaining ───────────────

    /// Charging one unit reduces `remaining` by one and increments `consumed`.
    #[test]
    fn consuming_one_unit_updates_remaining() {
        // Arrange
        let mut b = ProcessingBudget::new(5);

        // Act
        let charged = b.consume(1);

        // Assert
        assert_eq!(charged, 1);
        assert_eq!(b.consumed(), 1);
        assert_eq!(b.remaining(), 4);
        assert!(!b.exhausted());
    }

    /// Consuming multiple units in a single call updates remaining correctly.
    #[test]
    fn consuming_multiple_units_updates_remaining() {
        // Arrange
        let mut b = ProcessingBudget::new(10);

        // Act
        let charged = b.consume(3);

        // Assert
        assert_eq!(charged, 3);
        assert_eq!(b.consumed(), 3);
        assert_eq!(b.remaining(), 7);
    }

    /// Consuming across several calls accumulates correctly — models T-DEC-6
    /// where each medium-confidence hit charges one unit.
    #[test]
    fn successive_consumes_accumulate() {
        // Arrange
        let mut b = ProcessingBudget::new(5);

        // Act: simulate 3 separate medium-confidence hits
        b.consume(1);
        b.consume(1);
        b.consume(1);

        // Assert
        assert_eq!(b.consumed(), 3);
        assert_eq!(b.remaining(), 2);
        assert!(!b.exhausted());
    }

    // ── Representative behaviour 2: exhausted budget reports correctly ─────────

    /// When exactly `ceiling` units have been consumed, `exhausted()` is true
    /// and `remaining()` is zero.
    #[test]
    fn budget_exhausted_at_ceiling() {
        // Arrange
        let mut b = ProcessingBudget::new(3);

        // Act: consume exactly the ceiling
        b.consume(1);
        b.consume(1);
        b.consume(1);

        // Assert
        assert_eq!(b.remaining(), 0);
        assert!(b.exhausted());
    }

    /// T-DEC-6 scenario: N accumulated medium-confidence hits exhaust the
    /// budget even though no individual hit would have triggered a block.
    #[test]
    fn accumulated_medium_hits_exhaust_budget() {
        // Arrange: session budget of 4 medium-confidence hits allowed
        let mut b = ProcessingBudget::new(4);

        // Act: 4 hits each charge 1 unit (each alone would PASS)
        for _ in 0..4 {
            b.consume(1);
        }

        // Assert: budget now exhausted → caller must escalate to Block
        assert!(b.exhausted());
        assert_eq!(b.remaining(), 0);
    }

    // ── Saturation — consume can never push consumed above ceiling ────────────

    /// Attempting to consume more than the ceiling saturates at the ceiling;
    /// no overflow or panic occurs.
    #[test]
    fn consume_saturates_at_ceiling() {
        // Arrange
        let mut b = ProcessingBudget::new(5);

        // Act: request far more units than available
        let charged = b.consume(100);

        // Assert
        assert_eq!(charged, 5); // only 5 actually charged
        assert_eq!(b.consumed(), 5);
        assert_eq!(b.remaining(), 0);
        assert!(b.exhausted());
    }

    /// Consuming zero units is a no-op.
    #[test]
    fn consuming_zero_units_is_a_noop() {
        let mut b = ProcessingBudget::new(10);
        let charged = b.consume(0);

        assert_eq!(charged, 0);
        assert_eq!(b.consumed(), 0);
        assert_eq!(b.remaining(), 10);
        assert!(!b.exhausted());
    }

    /// Charging additional units against an already exhausted budget is safe
    /// and charges nothing.
    #[test]
    fn consume_on_exhausted_budget_charges_zero() {
        let mut b = ProcessingBudget::new(2);
        b.consume(2); // exhaust it

        let charged = b.consume(5);

        assert_eq!(charged, 0);
        assert_eq!(b.consumed(), 2);
        assert!(b.exhausted());
    }

    // ── remaining() + consumed() == ceiling ──────────────────────────────────

    /// The invariant `consumed + remaining == ceiling` always holds.
    #[test]
    fn consumed_plus_remaining_equals_ceiling() {
        let mut b = ProcessingBudget::new(7);

        for units in [2, 1, 3] {
            b.consume(units);
            assert_eq!(
                b.consumed() + b.remaining(),
                b.ceiling(),
                "invariant broken after consuming {units} units"
            );
        }
    }

    // ── reset ─────────────────────────────────────────────────────────────────

    /// After `reset()`, the budget returns to the full ceiling as if freshly
    /// constructed.
    #[test]
    fn reset_restores_full_budget() {
        let mut b = ProcessingBudget::new(5);
        b.consume(3);

        b.reset();

        assert_eq!(b.consumed(), 0);
        assert_eq!(b.remaining(), 5);
        assert!(!b.exhausted());
    }

    /// reset() on an exhausted budget makes it usable again.
    #[test]
    fn reset_on_exhausted_budget_makes_it_usable() {
        let mut b = ProcessingBudget::new(2);
        b.consume(2);
        assert!(b.exhausted());

        b.reset();

        assert!(!b.exhausted());
        let charged = b.consume(1);
        assert_eq!(charged, 1);
    }

    // ── Clone produces an independent copy ───────────────────────────────────

    /// Consuming from a clone does not affect the original.
    #[test]
    fn clone_is_independent() {
        let mut original = ProcessingBudget::new(10);
        original.consume(3);

        let mut cloned = original.clone();
        cloned.consume(5);

        // original unchanged
        assert_eq!(original.consumed(), 3);
        // clone advanced independently
        assert_eq!(cloned.consumed(), 8);
    }
}
