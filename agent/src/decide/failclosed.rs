//! # Fail-Closed Safety Wrapper — T-DEC-3 / T-DEC-4
//!
//! Guarantees that **no failure mode can produce an unsafe [`Decision::Allow`]**.
//!
//! The rule is absolute:
//!
//! > If detector output is missing, malformed, or internal decision evaluation
//! > fails for any reason, the result **must** be [`Decision::Block`].
//!
//! ## How to use
//!
//! The caller (stream / ingress layer) drives the pipeline:
//!
//! 1. Ask the detector for findings → `Option<Vec<DetectorFinding>>`.
//!    `None` means the detector did not return output (timeout, network error,
//!    missing response, etc.).
//! 2. Call [`evaluate_decision`] on the findings → `Result<DecisionResult, E>`.
//!    An `Err` means something went wrong during evaluation itself.
//! 3. Pass both into [`fail_closed`].  If either signals a problem, the
//!    returned [`DecisionResult`] is always [`Decision::Block`].
//!
//! ## What this module does NOT do
//!
//! * It does not run the detector.
//! * It does not inspect or re-parse raw text.
//! * It does not write logs, ledger events, or touch storage.
//! * It does not perform any network I/O.

use crate::decide::decision::{Decision, DecisionReason, DecisionResult, DetectorFinding};

// ── FailClosedReason — why the fail-closed path was taken ────────────────────

/// Records why the fail-closed path was activated.
///
/// Stored inside the [`DecisionResult`] returned when a failure is caught, so
/// that the caller can write a meaningful ledger event without this module
/// having to do so directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClosedReason {
    /// The detector produced no output at all (timeout, missing response, etc.).
    DetectorOutputMissing,
    /// The detector returned output, but [`evaluate_decision`] returned an
    /// error — the findings could not be safely evaluated.
    EvaluationFailed,
}

// ── fail_closed — the core safety wrapper ────────────────────────────────────

/// Wrap the decision pipeline so that every failure mode yields
/// [`Decision::Block`].
///
/// # Parameters
///
/// * `detector_output` — `None` if the detector did not return findings (e.g.
///   timeout, scanner error, missing body).  `Some(findings)` if the detector
///   returned a (possibly empty) list of findings.
///
/// * `evaluation_result` — the `Result` produced by calling
///   [`evaluate_decision`] on the findings.  Pass `Err(e)` if evaluation
///   itself panicked or failed for any reason.  The error value `E` is
///   discarded; only the `Ok`/`Err` distinction matters here.
///
/// # Returns
///
/// * If `detector_output` is `None` → `Decision::Block` with
///   [`FailClosedReason::DetectorOutputMissing`] recorded in the reason field
///   (via [`DecisionReason::BlockRequired`]) and `fail_closed_reason` set.
/// * If `evaluation_result` is `Err(_)` → same structure but with
///   [`FailClosedReason::EvaluationFailed`].
/// * If both are healthy → the original [`DecisionResult`] is returned
///   unmodified; `fail_closed_reason` is `None`.
///
/// # Invariant 5
///
/// The returned [`DecisionResult::decision`] is **never** an error.  Block is
/// a compliance outcome and is returned as a plain value in all cases.
#[must_use]
pub fn fail_closed<E>(
    detector_output: Option<Vec<DetectorFinding>>,
    evaluation_result: Result<DecisionResult, E>,
) -> FailClosedResult {
    // Case 1: detector produced no output — missing input.
    if detector_output.is_none() {
        return FailClosedResult {
            inner: block_result(),
            fail_closed_reason: Some(FailClosedReason::DetectorOutputMissing),
        };
    }

    // Case 2: evaluation itself failed.
    match evaluation_result {
        Err(_) => FailClosedResult {
            inner: block_result(),
            fail_closed_reason: Some(FailClosedReason::EvaluationFailed),
        },
        // Case 3: healthy path — pass through unchanged.
        Ok(result) => FailClosedResult {
            inner: result,
            fail_closed_reason: None,
        },
    }
}

// ── FailClosedResult — the enriched return type ───────────────────────────────

/// The value returned by [`fail_closed`].
///
/// Wraps a [`DecisionResult`] and annotates it with the reason the fail-closed
/// path was activated, if it was.
#[derive(Debug, Clone)]
pub struct FailClosedResult {
    /// The final decision.  Always [`Decision::Block`] when
    /// `fail_closed_reason` is `Some(_)`.
    pub inner: DecisionResult,

    /// `Some(reason)` if the fail-closed path was taken; `None` if the healthy
    /// path was taken.
    pub fail_closed_reason: Option<FailClosedReason>,
}

impl FailClosedResult {
    /// Returns `true` if the fail-closed path was activated.
    #[must_use]
    pub fn was_fail_closed(&self) -> bool {
        self.fail_closed_reason.is_some()
    }

    /// Convenience accessor for the resolved [`Decision`].
    #[must_use]
    pub fn decision(&self) -> Decision {
        self.inner.decision
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Build the canonical fail-closed Block result.
///
/// `driving_findings` is empty because the caller never produced usable
/// findings — there is nothing to record.
fn block_result() -> DecisionResult {
    DecisionResult {
        decision: Decision::Block,
        reason: DecisionReason::BlockRequired,
        driving_findings: vec![],
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::decision::{
        apply_decision, evaluate_decision, DetectorFinding, FindingAction, Representation,
    };

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// A healthy evaluation over clean (Allow) findings.
    fn healthy_allow() -> Result<DecisionResult, String> {
        Ok(evaluate_decision(&[]))
    }

    /// A healthy evaluation that contains a Redact finding.
    fn healthy_redact() -> Result<DecisionResult, String> {
        let findings = vec![DetectorFinding::new(
            0..5,
            0.75,
            FindingAction::Redact,
            Representation::Raw,
        )];
        Ok(evaluate_decision(&findings))
    }

    /// A healthy evaluation that contains a Block finding.
    fn healthy_block() -> Result<DecisionResult, String> {
        let findings = vec![DetectorFinding::new(
            0..5,
            0.97,
            FindingAction::Block,
            Representation::Raw,
        )];
        Ok(evaluate_decision(&findings))
    }

    /// Simulates an evaluation error (e.g. internal panic recovery).
    fn failed_evaluation() -> Result<DecisionResult, String> {
        Err("detector evaluation panicked".to_string())
    }

    // ── Representative behaviour 1: missing detector output → Block ───────────

    /// T-DEC-3 / T-DEC-4 analogue: when the detector produces no output at all
    /// (None), the result must be Block.
    #[test]
    fn missing_detector_output_blocks() {
        // Arrange: detector returned nothing (timeout / missing response)
        let detector_output: Option<Vec<DetectorFinding>> = None;

        // Act
        let fc = fail_closed(detector_output, healthy_allow());

        // Assert
        assert_eq!(fc.decision(), Decision::Block);
        assert!(fc.was_fail_closed());
        assert_eq!(
            fc.fail_closed_reason,
            Some(FailClosedReason::DetectorOutputMissing)
        );
    }

    // ── Representative behaviour 2: invalid / failed evaluation → Block ───────

    /// When evaluation returns Err (malformed findings, internal failure), the
    /// result must be Block regardless of what the detector returned.
    #[test]
    fn failed_evaluation_blocks() {
        // Arrange: detector returned something, but evaluation failed
        let findings = vec![DetectorFinding::new(
            0..5,
            0.20,
            FindingAction::Allow,
            Representation::Raw,
        )];
        let detector_output = Some(findings);

        // Act
        let fc = fail_closed(detector_output, failed_evaluation());

        // Assert
        assert_eq!(fc.decision(), Decision::Block);
        assert!(fc.was_fail_closed());
        assert_eq!(
            fc.fail_closed_reason,
            Some(FailClosedReason::EvaluationFailed)
        );
    }

    // ── Representative behaviour 3: successful output is left unchanged ───────

    /// When both detector output and evaluation are healthy, the result passes
    /// through unmodified.
    #[test]
    fn successful_allow_passes_through_unchanged() {
        // Arrange: clean scan, evaluation succeeded with Allow
        let detector_output: Option<Vec<DetectorFinding>> = Some(vec![]);

        // Act
        let fc = fail_closed(detector_output, healthy_allow());

        // Assert
        assert_eq!(fc.decision(), Decision::Allow);
        assert!(!fc.was_fail_closed());
        assert_eq!(fc.fail_closed_reason, None);
    }

    /// Successful Redact result passes through unchanged.
    #[test]
    fn successful_redact_passes_through_unchanged() {
        // Arrange
        let findings = vec![DetectorFinding::new(
            0..5,
            0.75,
            FindingAction::Redact,
            Representation::Raw,
        )];
        let detector_output = Some(findings);

        // Act
        let fc = fail_closed(detector_output, healthy_redact());

        // Assert
        assert_eq!(fc.decision(), Decision::Redact);
        assert!(!fc.was_fail_closed());
        assert_eq!(fc.fail_closed_reason, None);
    }

    /// Successful Block result (from a genuine finding) passes through
    /// unchanged and is NOT mis-classified as a fail-closed event.
    #[test]
    fn successful_block_passes_through_unchanged() {
        // Arrange
        let findings = vec![DetectorFinding::new(
            0..5,
            0.97,
            FindingAction::Block,
            Representation::Raw,
        )];
        let detector_output = Some(findings);

        // Act
        let fc = fail_closed(detector_output, healthy_block());

        // Assert
        assert_eq!(fc.decision(), Decision::Block);
        assert!(
            !fc.was_fail_closed(),
            "a genuine Block finding is not a fail-closed event"
        );
        assert_eq!(fc.fail_closed_reason, None);
    }

    // ── Edge cases ────────────────────────────────────────────────────────────

    /// None input overrides even a successful evaluation — the detector output
    /// being absent is sufficient to block regardless of what evaluation would
    /// have returned.
    #[test]
    fn none_input_overrides_ok_evaluation() {
        // Arrange: evaluation would succeed with Allow, but no detector output
        let detector_output: Option<Vec<DetectorFinding>> = None;

        // Act
        let fc = fail_closed(detector_output, healthy_allow());

        // Assert: blocked, not allowed
        assert_eq!(fc.decision(), Decision::Block);
        assert_eq!(
            fc.fail_closed_reason,
            Some(FailClosedReason::DetectorOutputMissing)
        );
    }

    /// None input with a failed evaluation still blocks (and records the
    /// more fundamental reason: output was missing).
    #[test]
    fn none_input_with_failed_evaluation_still_blocks() {
        // Arrange
        let detector_output: Option<Vec<DetectorFinding>> = None;

        // Act
        let fc = fail_closed(detector_output, failed_evaluation());

        // Assert
        assert_eq!(fc.decision(), Decision::Block);
        // Missing output is detected first, so that reason is recorded.
        assert_eq!(
            fc.fail_closed_reason,
            Some(FailClosedReason::DetectorOutputMissing)
        );
    }

    /// The fail-closed Block result carries no driving findings — there are no
    /// valid findings to record when the detector failed.
    #[test]
    fn fail_closed_block_has_no_driving_findings() {
        // Arrange
        let detector_output: Option<Vec<DetectorFinding>> = None;

        // Act
        let fc = fail_closed(detector_output, healthy_allow());

        // Assert
        assert!(fc.inner.driving_findings.is_empty());
    }

    // ── Invariant 5 — Block is never an error ────────────────────────────────

    /// Even when triggered by a real failure, the returned value is a plain
    /// DecisionResult (Block), never an Err.
    #[test]
    fn fail_closed_result_is_never_an_error() {
        let detector_output: Option<Vec<DetectorFinding>> = None;
        let fc = fail_closed(detector_output, failed_evaluation());

        // This compiles only if FailClosedResult is a plain value, not Result<_>.
        let _ = match fc.decision() {
            Decision::Allow | Decision::Redact | Decision::Block => "info-level outcome",
        };
    }

    // ── T-DEC-3 / T-DEC-4 — scan_timeout_blocks_not_leaks / scanner_error_blocks_not_leaks ──

    #[test]
    fn scan_timeout_and_scanner_error_suppress_raw_text() {
        // Arrange: 1. Timeout (represented by detector_output = None)
        let raw_text = "sensitive information here";
        let fc_timeout = fail_closed(None, healthy_allow());

        // 2. Scanner error (represented by evaluation failure)
        let findings = vec![DetectorFinding::new(
            0..5,
            0.20,
            FindingAction::Allow,
            Representation::Raw,
        )];
        let fc_error = fail_closed(Some(findings), failed_evaluation());

        // Act
        let output_timeout = apply_decision(raw_text, &fc_timeout.inner);
        let output_error = apply_decision(raw_text, &fc_error.inner);

        // Assert: both outcomes yield completely suppressed (empty) output
        assert_eq!(output_timeout, "");
        assert_eq!(output_error, "");
    }
}
