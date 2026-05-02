//! SQLite-backed adaptive memory for recovered element selectors.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CREATE_ELEMENT_FINGERPRINTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS element_fingerprints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL,
    url_pattern TEXT NOT NULL,
    original_selector TEXT NOT NULL,
    element_tag TEXT NOT NULL,
    text_hash TEXT NOT NULL,
    dom_path TEXT NOT NULL,
    sibling_signature TEXT NOT NULL,
    bbox_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_used_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    successful_uses INTEGER NOT NULL DEFAULT 0 CHECK (successful_uses >= 0)
);
"#;

/// A fingerprint captured for an element that may need selector recovery later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementFingerprint {
    /// Domain where the element was observed.
    pub domain: String,
    /// URL pattern associated with this selector.
    pub url_pattern: String,
    /// Selector that originally matched the element.
    pub original_selector: String,
    /// Lowercase element tag name.
    pub element_tag: String,
    /// Stable hash of visible or identifying text.
    pub text_hash: String,
    /// Structural DOM path for the element.
    pub dom_path: String,
    /// Signature describing nearby siblings.
    pub sibling_signature: String,
    /// JSON-encoded bounding box metadata.
    pub bbox_json: String,
}

/// A persisted fingerprint returned from adaptive memory lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementFingerprintCandidate {
    /// Database row id.
    pub id: i64,
    /// Domain where the element was observed.
    pub domain: String,
    /// URL pattern associated with this selector.
    pub url_pattern: String,
    /// Selector that originally matched the element.
    pub original_selector: String,
    /// Lowercase element tag name.
    pub element_tag: String,
    /// Stable hash of visible or identifying text.
    pub text_hash: String,
    /// Structural DOM path for the element.
    pub dom_path: String,
    /// Signature describing nearby siblings.
    pub sibling_signature: String,
    /// JSON-encoded bounding box metadata.
    pub bbox_json: String,
    /// UTC creation timestamp.
    pub created_at: String,
    /// UTC timestamp when this fingerprint was last used.
    pub last_used_at: String,
    /// Count of successful recoveries using this fingerprint.
    pub successful_uses: u64,
}

/// SQLite-backed store for selector recovery fingerprints.
#[derive(Debug)]
pub struct AdaptiveMemory {
    conn: Connection,
}

impl AdaptiveMemory {
    /// Connect to a SQLite database at `path`.
    pub fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("failed to open adaptive memory database")?;
        Ok(Self { conn })
    }

    /// Create an in-memory database, primarily useful for tests.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .context("failed to open in-memory adaptive memory database")?;
        Ok(Self { conn })
    }

    /// Return the underlying SQLite connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Initialize the adaptive memory schema.
    pub fn init_db(&self) -> Result<()> {
        self.conn
            .execute_batch(CREATE_ELEMENT_FINGERPRINTS_TABLE)
            .context("failed to initialize adaptive memory schema")
    }

    /// Persist an element fingerprint and return its row id.
    pub fn save_fingerprint(&self, fingerprint: &ElementFingerprint) -> Result<i64> {
        self.conn
            .execute(
                r#"
                INSERT INTO element_fingerprints (
                    domain,
                    url_pattern,
                    original_selector,
                    element_tag,
                    text_hash,
                    dom_path,
                    sibling_signature,
                    bbox_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    fingerprint.domain,
                    fingerprint.url_pattern,
                    fingerprint.original_selector,
                    fingerprint.element_tag,
                    fingerprint.text_hash,
                    fingerprint.dom_path,
                    fingerprint.sibling_signature,
                    fingerprint.bbox_json,
                ],
            )
            .context("failed to save element fingerprint")?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Find prior fingerprints for a domain and URL pattern.
    pub fn find_candidates(
        &self,
        domain: &str,
        url_pattern: &str,
    ) -> Result<Vec<ElementFingerprintCandidate>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT
                    id,
                    domain,
                    url_pattern,
                    original_selector,
                    element_tag,
                    text_hash,
                    dom_path,
                    sibling_signature,
                    bbox_json,
                    created_at,
                    last_used_at,
                    successful_uses
                FROM element_fingerprints
                WHERE domain = ?1 AND url_pattern = ?2
                ORDER BY successful_uses DESC, last_used_at DESC, id DESC
                "#,
            )
            .context("failed to prepare adaptive memory candidate query")?;

        let rows = stmt
            .query_map(params![domain, url_pattern], |row| {
                let successful_uses = row.get::<_, i64>(11)?;

                Ok(ElementFingerprintCandidate {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    url_pattern: row.get(2)?,
                    original_selector: row.get(3)?,
                    element_tag: row.get(4)?,
                    text_hash: row.get(5)?,
                    dom_path: row.get(6)?,
                    sibling_signature: row.get(7)?,
                    bbox_json: row.get(8)?,
                    created_at: row.get(9)?,
                    last_used_at: row.get(10)?,
                    successful_uses: u64::try_from(successful_uses).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            11,
                            rusqlite::types::Type::Integer,
                            Box::new(error),
                        )
                    })?,
                })
            })
            .context("failed to query adaptive memory candidates")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read adaptive memory candidates")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_finds_fingerprint_candidates() {
        let memory = AdaptiveMemory::in_memory().unwrap();
        memory.init_db().unwrap();

        let row_id = memory
            .save_fingerprint(&ElementFingerprint {
                domain: "example.com".to_owned(),
                url_pattern: "/products/*".to_owned(),
                original_selector: ".buy-button".to_owned(),
                element_tag: "button".to_owned(),
                text_hash: "hash-123".to_owned(),
                dom_path: "html>body>main>button".to_owned(),
                sibling_signature: "button+a".to_owned(),
                bbox_json: r#"{"x":1,"y":2,"width":3,"height":4}"#.to_owned(),
            })
            .unwrap();

        let candidates = memory
            .find_candidates("example.com", "/products/*")
            .unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, row_id);
        assert_eq!(candidates[0].original_selector, ".buy-button");
        assert_eq!(candidates[0].successful_uses, 0);
    }
}
