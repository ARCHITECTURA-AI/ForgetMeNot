//! # Registry CLI Commands — T-CLI-2 / T-REG-9 / F15 / F21
//!
//! Implements the CLI functionality behind:
//!
//! ```text
//! forgetmenot registry add  --request-id <ID> --jurisdiction <JUR> \
//!                           --name-hash <HASH> --org-hash <HASH>   \
//!                           --scope <SCOPE> [--alias-hashes <H>,…] \
//!                           [--requester-verified]
//! forgetmenot registry list
//! ```
//!
//! ## Responsibilities
//!
//! * Translate CLI arguments into [`RegistryEntry`] structs and delegate to
//!   the existing [`RegistryStore`].
//! * Return the new `entity_id` on a successful `add`.
//! * Return a `Vec<String>` of entity IDs (never plaintext names) on `list`
//!   (T-REG-9, F21).
//!
//! ## What this module does NOT do
//!
//! * It does **not** implement hashing, detection, scanning, or proxy logic.
//! * It does **not** create a new schema or storage abstraction.
//! * It does **not** perform authentication or tenant isolation.
//! * It does **not** open a network connection.

use crate::registry::store::{RegistryEntry, RegistryStore};
use chrono::Utc;
use uuid::Uuid;

// ── RegistryAddArgs ───────────────────────────────────────────────────────────

/// Arguments required to register a new entity.
///
/// The CLI layer accepts these as command-line flags and passes them here.
/// All identity fields are **pre-hashed by the caller** before reaching this
/// struct — the store never sees plaintext names or emails (T-REG-3, F21).
#[derive(Debug, Clone)]
pub struct RegistryAddArgs {
    /// Stable erasure-request identifier (e.g. `GDPR-2026-00441`).
    pub request_id: String,

    /// Jurisdiction code: `IN`, `EU`, `US`, etc. (T-REG-1 — required).
    pub jurisdiction: String,

    /// SHA-256 (or equivalent) hash of the canonical name — never plaintext
    /// (T-REG-3, F21).
    pub name_hash: String,

    /// Optional hashes of known aliases.
    pub alias_hashes: Vec<String>,

    /// Hash of the organisation name.
    pub org_hash: String,

    /// Comma-separated list of data-surface scopes, e.g. `"chat,logs"`
    /// (T-REG-2 — required, must be non-empty).
    pub scope: String,

    /// Whether the requester's identity was independently verified
    /// (T-REG-8 / B10).
    pub requester_identity_verified: bool,
}

// ── registry_add ──────────────────────────────────────────────────────────────

/// Insert a new entity into the registry and return the generated `entity_id`.
///
/// # Validation
///
/// * `args.jurisdiction` must be non-empty (T-REG-1).
/// * `args.scope` must be non-empty (T-REG-2).
///
/// # Returns
///
/// The `entity_id` string on success.
///
/// # Errors
///
/// Returns an error if validation fails or the underlying store write fails.
pub fn registry_add(store: &RegistryStore, args: RegistryAddArgs) -> anyhow::Result<String> {
    // T-REG-1 — jurisdiction is mandatory.
    if args.jurisdiction.trim().is_empty() {
        anyhow::bail!("jurisdiction is required (T-REG-1)");
    }
    // T-REG-2 — scope is mandatory.
    if args.scope.trim().is_empty() {
        anyhow::bail!("scope is required (T-REG-2)");
    }

    let entity_id = format!("ent-{}", Uuid::new_v4());
    let created_at = Utc::now().to_rfc3339();

    let entry = RegistryEntry {
        entity_id: entity_id.clone(),
        request_id: args.request_id,
        jurisdiction: args.jurisdiction,
        name_hash: args.name_hash,
        alias_hashes: args.alias_hashes,
        org_hash: args.org_hash,
        scope: args.scope,
        requester_identity_verified: args.requester_identity_verified,
        created_at,
        version: 1,
    };

    store.insert(&entry)?;
    Ok(entity_id)
}

// ── registry_list ─────────────────────────────────────────────────────────────

/// Return the `entity_id` of every registered entity.
///
/// Only entity IDs are returned — **no plaintext names, email addresses, or
/// other PII** (T-REG-9, F21).
///
/// # Errors
///
/// Returns an error if the underlying store read fails.
pub fn registry_list(store: &RegistryStore) -> anyhow::Result<Vec<String>> {
    let entries = store.list()?;
    let ids = entries.into_iter().map(|e| e.entity_id).collect();
    Ok(ids)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Open an isolated in-memory [`RegistryStore`] for each test.
    fn make_store() -> RegistryStore {
        let conn = Connection::open_in_memory().expect("in-memory db");
        RegistryStore::new(conn).expect("store init")
    }

    /// Minimal valid [`RegistryAddArgs`] for test fixtures.
    fn valid_args() -> RegistryAddArgs {
        RegistryAddArgs {
            request_id: "req-test-001".to_string(),
            jurisdiction: "IN".to_string(),
            name_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .to_string(),
            alias_hashes: vec![],
            org_hash: "org-hash-acme".to_string(),
            scope: "chat".to_string(),
            requester_identity_verified: true,
        }
    }

    // ── T-CLI-2 ───────────────────────────────────────────────────────────────
    //
    // "registry_add_round_trips": `registry add …` then `registry list`
    // shows the new entity ID.

    /// T-CLI-2 — `registry_add_round_trips`
    ///
    /// 1. Open an isolated in-memory store.
    /// 2. Call `registry_add` (equivalent of `forgetmenot registry add …`).
    /// 3. Call `registry_list` (equivalent of `forgetmenot registry list`).
    /// 4. Assert the returned entity ID appears in the list.
    #[test]
    fn registry_add_round_trips() {
        // Arrange
        let store = make_store();

        // Act — step 2: add an entity
        let entity_id =
            registry_add(&store, valid_args()).expect("registry_add must succeed with valid args");

        // Act — step 3: list entities
        let ids = registry_list(&store).expect("registry_list must succeed");

        // Assert — step 4: the entity ID round-trips
        assert!(
            ids.contains(&entity_id),
            "entity_id {entity_id:?} must appear in registry_list output; got: {ids:?}"
        );
    }

    // ── Additional tests ──────────────────────────────────────────────────────

    /// `registry_list` output must never contain plaintext names (T-REG-9,
    /// F21).  We verify by checking that none of the returned strings match
    /// the known plaintext used in the name_hash fixture.
    #[test]
    fn registry_list_shows_ids_not_names() {
        let store = make_store();
        let plaintext_name = "John Smith";

        let mut args = valid_args();
        // name_hash is a hash — but store plaintext in org_hash to prove only
        // entity_id surfaces.  The list output must not contain the name.
        args.name_hash = format!("hash-of-{plaintext_name}");

        registry_add(&store, args).unwrap();

        let ids = registry_list(&store).unwrap();
        for id in &ids {
            assert!(
                !id.contains(plaintext_name),
                "registry list must not surface plaintext names; got: {id:?}"
            );
        }
    }

    /// Adding two entities produces two distinct entity IDs; both appear in
    /// the list.
    #[test]
    fn two_entities_both_appear_in_list() {
        let store = make_store();

        let mut args1 = valid_args();
        args1.request_id = "req-001".to_string();

        let mut args2 = valid_args();
        args2.request_id = "req-002".to_string();
        args2.name_hash = "different-hash".to_string();

        let id1 = registry_add(&store, args1).unwrap();
        let id2 = registry_add(&store, args2).unwrap();

        assert_ne!(id1, id2, "each entity_id must be unique");

        let ids = registry_list(&store).unwrap();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert_eq!(ids.len(), 2);
    }

    /// `registry_list` returns an empty vec when the store is empty.
    #[test]
    fn list_on_empty_store_returns_empty_vec() {
        let store = make_store();
        let ids = registry_list(&store).unwrap();
        assert!(ids.is_empty());
    }

    /// Missing jurisdiction must produce an error (T-REG-1).
    #[test]
    fn add_without_jurisdiction_returns_error() {
        let store = make_store();
        let mut args = valid_args();
        args.jurisdiction = String::new();

        let result = registry_add(&store, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("jurisdiction is required"));
    }

    /// Missing scope must produce an error (T-REG-2).
    #[test]
    fn add_without_scope_returns_error() {
        let store = make_store();
        let mut args = valid_args();
        args.scope = String::new();

        let result = registry_add(&store, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("scope is required"));
    }

    /// `registry_add` returns the entity_id, which has the expected `ent-`
    /// prefix and a UUID suffix.
    #[test]
    fn returned_entity_id_has_ent_prefix() {
        let store = make_store();
        let id = registry_add(&store, valid_args()).unwrap();
        assert!(
            id.starts_with("ent-"),
            "entity_id must start with 'ent-'; got: {id:?}"
        );
        // UUID suffix must be present (length check: "ent-" + 36 UUID chars)
        assert_eq!(id.len(), 4 + 36, "unexpected entity_id length: {id:?}");
    }
}
