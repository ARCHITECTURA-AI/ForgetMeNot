use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq)]
pub struct RegistryEntry {
    pub entity_id: String,
    pub request_id: String,
    pub jurisdiction: String,
    pub name_hash: String,
    pub alias_hashes: Vec<String>,
    pub org_hash: String,
    pub scope: String,
    pub requester_identity_verified: bool,
    pub created_at: String,
    pub version: u32,
}

pub struct RegistryStore {
    conn: Connection,
}

impl RegistryStore {
    pub fn new(conn: Connection) -> anyhow::Result<Self>
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS registry (
                entity_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                jurisdiction TEXT NOT NULL,
                name_hash TEXT NOT NULL,
                alias_hashes TEXT NOT NULL,
                org_hash TEXT NOT NULL,
                scope TEXT NOT NULL,
                requester_identity_verified INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                version INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, entry: &RegistryEntry) -> anyhow::Result<()> {
        let alias_hashes_json = serde_json::to_string(&entry.alias_hashes)?;
        let verified = if entry.requester_identity_verified { 1 } else { 0 };

        self.conn.execute(
            "INSERT INTO registry (
                entity_id, request_id, jurisdiction, name_hash, alias_hashes,
                org_hash, scope, requester_identity_verified, created_at, version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.entity_id,
                entry.request_id,
                entry.jurisdiction,
                entry.name_hash,
                alias_hashes_json,
                entry.org_hash,
                entry.scope,
                verified,
                entry.created_at,
                entry.version,
            ],
        )?;

        Ok(())
    }

    pub fn get(&self, entity_id: &str) -> anyhow::Result<Option<RegistryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id, request_id, jurisdiction, name_hash, alias_hashes,
                    org_hash, scope, requester_identity_verified, created_at, version
             FROM registry WHERE entity_id = ?1",
        )?;

        let res = stmt.query_row([entity_id], |row| {
            let alias_hashes_str: String = row.get(4)?;
            let alias_hashes: Vec<String> = serde_json::from_str(&alias_hashes_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;

            let verified_int: i32 = row.get(7)?;

            Ok(RegistryEntry {
                entity_id: row.get(0)?,
                request_id: row.get(1)?,
                jurisdiction: row.get(2)?,
                name_hash: row.get(3)?,
                alias_hashes,
                org_hash: row.get(5)?,
                scope: row.get(6)?,
                requester_identity_verified: verified_int != 0,
                created_at: row.get(8)?,
                version: row.get(9)?,
            })
        }).optional()?;

        Ok(res)
    }

    pub fn list(&self) -> anyhow::Result<Vec<RegistryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT entity_id, request_id, jurisdiction, name_hash, alias_hashes,
                    org_hash, scope, requester_identity_verified, created_at, version
             FROM registry",
        )?;

        let rows = stmt.query_map([], |row| {
            let alias_hashes_str: String = row.get(4)?;
            let alias_hashes: Vec<String> = serde_json::from_str(&alias_hashes_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;

            let verified_int: i32 = row.get(7)?;

            Ok(RegistryEntry {
                entity_id: row.get(0)?,
                request_id: row.get(1)?,
                jurisdiction: row.get(2)?,
                name_hash: row.get(3)?,
                alias_hashes,
                org_hash: row.get(5)?,
                scope: row.get(6)?,
                requester_identity_verified: verified_int != 0,
                created_at: row.get(8)?,
                version: row.get(9)?,
            })
        })?;

        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_store() -> RegistryStore {
        let conn = Connection::open_in_memory().unwrap();
        RegistryStore::new(conn)
    }

    #[test]
    fn test_insert_and_get() {
        let store = setup_store();
        let entry = RegistryEntry {
            entity_id: "ent-1".to_string(),
            request_id: "req-1".to_string(),
            jurisdiction: "IN".to_string(),
            name_hash: "hash-1".to_string(),
            alias_hashes: vec!["alias-1".to_string(), "alias-2".to_string()],
            org_hash: "org-1".to_string(),
            scope: "chat,logs".to_string(),
            requester_identity_verified: true,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            version: 1,
        };

        store.insert(&entry).unwrap();

        let retrieved = store.get("ent-1").unwrap().unwrap();
        assert_eq!(retrieved, entry);
    }

    #[test]
    fn test_get_missing() {
        let store = setup_store();
        let retrieved = store.get("missing").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_entries() {
        let store = setup_store();
        let entry1 = RegistryEntry {
            entity_id: "ent-1".to_string(),
            request_id: "req-1".to_string(),
            jurisdiction: "IN".to_string(),
            name_hash: "hash-1".to_string(),
            alias_hashes: vec!["alias-1".to_string()],
            org_hash: "org-1".to_string(),
            scope: "chat".to_string(),
            requester_identity_verified: true,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            version: 1,
        };
        let entry2 = RegistryEntry {
            entity_id: "ent-2".to_string(),
            request_id: "req-2".to_string(),
            jurisdiction: "EU".to_string(),
            name_hash: "hash-2".to_string(),
            alias_hashes: vec!["alias-2".to_string()],
            org_hash: "org-2".to_string(),
            scope: "logs".to_string(),
            requester_identity_verified: false,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            version: 2,
        };

        store.insert(&entry1).unwrap();
        store.insert(&entry2).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&entry1));
        assert!(list.contains(&entry2));
    }

    #[test]
    fn test_alias_hashes_serialization() {
        let store = setup_store();
        let entry = RegistryEntry {
            entity_id: "ent-1".to_string(),
            request_id: "req-1".to_string(),
            jurisdiction: "IN".to_string(),
            name_hash: "hash-1".to_string(),
            alias_hashes: vec!["alias-1".to_string(), "alias-2".to_string(), "alias-3".to_string()],
            org_hash: "org-1".to_string(),
            scope: "chat".to_string(),
            requester_identity_verified: true,
            created_at: "2026-06-24T00:00:00Z".to_string(),
            version: 1,
        };

        store.insert(&entry).unwrap();
        let retrieved = store.get("ent-1").unwrap().unwrap();
        assert_eq!(retrieved.alias_hashes, vec!["alias-1".to_string(), "alias-2".to_string(), "alias-3".to_string()]);
    }
}
