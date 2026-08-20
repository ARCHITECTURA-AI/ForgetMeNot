//! # Stream Buffer — F14 / Buffer-Release Architecture
//!
//! Temporarily accumulates streamed model-output chunks in arrival order so
//! the Decision Layer can inspect the **complete buffered text** before any
//! content is forwarded to the client.
//!
//! ## The guarantee this module provides (T-STR-1)
//!
//! > No token reaches the client until `release()` is called by the caller
//! > **after** the scan and ledger-write are complete.
//!
//! The buffer itself is a passive data structure.  The no-race guarantee is
//! enforced by the caller (the pipeline / stream layer), which must not call
//! `release()` until it has:
//!
//! 1. Received all chunks from the model.
//! 2. Passed the assembled text to the decision engine.
//! 3. Received a decision that permits forwarding.
//! 4. Durably written the ledger event.
//!
//! ## What this module does NOT do
//!
//! * It does **not** communicate with OpenAI or any upstream.
//! * It does **not** perform network I/O of any kind.
//! * It does **not** inspect policy, perform redaction, or run detection.
//! * It does **not** implement any streaming protocol (HTTP chunked, SSE, etc.).
//!
//! Those responsibilities belong to the detection, decision, and ingress layers.

// ── StreamBuffer ──────────────────────────────────────────────────────────────

/// An ordered, in-memory buffer of streamed output chunks.
///
/// Chunks are stored in the exact order they were received from the upstream
/// model.  The buffer imposes no size limit — the caller is responsible for
/// enforcing any token-count ceiling before calling [`StreamBuffer::push_chunk`].
///
/// # Type parameter
///
/// `C` is the chunk type (typically `String` for text, or `bytes::Bytes` for
/// binary transports).  The only requirement is `Clone`, so that
/// [`StreamBuffer::release`] can return an independent copy without consuming
/// the buffer.
///
/// # Invariants
///
/// * Chunks are returned by [`release`] in the **same order** they were pushed.
/// * [`clear`] leaves the buffer in the same state as a freshly constructed one.
/// * [`release`] never removes chunks from the buffer — call [`clear`] after
///   forwarding if the chunks should be dropped.
#[derive(Debug, Clone)]
pub struct StreamBuffer<C> {
    chunks: Vec<C>,
}

impl<C: Clone> StreamBuffer<C> {
    /// Create an empty [`StreamBuffer`].
    #[must_use]
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Create an empty [`StreamBuffer`] with pre-allocated capacity.
    ///
    /// Use this when the expected chunk count is known in advance to avoid
    /// repeated re-allocation as chunks arrive.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity),
        }
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Append one chunk to the end of the buffer.
    ///
    /// Chunks are stored in arrival order.  There is no deduplication or
    /// ordering applied — the caller determines when and what to push.
    pub fn push_chunk(&mut self, chunk: C) {
        self.chunks.push(chunk);
    }

    /// Clear all buffered chunks, restoring the buffer to an empty state.
    ///
    /// Typically called after a [`Decision::Block`] (drop everything) or after
    /// a successful [`release`] + forward cycle when the chunks are no longer
    /// needed.
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    /// Return a clone of all buffered chunks in their original push order.
    ///
    /// This does **not** remove the chunks from the buffer.  The buffer remains
    /// unchanged after this call.  Callers that want to free memory after
    /// forwarding should follow `release()` with [`clear`].
    ///
    /// Returning a clone rather than draining the buffer allows the caller to:
    /// * Forward the cloned chunks to the client, then
    /// * Independently decide whether to `clear()` or inspect the buffer again.
    #[must_use]
    pub fn release(&self) -> Vec<C> {
        self.chunks.clone()
    }

    /// Returns `true` if no chunks have been pushed, or if [`clear`] was
    /// called after the last push.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Returns the number of chunks currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Borrow the buffered chunks as a slice without cloning.
    ///
    /// Useful for inspection operations (e.g. passing to a scanner) that do
    /// not need ownership of the data.
    #[must_use]
    pub fn as_slice(&self) -> &[C] {
        &self.chunks
    }
}

// ── Specialisation for String chunks ─────────────────────────────────────────

impl StreamBuffer<String> {
    /// Join all buffered string chunks into a single `String` in push order.
    ///
    /// This produces the complete assembled text that the Decision Layer will
    /// scan.  The buffer is not modified.
    ///
    /// Allocation note: a new `String` is allocated on every call.  Cache the
    /// result if multiple layers need to scan the same text in one pipeline
    /// pass.
    #[must_use]
    pub fn assembled(&self) -> String {
        self.chunks.concat()
    }
}

// ── Default ───────────────────────────────────────────────────────────────────

impl<C: Clone> Default for StreamBuffer<C> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Representative behaviour 1: chunks are buffered in order ─────────────

    /// Pushing multiple chunks preserves arrival order; `len()` and
    /// `is_empty()` reflect the buffer state correctly.
    #[test]
    fn chunks_are_buffered_in_push_order() {
        // Arrange
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        assert!(buf.is_empty());

        // Act
        buf.push_chunk("Hello".to_string());
        buf.push_chunk(", ".to_string());
        buf.push_chunk("world".to_string());
        buf.push_chunk("!".to_string());

        // Assert
        assert!(!buf.is_empty());
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.as_slice(), ["Hello", ", ", "world", "!"]);
    }

    /// A single-chunk buffer correctly stores and reports that chunk.
    #[test]
    fn single_chunk_is_stored() {
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("token".to_string());

        assert_eq!(buf.len(), 1);
        assert_eq!(buf.as_slice()[0], "token");
    }

    // ── Representative behaviour 2: release returns chunks in order ───────────

    /// `release()` returns all chunks in the exact order they were pushed,
    /// and does not mutate the buffer.
    #[test]
    fn release_returns_all_chunks_in_original_order() {
        // Arrange
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("alpha".to_string());
        buf.push_chunk("beta".to_string());
        buf.push_chunk("gamma".to_string());

        // Act
        let released = buf.release();

        // Assert — correct order
        assert_eq!(released, vec!["alpha", "beta", "gamma"]);

        // Assert — buffer is unchanged after release
        assert_eq!(buf.len(), 3, "buffer must not be drained by release()");
    }

    /// `release()` on an empty buffer returns an empty Vec, not a panic.
    #[test]
    fn release_on_empty_buffer_returns_empty_vec() {
        let buf: StreamBuffer<String> = StreamBuffer::new();
        let released = buf.release();
        assert!(released.is_empty());
    }

    /// `release()` followed by `release()` returns the same chunks twice —
    /// the buffer is idempotent until `clear()` is called.
    #[test]
    fn release_is_idempotent_without_clear() {
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("x".to_string());

        let first = buf.release();
        let second = buf.release();

        assert_eq!(first, second);
        assert_eq!(buf.len(), 1, "buffer still intact after two releases");
    }

    // ── Representative behaviour 3: clear empties the buffer ─────────────────

    /// `clear()` removes all buffered chunks; the buffer behaves as if freshly
    /// constructed.
    #[test]
    fn clear_empties_the_buffer() {
        // Arrange
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("a".to_string());
        buf.push_chunk("b".to_string());
        buf.push_chunk("c".to_string());
        assert_eq!(buf.len(), 3);

        // Act
        buf.clear();

        // Assert
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.release(), Vec::<String>::new());
    }

    /// `clear()` on an already-empty buffer is a no-op (no panic).
    #[test]
    fn clear_on_empty_buffer_is_safe() {
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.clear(); // must not panic
        assert!(buf.is_empty());
    }

    /// After `clear()`, the buffer can accept new chunks as normal.
    #[test]
    fn push_after_clear_works_correctly() {
        // Arrange
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("old".to_string());
        buf.clear();

        // Act
        buf.push_chunk("new".to_string());

        // Assert — only the post-clear chunk is present
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.as_slice(), ["new"]);
    }

    // ── T-STR-3: clean answer releases in full ────────────────────────────────

    /// After the decision layer returns Allow, `release()` delivers every
    /// chunk that was pushed — nothing is silently dropped.
    #[test]
    fn allow_path_releases_all_chunks_in_full() {
        // Arrange: simulate a clean LLM answer arriving as 5 chunks
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        let answer_chunks = ["The ", "capital ", "of ", "France ", "is Paris."];
        for chunk in &answer_chunks {
            buf.push_chunk(chunk.to_string());
        }

        // Act: decision is Allow → release and forward
        let forwarded = buf.release();

        // Assert: every chunk forwarded, in order, nothing lost
        assert_eq!(forwarded.len(), 5);
        assert_eq!(forwarded.join(""), "The capital of France is Paris.");
    }

    // ── T-STR-4: blocked answer never emits raw chunks ────────────────────────

    /// After the decision layer returns Block, `clear()` is called so that
    /// `release()` on the cleared buffer yields nothing.  The raw tokens are
    /// never forwarded.
    #[test]
    fn block_path_clear_prevents_raw_chunk_release() {
        // Arrange: sensitive chunks in the buffer
        let mut buf: StreamBuffer<String> = StreamBuffer::new();
        buf.push_chunk("The CFO is john.smith@acme.com".to_string());
        buf.push_chunk(" and his salary is £250,000.".to_string());

        // Act: decision is Block → clear the buffer before any release
        buf.clear();

        // Assert: release now yields nothing — raw tokens never forwarded
        let forwarded = buf.release();
        assert!(forwarded.is_empty());
        assert!(!forwarded.iter().any(|c| c.contains("john.smith@acme.com")));
    }

    // ── assembled() — string specialisation ──────────────────────────────────

    /// `assembled()` concatenates all chunks into a single string for scanning.
    #[test]
    fn assembled_joins_chunks_for_scanning() {
        let mut buf = StreamBuffer::new();
        buf.push_chunk("Hello".to_string());
        buf.push_chunk(", ".to_string());
        buf.push_chunk("world".to_string());

        assert_eq!(buf.assembled(), "Hello, world");
    }

    /// `assembled()` on an empty buffer returns an empty string.
    #[test]
    fn assembled_on_empty_buffer_returns_empty_string() {
        let buf: StreamBuffer<String> = StreamBuffer::new();
        assert_eq!(buf.assembled(), "");
    }

    /// `assembled()` does not modify the buffer — chunks are still accessible
    /// individually after calling it.
    #[test]
    fn assembled_does_not_consume_buffer() {
        let mut buf = StreamBuffer::new();
        buf.push_chunk("a".to_string());
        buf.push_chunk("b".to_string());

        let _ = buf.assembled();

        // Buffer still intact
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.release(), vec!["a", "b"]);
    }

    // ── Generic use — non-String chunk type ───────────────────────────────────

    /// The buffer works correctly with any Clone type, not just String.
    #[test]
    fn buffer_works_with_byte_vec_chunks() {
        let mut buf: StreamBuffer<Vec<u8>> = StreamBuffer::new();
        buf.push_chunk(vec![1, 2, 3]);
        buf.push_chunk(vec![4, 5, 6]);

        let released = buf.release();
        assert_eq!(released, vec![vec![1u8, 2, 3], vec![4, 5, 6]]);
    }

    // ── with_capacity pre-allocation ──────────────────────────────────────────

    /// `with_capacity` constructs a usable buffer; pre-allocation does not
    /// affect observable behaviour.
    #[test]
    fn with_capacity_behaves_identically_to_new() {
        let mut buf: StreamBuffer<String> = StreamBuffer::with_capacity(50);
        assert!(buf.is_empty());

        buf.push_chunk("hello".to_string());
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.release(), vec!["hello"]);
    }

    // ── Default trait ─────────────────────────────────────────────────────────

    #[test]
    fn default_produces_empty_buffer() {
        let buf: StreamBuffer<String> = StreamBuffer::default();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    // ── Clone produces an independent copy ───────────────────────────────────

    /// Cloning the buffer produces an independent copy; mutations to the clone
    /// do not affect the original.
    #[test]
    fn cloned_buffer_is_independent() {
        let mut original: StreamBuffer<String> = StreamBuffer::new();
        original.push_chunk("first".to_string());

        let mut cloned = original.clone();
        cloned.push_chunk("second".to_string());

        // Original unchanged
        assert_eq!(original.len(), 1);
        // Clone has both chunks
        assert_eq!(cloned.len(), 2);
    }
}
