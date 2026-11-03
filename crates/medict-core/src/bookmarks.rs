use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub word: String,
    pub dict_id: String,
    pub dict_name: String,
    pub notebook_id: String,
    pub saved_at: i64,
}

#[derive(Debug, Clone)]
pub struct ExportRow {
    pub word: String,
    pub dict_name: String,
    pub notebook_name: String,
    pub html: String,
}

pub struct BookmarkStore {
    connection: Connection,
}

impl BookmarkStore {
    pub fn open(config_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(config_dir)?;
        let path = config_dir.join("bookmarks.db");
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS notebooks (
                 id         TEXT    PRIMARY KEY,
                 name       TEXT    NOT NULL,
                 is_default INTEGER NOT NULL DEFAULT 0,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS bookmarks (
                 id           INTEGER PRIMARY KEY AUTOINCREMENT,
                 word         TEXT    NOT NULL,
                 dict_id      TEXT    NOT NULL,
                 dict_name    TEXT    NOT NULL,
                 notebook_id  TEXT    NOT NULL,
                 saved_at     INTEGER NOT NULL,
                 content_html TEXT    NOT NULL DEFAULT ''
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_bookmarks
                 ON bookmarks(word, dict_id, notebook_id);
             CREATE INDEX IF NOT EXISTS idx_bookmarks_nb
                 ON bookmarks(notebook_id);",
        )?;
        let store = Self { connection };
        store.ensure_default_notebook()?;
        Ok(store)
    }

    fn ensure_default_notebook(&self) -> Result<()> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM notebooks", [], |row| row.get(0))?;
        if count == 0 {
            let id = Uuid::new_v4().to_string();
            let created_at = now();
            self.connection.execute(
                "INSERT INTO notebooks(id, name, is_default, created_at)
                 VALUES(?1, ?2, 1, ?3)",
                params![id, "默认生词本", created_at],
            )?;
            return Ok(());
        }
        self.connection.execute(
            "UPDATE notebooks SET is_default = (id = COALESCE(
                 (SELECT id FROM notebooks WHERE is_default = 1 ORDER BY created_at LIMIT 1),
                 (SELECT id FROM notebooks ORDER BY created_at LIMIT 1)
             ))",
            [],
        )?;
        Ok(())
    }

    pub fn default_notebook_id(&self) -> Result<String> {
        let id = self
            .connection
            .query_row(
                "SELECT id FROM notebooks WHERE is_default = 1 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .or_else(|| {
                self.connection
                    .query_row(
                        "SELECT id FROM notebooks ORDER BY created_at LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
            })
            .unwrap_or_default();
        Ok(id)
    }

    pub fn add(
        &self,
        word: &str,
        dict_id: &str,
        dict_name: &str,
        notebook_id: &str,
        html: &str,
    ) -> Result<()> {
        let notebook_id = if notebook_id.is_empty() {
            self.default_notebook_id()?
        } else {
            notebook_id.to_string()
        };
        self.connection.execute(
            "INSERT OR IGNORE INTO bookmarks
             (word, dict_id, dict_name, notebook_id, saved_at, content_html)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![word, dict_id, dict_name, notebook_id, now(), html],
        )?;
        Ok(())
    }

    pub fn remove(&self, word: &str, dict_id: &str, notebook_id: &str) -> Result<()> {
        self.connection.execute(
            "DELETE FROM bookmarks
             WHERE word = ?1 AND dict_id = ?2 AND notebook_id = ?3",
            params![word, dict_id, notebook_id],
        )?;
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<Bookmark>> {
        let mut statement = self.connection.prepare(
            "SELECT word, dict_id, dict_name, notebook_id, saved_at
             FROM bookmarks ORDER BY saved_at DESC, id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Bookmark {
                word: row.get(0)?,
                dict_id: row.get(1)?,
                dict_name: row.get(2)?,
                notebook_id: row.get(3)?,
                saved_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn snapshot(&self, word: &str, dict_id: &str, notebook_id: &str) -> Result<String> {
        Ok(self
            .connection
            .query_row(
                "SELECT content_html FROM bookmarks
                 WHERE word = ?1 AND dict_id = ?2 AND notebook_id = ?3",
                params![word, dict_id, notebook_id],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_default())
    }

    pub fn notebooks(&self) -> Result<Vec<Notebook>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, is_default, created_at
             FROM notebooks ORDER BY is_default DESC, created_at ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Notebook {
                id: row.get(0)?,
                name: row.get(1)?,
                is_default: row.get::<_, i64>(2)? == 1,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn create_notebook(&self, name: &str) -> Result<Notebook> {
        let name = name.trim();
        let name = if name.is_empty() { "生词本" } else { name };
        let notebook = Notebook {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            is_default: false,
            created_at: now(),
        };
        self.connection.execute(
            "INSERT INTO notebooks(id, name, is_default, created_at)
             VALUES(?1, ?2, 0, ?3)",
            params![notebook.id, notebook.name, notebook.created_at],
        )?;
        Ok(notebook)
    }

    pub fn rename_notebook(&self, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(crate::error::MedictError::Message(
                "Notebook name cannot be empty.".to_string(),
            ));
        }
        let changed = self.connection.execute(
            "UPDATE notebooks SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if changed == 0 {
            return Err(crate::error::MedictError::Message(
                "Notebook does not exist.".to_string(),
            ));
        }
        Ok(())
    }

    pub fn set_default_notebook(&mut self, id: &str) -> Result<()> {
        let exists: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM notebooks WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(crate::error::MedictError::Message(
                "Notebook does not exist.".to_string(),
            ));
        }
        let transaction = self.connection.transaction()?;
        transaction.execute("UPDATE notebooks SET is_default = 0", [])?;
        transaction.execute(
            "UPDATE notebooks SET is_default = 1 WHERE id = ?1",
            params![id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_notebook(&mut self, id: &str) -> Result<()> {
        let is_default: Option<i64> = self
            .connection
            .query_row(
                "SELECT is_default FROM notebooks WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(is_default) = is_default else {
            return Err(crate::error::MedictError::Message(
                "Notebook does not exist.".to_string(),
            ));
        };
        if is_default == 1 {
            return Err(crate::error::MedictError::Message(
                "The default notebook cannot be deleted.".to_string(),
            ));
        }
        let default_id = self.default_notebook_id()?;
        if default_id.is_empty() {
            return Err(crate::error::MedictError::Message(
                "The default notebook was not found.".to_string(),
            ));
        }

        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE OR IGNORE bookmarks SET notebook_id = ?1 WHERE notebook_id = ?2",
            params![default_id, id],
        )?;
        transaction.execute(
            "DELETE FROM bookmarks WHERE notebook_id = ?1",
            params![id],
        )?;
        transaction.execute("DELETE FROM notebooks WHERE id = ?1", params![id])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn export_rows(&self, notebook_id: &str) -> Result<Vec<ExportRow>> {
        let query = if notebook_id.is_empty() {
            "SELECT b.word, b.dict_name, n.name, b.content_html
             FROM bookmarks b JOIN notebooks n ON n.id = b.notebook_id
             ORDER BY b.saved_at DESC, b.id DESC"
        } else {
            "SELECT b.word, b.dict_name, n.name, b.content_html
             FROM bookmarks b JOIN notebooks n ON n.id = b.notebook_id
             WHERE b.notebook_id = ?1
             ORDER BY b.saved_at DESC, b.id DESC"
        };
        let mut statement = self.connection.prepare(query)?;
        let collect = |row: &rusqlite::Row<'_>| {
            Ok(ExportRow {
                word: row.get(0)?,
                dict_name: row.get(1)?,
                notebook_name: row.get(2)?,
                html: row.get(3)?,
            })
        };
        let rows = if notebook_id.is_empty() {
            statement.query_map([], collect)?.collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            statement
                .query_map(params![notebook_id], collect)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn format_timestamp(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}
