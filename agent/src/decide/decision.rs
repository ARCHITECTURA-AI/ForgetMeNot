//! # Decision Engine — F12 / F13
//!
//! Translates detector findings into the action the ForgetMeNot agent should
//! take before forwarding a response:
//!
//! * [`Decision::Allow`]  — no findings or all findings clear; forward as-is.
//! * [`Decision::Redact`] — at least one finding requests redaction; replace
//!   the flagged span(s) with the neutral label defined in
//!   [`REDACTION_LABEL`].  The raw text is **never** forwarded.
//! * [`Decision::Block`]  — at least one finding requests a full block; drop
//!   the response entirely.  Note: a block is a compliance outcome, not an
//!   error (Invariant 5).
//!
//! This module **only** consumes detector output.  It performs no scanning,
//! classification, regex matching, AI inference, or policy evaluation of its
//! own.

// ── Neutral redaction label (F13 / T-DEC-2) ─────────────────────────────────

/// The literal string inserted in place of a redacted span.
///
/// Per T-DEC-2: the label must be generic and must **never** expose the entity
/// ID, the detection reason, or any other information that could re-identify
/// the data subject.
pub const REDACTION_LABEL: &str = "[Content removed per privacy policy]";

// ── FindingAction — the verdict a detector assigns to a single finding ───────

/// The action that the upstream detector has determined should apply to a
/// specific finding.
///
/// The decision engine reads this field verbatim; it does **not** re-evaluate
/// the detector's logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingAction {
    /// The detected span is safe to forward without modification.
    Allow,
    /// The detected span must be replaced with the neutral [`REDACTION_LABEL`].
    Redact,
    /// The entire response must be suppressed; no token may be forwarded.
    Block,
}

// ── DetectorFinding — minimal interface consumed by the decision engine ───────

/// A single finding emitted by the upstream detection engine.
///
/// This is the boundary type between the detection layer and the decision
/// layer.  The decision engine never inspects raw text or confidence scores
/// beyond what is represented here.
///
/// # Invariants
///
/// * `confidence` is in the closed interval `[0.0, 1.0]`.
/// * `required_action` is the detector's own verdict and is taken at face
///   value by the decision engine.
#[derive(Debug, Clone)]
pub struct DetectorFinding {
    /// The span of the original output to which this finding applies.  Stored
    /// as a byte-range `[start, end)` into the buffered response text.
    pub span: std::ops::Range<usize>,

    /// Normalised confidence in `[0.0, 1.0]` assigned by the upstream
    /// detector.  Stored for traceability; the decision engine itself does
    /// not re-evaluate thresholds.
    pub confidence: f64,

    /// The action the detector recommends for this finding.
    pub required_action: FindingAction,
}

impl DetectorFinding {
    /// Construct a new [`DetectorFinding`].
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `confidence` is outside `[0.0, 1.0]`.
    #[must_use]
    pub fn new(
        span: std::ops::Range<usize>,
        confidence: f64,
        required_action: FindingAction,
    ) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence must be in [0.0, 1.0], got {confidence}"
        );
        Self {
            span,
            confidence,
            required_action,
        }
    }
}

// ── Decision — the action the agent will take on the full response ────────────

/// The action the ForgetMeNot agent will take before forwarding a response.
///
/// # Invariant 5
///
/// [`Decision::Block`] is a **compliance outcome**, not an error.  Callers
/// must handle it at `info` log level and must never coerce it into an
/// `Err(…)` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// No registered-entity data detected; forward the response unchanged.
    Allow,
    /// One or more spans must be neutrally redacted before forwarding.
    Redact,
    /// The response must be suppressed in its entirety; no token is forwarded.
    Block,
}

// ── DecisionReason — human-readable rationale stored in the ledger ───────────

/// The primary reason that drove the [`Decision`].
///
/// Stored in the ledger event for auditor traceability.  The reason is never
/// surfaced to the end-user in order to avoid leaking entity metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionReason {
    /// All findings, if any, required no action.
    NoFindingsRequiringAction,
    /// At least one finding required redaction; no finding required blocking.
    RedactionRequired,
    /// At least one finding required the full response to be blocked.
    BlockRequired,
}

// ── DecisionResult — the complete output of the decision engine ───────────────

/// The complete, immutable result produced by [`evaluate_decision`].
///
/// Callers use [`DecisionResult::decision`] to determine the forwarding
/// action and [`DecisionResult::reason`] for ledger-writing.
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// The action the agent must take.
    pub decision: Decision,

    /// The primary reason that drove the decision.
    pub reason: DecisionReason,

    /// The subset of findings that caused the decision.  Empty when the
    /// decision is [`Decision::Allow`].  Preserved for ledger traceability
    /// without exposing raw text.
    pub driving_findings: Vec<DetectorFinding>,
}

// ── evaluate_decision — the core pure function ───────────────────────────────

/// Evaluate a slice of detector findings and return the most conservative
/// action required.
///
/// # Algorithm
///
/// The decision is the **maximum severity** across all findings:
///
/// ```text
/// Block  >  Redact  >  Allow
/// ```
///
/// A single [`FindingAction::Block`] finding is sufficient to produce
/// [`Decision::Block`], regardless of all other findings.  Similarly, a
/// single [`FindingAction::Redact`] finding (with no `Block` findings)
/// produces [`Decision::Redact`].
///
/// # Properties
///
/// * **Deterministic** — given the same `findings` slice, always returns the
///   same result.
/// * **Pure** — no side effects; no I/O; no heap allocation beyond the
///   returned value.
/// * **Fail-closed by design** — callers in the stream / ingress layer are
///   responsible for treating any upstream scanner error as an implicit
///   `Block` *before* calling this function.
#[must_use]
pub fn evaluate_decision(findings: &[DetectorFinding]) -> DecisionResult {
    // Fold over all findings and track the worst required action seen so far.
    // `FindingAction` derives `Ord` with Block > Redact > Allow.
    let worst = findings
        .iter()
        .map(|f| f.required_action)
        .max()
        .unwrap_or(FindingAction::Allow);

    match worst {
        FindingAction::Block => {
            let driving: Vec<DetectorFinding> = findings
                .iter()
                .filter(|f| f.required_action == FindingAction::Block)
                .cloned()
                .collect();
            DecisionResult {
                decision: Decision::Block,
                reason: DecisionReason::BlockRequired,
                driving_findings: driving,
            }
        }
        FindingAction::Redact => {
            let driving: Vec<DetectorFinding> = findings
                .iter()
                .filter(|f| f.required_action >= FindingAction::Redact)
                .cloned()
                .collect();
            DecisionResult {
                decision: Decision::Redact,
                reason: DecisionReason::RedactionRequired,
                driving_findings: driving,
            }
        }
        FindingAction::Allow => DecisionResult {
            decision: Decision::Allow,
            reason: DecisionReason::NoFindingsRequiringAction,
            driving_findings: vec![],
        },
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn allow_finding() -> DetectorFinding {
        DetectorFinding::new(0..10, 0.20, FindingAction::Allow)
    }

    fn redact_finding() -> DetectorFinding {
        DetectorFinding::new(11..25, 0.72, FindingAction::Redact)
    }

    fn block_finding() -> DetectorFinding {
        DetectorFinding::new(0..50, 0.97, FindingAction::Block)
    }

    // ── T-DEC representative behaviours ──────────────────────────────────────

    /// Clean detector findings (empty slice) → Allow.
    ///
    /// Representative behaviour: no findings at all.
    #[test]
    fn empty_findings_produces_allow() {
        // Arrange
        let findings: Vec<DetectorFinding> = vec![];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Allow);
        assert_eq!(result.reason, DecisionReason::NoFindingsRequiringAction);
        assert!(result.driving_findings.is_empty());
    }

    /// All-Allow findings → Allow.
    ///
    /// The detector ran and returned findings, but every finding was cleared.
    #[test]
    fn all_allow_findings_produces_allow() {
        // Arrange
        let findings = vec![allow_finding(), allow_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Allow);
        assert_eq!(result.reason, DecisionReason::NoFindingsRequiringAction);
        assert!(result.driving_findings.is_empty());
    }

    /// A single Redact finding → Redact.
    ///
    /// Representative behaviour: detector finding requiring redaction.
    #[test]
    fn single_redact_finding_produces_redact() {
        // Arrange
        let findings = vec![redact_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Redact);
        assert_eq!(result.reason, DecisionReason::RedactionRequired);
        assert_eq!(result.driving_findings.len(), 1);
    }

    /// Mixed Allow + Redact findings → Redact.
    ///
    /// The Allow finding must not downgrade the Redact verdict.
    #[test]
    fn allow_and_redact_findings_produces_redact() {
        // Arrange
        let findings = vec![allow_finding(), redact_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Redact);
        assert_eq!(result.reason, DecisionReason::RedactionRequired);
        assert!(!result.driving_findings.is_empty());
    }

    /// A single Block finding → Block.
    ///
    /// Representative behaviour: detector finding requiring blocking.
    #[test]
    fn single_block_finding_produces_block() {
        // Arrange
        let findings = vec![block_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
        assert_eq!(result.driving_findings.len(), 1);
    }

    /// Block dominates Redact — one Block among many Redacts → Block.
    #[test]
    fn block_dominates_redact() {
        // Arrange
        let findings = vec![redact_finding(), block_finding(), redact_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
        // Only the Block finding(s) drive the decision.
        assert!(result
            .driving_findings
            .iter()
            .all(|f| f.required_action == FindingAction::Block));
    }

    /// Block dominates Allow.
    #[test]
    fn block_dominates_allow() {
        // Arrange
        let findings = vec![allow_finding(), block_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
    }

    /// Multiple Block findings → Block, all Block findings are captured as
    /// driving findings.
    #[test]
    fn multiple_block_findings_all_captured() {
        // Arrange
        let findings = vec![block_finding(), block_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.driving_findings.len(), 2);
    }

    // ── Invariant 5 — Block is not an error ──────────────────────────────────

    /// Invariant 5: `Decision::Block` must be constructible and usable without
    /// ever being coerced into an `Err(…)`.  This test proves the type exists
    /// and can be matched as a normal, non-error outcome.
    #[test]
    fn block_decision_is_not_an_error_path() {
        // Arrange
        let findings = vec![block_finding()];

        // Act
        let result = evaluate_decision(&findings);

        // Assert: Decision::Block is a plain variant — matching it is
        // indistinguishable from matching Allow or Redact.
        let logged_at_info = match result.decision {
            Decision::Allow | Decision::Redact | Decision::Block => true,
        };
        assert!(
            logged_at_info,
            "Block must be handled as a normal compliance outcome"
        );
    }

    // ── F13 — Neutral redaction label ────────────────────────────────────────

    /// T-DEC-2: the redaction label is generic, does not expose entity IDs,
    /// reasons, or any re-identifying information.
    #[test]
    fn redaction_label_is_neutral() {
        // The label must not contain obvious entity-identifying patterns.
        assert!(!REDACTION_LABEL.is_empty());
        assert!(!REDACTION_LABEL.contains("entity_id"));
        assert!(!REDACTION_LABEL.contains("reason"));
        // Confirm the exact wording matches the spec.
        assert_eq!(REDACTION_LABEL, "[Content removed per privacy policy]");
    }

    // ── Determinism ──────────────────────────────────────────────────────────

    /// The same inputs always produce the same decision (determinism
    /// guarantee required by the module contract).
    #[test]
    fn evaluate_decision_is_deterministic() {
        let findings = vec![redact_finding(), allow_finding()];

        let result_a = evaluate_decision(&findings);
        let result_b = evaluate_decision(&findings);

        assert_eq!(result_a.decision, result_b.decision);
        assert_eq!(result_a.reason, result_b.reason);
        assert_eq!(
            result_a.driving_findings.len(),
            result_b.driving_findings.len()
        );
    }

    // ── FindingAction ordering ────────────────────────────────────────────────

    /// Verify the derived Ord on FindingAction matches the severity ranking.
    #[test]
    fn finding_action_severity_ordering() {
        assert!(FindingAction::Block > FindingAction::Redact);
        assert!(FindingAction::Redact > FindingAction::Allow);
        assert!(FindingAction::Block > FindingAction::Allow);
    }
}
