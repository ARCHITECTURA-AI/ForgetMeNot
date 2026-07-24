//! # Neutral Redaction — F13 / T-DEC-2
//!
//! Replaces sensitive byte-spans in a buffered response with the neutral
//! placeholder defined in [`REDACTION_LABEL`].
//!
//! ## Responsibilities
//!
//! * Accept **pre-located** spans from the caller (the detector's output).
//! * Sort, merge overlapping spans, then perform a single left-to-right
//!   substitution pass.
//! * Return the redacted text, or the original text unchanged when no spans
//!   require redaction.
//!
//! ## What this module does NOT do
//!
//! * It does **not** scan text for sensitive patterns.
//! * It does **not** perform regex matching or AI inference.
//! * It does **not** write logs or touch storage.
//! * It does **not** know what entity a span belongs to — the label is always
//!   generic (T-DEC-2).

use crate::decide::decision::REDACTION_LABEL;

// ── RedactionSpan ─────────────────────────────────────────────────────────────

/// A half-open byte range `[start, end)` within the response text that the
/// detector has identified as requiring neutral redaction.
///
/// Byte indices must point to valid UTF-8 character boundaries.  If `start >=
/// end` the span is treated as empty and skipped silently.
///
/// # Source
///
/// Spans are produced by the upstream detector and threaded through
/// [`crate::decide::decision::DetectorFinding::span`].  This type is a
/// lightweight, redaction-specific projection of that range so that
/// `redact_text` has no compile-time dependency on the full
/// `DetectorFinding` struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionSpan {
    /// Inclusive start byte index.
    pub start: usize,
    /// Exclusive end byte index.
    pub end: usize,
}

impl RedactionSpan {
    /// Construct a [`RedactionSpan`] from a half-open byte range.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `start > end`.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "RedactionSpan: start ({start}) must be <= end ({end})");
        Self { start, end }
    }

    /// Returns `true` if the span covers zero bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl From<std::ops::Range<usize>> for RedactionSpan {
    fn from(r: std::ops::Range<usize>) -> Self {
        Self::new(r.start, r.end)
    }
}

// ── redact_text ───────────────────────────────────────────────────────────────

/// Replace every span in `spans` within `text` with [`REDACTION_LABEL`].
///
/// # Algorithm
///
/// 1. **Filter** empty spans (`start >= end`).
/// 2. **Sort** remaining spans by `start` byte position.
/// 3. **Merge** overlapping or adjacent spans into a minimal covering set.
///    This prevents double-replacement and index corruption when the detector
///    emits overlapping findings.
/// 4. **Substitute** in a single left-to-right pass: copy the gap before each
///    merged span verbatim, then append [`REDACTION_LABEL`].
/// 5. Copy any trailing text after the last span.
///
/// Span boundaries are **clamped** to the length of `text` before use, so an
/// out-of-range end byte never causes a panic — it simply extends to the end
/// of the string.  Start bytes beyond the text length produce an empty span
/// and are skipped.
///
/// # Returns
///
/// * The redacted string when at least one non-empty span is present.
/// * A clone of `text` when `spans` is empty or all spans are empty.
///
/// # Properties
///
/// * **Pure** — no side effects, no I/O.
/// * **Deterministic** — same inputs always produce the same output.
/// * **Label-neutral** — the replacement is always [`REDACTION_LABEL`];
///   no entity information is embedded (T-DEC-2).
#[must_use]
pub fn redact_text(text: &str, spans: &[RedactionSpan]) -> String {
    // ── Step 1: filter empty spans ───────────────────────────────────────────
    let mut active: Vec<RedactionSpan> = spans
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect();

    // Fast path: nothing to do.
    if active.is_empty() {
        return text.to_string();
    }

    // ── Step 2: sort by start position ───────────────────────────────────────
    active.sort_unstable_by_key(|s| s.start);

    // ── Step 3: merge overlapping / adjacent spans ────────────────────────────
    let merged = merge_spans(active, text.len());

    // ── Step 4 & 5: single substitution pass ─────────────────────────────────
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    for span in &merged {
        // Copy the gap between the previous span's end and this span's start.
        if cursor < span.start {
            out.push_str(&text[cursor..span.start]);
        }
        // Replace the sensitive span with the neutral label.
        out.push_str(REDACTION_LABEL);
        cursor = span.end;
    }

    // Copy any text following the last redacted span.
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }

    out
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Merge a **sorted** list of spans into a minimal non-overlapping set.
///
/// Spans are clamped to `[0, text_len]` before merging.
fn merge_spans(sorted: Vec<RedactionSpan>, text_len: usize) -> Vec<RedactionSpan> {
    let mut merged: Vec<RedactionSpan> = Vec::with_capacity(sorted.len());

    for span in sorted {
        // Clamp to valid range.
        let start = span.start.min(text_len);
        let end = span.end.min(text_len);

        if start >= end {
            continue; // became empty after clamping
        }

        if let Some(last) = merged.last_mut() {
            if start <= last.end {
                // Overlapping or adjacent — extend the current merged span.
                last.end = last.end.max(end);
            } else {
                merged.push(RedactionSpan { start, end });
            }
        } else {
            merged.push(RedactionSpan { start, end });
        }
    }

    merged
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Representative behaviour 1: redact an email span ─────────────────────

    /// The detector locates an email address; redact_text replaces exactly
    /// that span with the neutral label.
    #[test]
    fn redacts_email_span() {
        // Arrange
        let text = "Contact us at john.smith@acme.com for support.";
        //                        ^14              ^33
        let spans = vec![RedactionSpan::new(14, 33)];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(
            result,
            format!("Contact us at {} for support.", REDACTION_LABEL)
        );
        assert!(!result.contains("john.smith@acme.com"));
    }

    // ── Representative behaviour 2: redact an API key span ────────────────────

    /// The detector locates an embedded API key token; it is replaced without
    /// touching the surrounding prose.
    #[test]
    fn redacts_api_key_span() {
        // Arrange
        let text = "Use token sk-ABCD1234EFGH5678 to authenticate.";
        //                     ^10               ^29  ("sk-ABCD1234EFGH5678" = 19 bytes)
        let spans = vec![RedactionSpan::new(10, 29)];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(
            result,
            format!("Use token {} to authenticate.", REDACTION_LABEL)
        );
        assert!(!result.contains("sk-ABCD1234EFGH5678"));
    }

    // ── Representative behaviour 3: leave ordinary text unchanged ─────────────

    /// When no spans are provided, the text is returned verbatim.
    #[test]
    fn leaves_ordinary_text_unchanged() {
        // Arrange
        let text = "The weather in Paris is sunny today.";
        let spans: Vec<RedactionSpan> = vec![];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(result, text);
    }

    // ── Multiple non-overlapping spans ────────────────────────────────────────

    /// Two separate sensitive spans are both replaced; text between them is
    /// preserved.
    #[test]
    fn redacts_multiple_non_overlapping_spans() {
        // Arrange
        // "Name: Alice, Email: alice@example.com"
        //  0123456789...
        let text = "Name: Alice, Email: alice@example.com";
        let name_span = RedactionSpan::new(6, 11);   // "Alice"
        let email_span = RedactionSpan::new(20, 37); // "alice@example.com"
        let spans = vec![name_span, email_span];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        let expected = format!(
            "Name: {}, Email: {}",
            REDACTION_LABEL, REDACTION_LABEL
        );
        assert_eq!(result, expected);
        assert!(!result.contains("Alice"));
        assert!(!result.contains("alice@example.com"));
    }

    // ── Overlapping spans are merged ──────────────────────────────────────────

    /// When the detector emits two overlapping spans (e.g. from different
    /// engines), they are merged into one redaction — no double-label and no
    /// index corruption.
    #[test]
    fn overlapping_spans_are_merged_into_one_redaction() {
        // Arrange
        let text = "Secret: ABCDEF123456 is classified.";
        let span_a = RedactionSpan::new(8, 18); // "ABCDEF1234"
        let span_b = RedactionSpan::new(12, 20); // "EF123456"  (overlaps)
        let spans = vec![span_a, span_b];

        // Act
        let result = redact_text(text, &spans);

        // Assert: exactly one label, not two
        assert_eq!(result.matches(REDACTION_LABEL).count(), 1);
        assert!(!result.contains("ABCDEF123456"));
    }

    // ── Adjacent spans are merged ─────────────────────────────────────────────

    /// Adjacent (touching) spans are merged so only one label appears.
    #[test]
    fn adjacent_spans_are_merged() {
        // Arrange
        let text = "AAABBB rest";
        let span_a = RedactionSpan::new(0, 3); // "AAA"
        let span_b = RedactionSpan::new(3, 6); // "BBB" — touches span_a
        let spans = vec![span_a, span_b];

        // Act
        let result = redact_text(text, &spans);

        // Assert: one label covering "AAABBB"
        assert_eq!(result.matches(REDACTION_LABEL).count(), 1);
        assert_eq!(result, format!("{} rest", REDACTION_LABEL));
    }

    // ── Span covering the full text ───────────────────────────────────────────

    #[test]
    fn span_covering_entire_text_redacts_everything() {
        // Arrange
        let text = "completely sensitive";
        let spans = vec![RedactionSpan::new(0, text.len())];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(result, REDACTION_LABEL);
    }

    // ── Span at start of text ─────────────────────────────────────────────────

    #[test]
    fn span_at_start_of_text() {
        // Arrange
        let text = "secret tail";
        let spans = vec![RedactionSpan::new(0, 6)]; // "secret"

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(result, format!("{} tail", REDACTION_LABEL));
    }

    // ── Span at end of text ───────────────────────────────────────────────────

    #[test]
    fn span_at_end_of_text() {
        // Arrange
        let text = "head secret";
        let spans = vec![RedactionSpan::new(5, 11)]; // "secret"

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(result, format!("head {}", REDACTION_LABEL));
    }

    // ── Empty span is silently skipped ────────────────────────────────────────

    #[test]
    fn empty_span_is_skipped() {
        // Arrange
        let text = "no change";
        let spans = vec![RedactionSpan::new(3, 3)]; // zero-length

        // Act
        let result = redact_text(text, &spans);

        // Assert
        assert_eq!(result, text);
    }

    // ── Spans provided out of order are sorted correctly ─────────────────────

    #[test]
    fn out_of_order_spans_are_sorted_before_application() {
        // Arrange
        let text = "AAA middle BBB";
        let span_bbb = RedactionSpan::new(11, 14); // "BBB" — listed first
        let span_aaa = RedactionSpan::new(0, 3);   // "AAA" — listed second
        let spans = vec![span_bbb, span_aaa];

        // Act
        let result = redact_text(text, &spans);

        // Assert
        let expected = format!("{} middle {}", REDACTION_LABEL, REDACTION_LABEL);
        assert_eq!(result, expected);
    }

    // ── Out-of-range end byte is clamped, not panicked ────────────────────────

    #[test]
    fn out_of_range_end_is_clamped_to_text_length() {
        // Arrange
        let text = "hello";
        let spans = vec![RedactionSpan::new(3, 999)]; // end >> text.len()

        // Act
        let result = redact_text(text, &spans);

        // Assert: "lo" at [3..5] is redacted, "hel" is preserved
        assert_eq!(result, format!("hel{}", REDACTION_LABEL));
    }

    // ── Redaction label is always neutral ─────────────────────────────────────

    /// The replacement string is the canonical neutral label — never an entity
    /// ID, entity name, detection reason, or any identifying token.
    #[test]
    fn replacement_label_is_always_the_neutral_constant() {
        let text = "sensitive data here";
        let spans = vec![RedactionSpan::new(0, 9)]; // "sensitive"

        let result = redact_text(text, &spans);

        assert!(result.contains(REDACTION_LABEL));
        assert!(result.contains("[Content removed per privacy policy]"));
    }

    // ── Unicode: multi-byte characters ───────────────────────────────────────

    /// Ensure byte-range spans on ASCII-only regions work correctly even when
    /// the surrounding text contains multi-byte UTF-8 characters.
    #[test]
    fn redacts_correctly_when_surrounding_text_contains_unicode() {
        // "café secret café" — "café" is 5 bytes (c-a-f-é where é=2 bytes)
        // We redact the ASCII "secret" substring.
        let text = "caf\u{00e9} secret caf\u{00e9}";
        // "caf\u{00e9}" = 5 bytes, then " " = 1, so "secret" starts at byte 6
        let secret_start = 6usize;
        let secret_end = secret_start + "secret".len(); // 12
        let spans = vec![RedactionSpan::new(secret_start, secret_end)];

        let result = redact_text(text, &spans);

        assert!(!result.contains("secret"));
        assert!(result.contains(REDACTION_LABEL));
    }
}
