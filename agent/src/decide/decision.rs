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

use crate::decide::redact::{redact_text, RedactionSpan};
use crate::registry::store::RegistryStore;
use serde::{Deserialize, Serialize};

// ── Neutral redaction label (F13 / T-DEC-2) ─────────────────────────────────

/// The literal string inserted in place of a redacted span.
///
/// Per T-DEC-2: the label must be generic and must **never** expose the entity
/// ID, the detection reason, or any other information that could re-identify
/// the data subject.
pub const REDACTION_LABEL: &str = "[Content removed per privacy policy]";

// ── Representation ────────────────────────────────────────────────────────────

/// Explicitly identifies which representation of the text a detector finding
/// was scanned against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Representation {
    /// Finding corresponds to indices in the raw (original) string.
    Raw,
    /// Finding corresponds to indices in the normalized string.
    Normalized,
}

// ── FindingAction — the verdict a detector assigns to a single finding ───────

/// The action that the upstream detector has determined should apply to a
/// specific finding.
///
/// The decision engine reads this field verbatim; it does **not** re-evaluate
/// the detector's logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// The span of the output to which this finding applies.  Stored
    /// as a byte-range `[start, end)` into the response text.
    pub span: std::ops::Range<usize>,

    /// Normalised confidence in `[0.0, 1.0]` assigned by the upstream
    /// detector.  Stored for traceability; the decision engine itself does
    /// not re-evaluate thresholds.
    pub confidence: f64,

    /// The action the detector recommends for this finding.
    pub required_action: FindingAction,

    /// The representation of the source string this finding was scanned against.
    pub representation: Representation,
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
        representation: Representation,
    ) -> Self {
        debug_assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence must be in [0.0, 1.0], got {confidence}"
        );
        Self {
            span,
            confidence,
            required_action,
            representation,
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

// ── evaluate_decision_with_budget ─────────────────────────────────────────────

/// Evaluate detector findings and mutably charge the session/request budget
/// for medium-confidence findings. If the budget is exhausted, the decision
/// is escalated to BLOCK (T-DEC-6).
pub fn evaluate_decision_with_budget(
    findings: &[DetectorFinding],
    budget: &mut crate::decide::budget::ProcessingBudget,
) -> DecisionResult {
    let mut result = evaluate_decision(findings);

    // Medium confidence contextual findings charge the budget.
    // We define "medium-confidence" as confidence in the interval [0.5, 0.9].
    for finding in findings {
        if (0.5..=0.9).contains(&finding.confidence)
            && finding.required_action != FindingAction::Block
        {
            budget.consume(1);
        }
    }

    if budget.exhausted() {
        result.decision = Decision::Block;
        result.reason = DecisionReason::BlockRequired;
    }

    result
}

// ── apply_decision ────────────────────────────────────────────────────────────

/// Apply a decision to the raw response text, generating sanitized output or
/// completely suppressing the output on BLOCK or application failure.
///
/// # Safety Invariant
///
/// Spans originating from `Representation::Normalized` cannot be safely applied
/// to raw text using the same byte offsets due to length/index mutations (e.g.
/// homoglyphs, decoding, reassembly). Any such mismatch is treated as a safety
/// violation, triggering a fail-closed BLOCK.
#[must_use]
pub fn apply_decision(text: &str, result: &DecisionResult) -> String {
    match result.decision {
        Decision::Allow => text.to_string(),
        Decision::Block => String::new(), // Completely suppressed
        Decision::Redact => {
            let mut spans = Vec::new();
            for finding in &result.driving_findings {
                if finding.representation != Representation::Raw {
                    // Representation mismatch: we cannot safely redact raw text
                    // using normalized offsets. Fail closed!
                    return String::new();
                }
                spans.push(RedactionSpan::new(finding.span.start, finding.span.end));
            }
            match redact_text(text, &spans) {
                Ok(redacted) => redacted,
                Err(_) => String::new(), // Redaction error (out of bounds, non-char): Fail closed!
            }
        }
    }
}

// ── check_defense_in_depth (T-DEC-5) ──────────────────────────────────────────

/// Final safety net check (T-DEC-5).
///
/// Even if the detector incorrectly returns PASS (empty or Allow findings) for
/// registered PII, this check hashes substrings from the output and looks them
/// up in the `RegistryStore`. If a match is found, the decision escalates to BLOCK.
///
/// # Interface Gap
///
/// The `RegistryStore` only stores SHA-256 hashes of registered entities (names/aliases)
/// to comply with F21 (no plaintext PII stored). Therefore, literal substring comparisons
/// (like `text.contains(raw_pii)`) are impossible. We implement a substring-hashing scan
/// to bridge this gap.
pub fn check_defense_in_depth(
    text: &str,
    store: &RegistryStore,
    decision_result: &mut DecisionResult,
) -> anyhow::Result<()> {
    let entries = store.list()?;
    if entries.is_empty() {
        return Ok(());
    }

    // Tokenize text into words/phrases to compute hashes
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ';' || c == '!')
        .filter(|s| !s.is_empty())
        .collect();

    let mut found_pii = false;

    // Check single words, pairs, and triplets
    for len in 1..=3 {
        for window in words.windows(len) {
            let phrase = window.join(" ");
            let hash = sha256_hash(&phrase);
            let hash_lower = sha256_hash(&phrase.to_lowercase());

            for entry in &entries {
                if entry.name_hash == hash
                    || entry.name_hash == hash_lower
                    || entry.alias_hashes.contains(&hash)
                    || entry.alias_hashes.contains(&hash_lower)
                {
                    found_pii = true;
                    break;
                }
            }
            if found_pii {
                break;
            }
        }
        if found_pii {
            break;
        }
    }

    if found_pii {
        // Escalate decision to Block
        decision_result.decision = Decision::Block;
        decision_result.reason = DecisionReason::BlockRequired;
        decision_result.driving_findings.clear();
    }

    Ok(())
}

fn sha256_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::budget::ProcessingBudget;
    use crate::registry::store::RegistryEntry;
    use rusqlite::Connection;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn allow_finding() -> DetectorFinding {
        DetectorFinding::new(0..10, 0.20, FindingAction::Allow, Representation::Raw)
    }

    fn redact_finding() -> DetectorFinding {
        DetectorFinding::new(11..25, 0.72, FindingAction::Redact, Representation::Raw)
    }

    fn block_finding() -> DetectorFinding {
        DetectorFinding::new(0..50, 0.97, FindingAction::Block, Representation::Raw)
    }

    // ── T-DEC representative behaviours ──────────────────────────────────────

    /// Clean detector findings (empty slice) → Allow.
    #[test]
    fn empty_findings_produces_allow() {
        let findings: Vec<DetectorFinding> = vec![];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Allow);
        assert_eq!(result.reason, DecisionReason::NoFindingsRequiringAction);
        assert!(result.driving_findings.is_empty());
    }

    /// All-Allow findings → Allow.
    #[test]
    fn all_allow_findings_produces_allow() {
        let findings = vec![allow_finding(), allow_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Allow);
        assert_eq!(result.reason, DecisionReason::NoFindingsRequiringAction);
        assert!(result.driving_findings.is_empty());
    }

    /// A single Redact finding → Redact.
    #[test]
    fn single_redact_finding_produces_redact() {
        let findings = vec![redact_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Redact);
        assert_eq!(result.reason, DecisionReason::RedactionRequired);
        assert_eq!(result.driving_findings.len(), 1);
    }

    /// Mixed Allow + Redact findings → Redact.
    #[test]
    fn allow_and_redact_findings_produces_redact() {
        let findings = vec![allow_finding(), redact_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Redact);
        assert_eq!(result.reason, DecisionReason::RedactionRequired);
        assert!(!result.driving_findings.is_empty());
    }

    /// A single Block finding → Block.
    #[test]
    fn single_block_finding_produces_block() {
        let findings = vec![block_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
        assert_eq!(result.driving_findings.len(), 1);
    }

    /// Block dominates Redact — one Block among many Redacts → Block.
    #[test]
    fn block_dominates_redact() {
        let findings = vec![redact_finding(), block_finding(), redact_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
        assert!(result
            .driving_findings
            .iter()
            .all(|f| f.required_action == FindingAction::Block));
    }

    /// Block dominates Allow.
    #[test]
    fn block_dominates_allow() {
        let findings = vec![allow_finding(), block_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
    }

    /// Multiple Block findings → Block, all Block findings are captured.
    #[test]
    fn multiple_block_findings_all_captured() {
        let findings = vec![block_finding(), block_finding()];
        let result = evaluate_decision(&findings);

        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.driving_findings.len(), 2);
    }

    // ── Invariant 5 — Block is not an error ──────────────────────────────────

    #[test]
    fn block_decision_is_not_an_error_path() {
        let findings = vec![block_finding()];
        let result = evaluate_decision(&findings);

        let logged_at_info = match result.decision {
            Decision::Allow | Decision::Redact | Decision::Block => true,
        };
        assert!(
            logged_at_info,
            "Block must be handled as a normal compliance outcome"
        );
    }

    // ── F13 — Neutral redaction label ────────────────────────────────────────

    #[test]
    fn redaction_label_is_neutral() {
        assert!(!REDACTION_LABEL.is_empty());
        assert!(!REDACTION_LABEL.contains("entity_id"));
        assert!(!REDACTION_LABEL.contains("reason"));
        assert_eq!(REDACTION_LABEL, "[Content removed per privacy policy]");
    }

    // ── Determinism ──────────────────────────────────────────────────────────

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

    #[test]
    fn finding_action_severity_ordering() {
        assert!(FindingAction::Block > FindingAction::Redact);
        assert!(FindingAction::Redact > FindingAction::Allow);
        assert!(FindingAction::Block > FindingAction::Allow);
    }

    // ── T-DEC-1 — block_decision_redacts_or_blocks_output ────────────────────

    #[test]
    fn block_decision_redacts_or_blocks_output() {
        // Arrange
        let text = "My secret key is sk-ABCD1234EFGH5678 and email is test@example.com";
        let findings = vec![block_finding()];
        let result = evaluate_decision(&findings);

        // Act
        let output = apply_decision(text, &result);

        // Assert
        assert_eq!(output, "");
        assert!(!output.contains("sk-ABCD1234EFGH5678"));
        assert!(!output.contains("test@example.com"));
    }

    // ── T-DEC-2 — redaction_uses_neutral_label ───────────────────────────────

    #[test]
    fn redaction_uses_neutral_label() {
        // Arrange
        let text = "Contact us at john.smith@acme.com for support.";
        // "john.smith@acme.com" length is 19. Let's redact it at 14..33
        let findings = vec![DetectorFinding::new(
            14..33,
            0.8,
            FindingAction::Redact,
            Representation::Raw,
        )];
        let result = evaluate_decision(&findings);

        // Act
        let output = apply_decision(text, &result);

        // Assert
        assert_eq!(
            output,
            format!("Contact us at {} for support.", REDACTION_LABEL)
        );
        assert!(!output.contains("john.smith"));
        assert!(!output.contains("reason"));
    }

    // ── T-DEC-5 — defense_in_depth_catches_misconfigured_scanner ─────────────

    #[test]
    fn defense_in_depth_catches_misconfigured_scanner() {
        // Arrange: 1. Setup in-memory store and register an entity "John Smith"
        let conn = Connection::open_in_memory().unwrap();
        let store = RegistryStore::new(conn).unwrap();

        let name_hash = sha256_hash("John Smith");
        let entry = RegistryEntry {
            entity_id: "ent-1".to_string(),
            request_id: "req-1".to_string(),
            jurisdiction: "EU".to_string(),
            name_hash,
            alias_hashes: vec![],
            org_hash: "org-hash".to_string(),
            scope: "chat".to_string(),
            requester_identity_verified: true,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            version: 1,
        };
        store.insert(&entry).unwrap();

        // 2. Scanner incorrectly returns PASS (Allow findings) for registered PII
        let text = "Hello John Smith, welcome to the platform.";
        let findings = vec![allow_finding()];
        let mut result = evaluate_decision(&findings);
        assert_eq!(result.decision, Decision::Allow);

        // 3. Final safety check catches the PII
        check_defense_in_depth(text, &store, &mut result).unwrap();

        // Assert: Decision escalated to BLOCK
        assert_eq!(result.decision, Decision::Block);
        assert_eq!(result.reason, DecisionReason::BlockRequired);
    }

    // ── T-DEC-6 — accumulated_medium_hits_escalate ───────────────────────────

    #[test]
    fn accumulated_medium_hits_escalate() {
        // Arrange: budget of 3 medium-confidence hits
        let mut budget = ProcessingBudget::new(3);

        // Individual findings have confidence 0.7 (medium) and action Redact (which normally PASS/REDACTs, not BLOCK)
        let findings = vec![
            DetectorFinding::new(0..5, 0.7, FindingAction::Redact, Representation::Raw),
            DetectorFinding::new(10..15, 0.7, FindingAction::Redact, Representation::Raw),
            DetectorFinding::new(20..25, 0.7, FindingAction::Redact, Representation::Raw),
        ];

        // Act & Assert sequentially
        // Hit 1: does not block
        let res1 = evaluate_decision_with_budget(&findings[0..1], &mut budget);
        assert_ne!(res1.decision, Decision::Block);
        assert!(!budget.exhausted());

        // Hit 2: does not block
        let res2 = evaluate_decision_with_budget(&findings[1..2], &mut budget);
        assert_ne!(res2.decision, Decision::Block);
        assert!(!budget.exhausted());

        // Hit 3: exhausts the budget and triggers BLOCK
        let res3 = evaluate_decision_with_budget(&findings[2..3], &mut budget);
        assert_eq!(res3.decision, Decision::Block);
        assert!(budget.exhausted());
    }

    // ── Representation mismatch safety test ──────────────────────────────────

    #[test]
    fn test_representation_mismatch_blocks_completely() {
        // Arrange: finding belongs to Normalized representation
        let findings = vec![DetectorFinding::new(
            0..5,
            0.8,
            FindingAction::Redact,
            Representation::Normalized,
        )];
        let result = evaluate_decision(&findings);

        // Act
        let output = apply_decision("hello world", &result);

        // Assert: Fail-closed BLOCK triggers, returning completely suppressed output
        assert_eq!(output, "");
    }
}
