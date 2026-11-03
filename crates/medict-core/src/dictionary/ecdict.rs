use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use html_escape::encode_text;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{MedictError, Result};

const FULL_SOURCE_URL: &str = "https://raw.githubusercontent.com/skywind3000/ECDICT/master/ecdict.csv";
const MINIMUM_FULL_ROWS: i64 = 700_000;

pub struct EcdictBackend {
    path: PathBuf,
    connection: Connection,
}

impl EcdictBackend {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = open_database(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn status(&self) -> Result<(String, i64)> {
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM ecdict", [], |row| row.get(0))?;
        let has_metadata: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'medict_meta'",
            [],
            |row| row.get(0),
        )?;
        let edition = if has_metadata > 0 {
            self.connection
                .query_row(
                    "SELECT value FROM medict_meta WHERE key = 'edition'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_else(|| "compact".to_string())
        } else {
            "compact".to_string()
        };
        Ok((edition, count))
    }

    pub fn find_key(&mut self, word: &str) -> Result<Option<String>> {
        for candidate in lookup_candidates(word) {
            if let Some(found) = self
                .connection
                .query_row(
                    "SELECT word FROM ecdict WHERE word = ?1",
                    params![candidate],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(Some(found));
            }
        }
        let prefix = self.suggestions(word, 1)?;
        if let Some(found) = prefix.into_iter().next() {
            return Ok(Some(found));
        }
        self.fuzzy_key(word)
    }

    pub fn definition(&mut self, key: &str) -> Result<Option<String>> {
        let Some(found) = self.find_exact(key)? else {
            return Ok(None);
        };
        Ok(Some(render_card(
            &found.word,
            &found.phonetic,
            &found.definition,
            &found.translation,
        )))
    }

    pub fn suggestions(&mut self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        let mut prefixes = vec![prefix.trim().to_string()];
        for candidate in lemma_candidates(prefix) {
            if !prefixes.contains(&candidate) {
                prefixes.push(candidate);
            }
        }
        for candidate in prefixes {
            if candidate.is_empty() {
                continue;
            }
            let lower = candidate.clone();
            let upper = format!("{candidate}\u{10ffff}");
            let mut statement = self.connection.prepare(
                "SELECT word FROM ecdict
                 WHERE word >= ?1 AND word < ?2
                 ORDER BY frq ASC LIMIT ?3",
            )?;
            let rows = statement.query_map(params![lower, upper, limit as i64], |row| {
                row.get::<_, String>(0)
            })?;
            let words = rows.collect::<rusqlite::Result<Vec<_>>>()?;
            if !words.is_empty() {
                return Ok(dedupe(words, limit));
            }
        }
        Ok(Vec::new())
    }

    pub fn build_index(&mut self) -> Result<()> {
        self.connection
            .execute("CREATE INDEX IF NOT EXISTS idx_ecdict_word ON ecdict(word)", [])?;
        Ok(())
    }

    pub fn base_css(&mut self) -> String {
        String::new()
    }

    pub fn install_full(&mut self) -> Result<i64> {
        let response = ureq::builder()
            .timeout(Duration::from_secs(900))
            .build()
            .get(FULL_SOURCE_URL)
            .set("User-Agent", "Medict/5 ECDICT installer")
            .call()?;
        let reader = response.into_reader();
        let temporary = self.path.with_extension("db.download");
        let backup = self.path.with_extension("db.compact-backup");
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_file(&backup);

        let count = build_database_from_csv(reader, &temporary)?;
        if count < MINIMUM_FULL_ROWS {
            let _ = fs::remove_file(&temporary);
            return Err(MedictError::Message(format!(
                "The ECDICT download contained {count} usable rows; at least {MINIMUM_FULL_ROWS} are required."
            )));
        }

        let old = std::mem::replace(
            &mut self.connection,
            Connection::open_in_memory().map_err(MedictError::from)?,
        );
        drop(old);
        fs::rename(&self.path, &backup)?;
        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _ = fs::rename(&backup, &self.path);
            self.connection = open_database(&self.path)?;
            return Err(error.into());
        }

        match open_database(&self.path) {
            Ok(connection) => {
                self.connection = connection;
                let _ = fs::remove_file(backup);
                Ok(count)
            }
            Err(error) => {
                let _ = fs::remove_file(&self.path);
                let _ = fs::rename(&backup, &self.path);
                self.connection = open_database(&self.path)?;
                Err(error)
            }
        }
    }

    fn find_exact(&self, word: &str) -> Result<Option<EcdictRow>> {
        for candidate in lookup_candidates(word) {
            let row = self
                .connection
                .query_row(
                    "SELECT word, phonetic, definition, translation
                     FROM ecdict WHERE word = ?1",
                    params![candidate],
                    |row| {
                        Ok(EcdictRow {
                            word: row.get(0)?,
                            phonetic: row.get(1)?,
                            definition: row.get(2)?,
                            translation: row.get(3)?,
                        })
                    },
                )
                .optional()?;
            if row.is_some() {
                return Ok(row);
            }
        }
        Ok(None)
    }

    fn fuzzy_key(&mut self, word: &str) -> Result<Option<String>> {
        let needle = word.to_lowercase();
        let Some(first) = needle.chars().next() else {
            return Ok(None);
        };
        let pattern = format!("{}%", escape_like(&first.to_string()));
        let mut statement = self.connection.prepare(
            "SELECT word FROM ecdict
             WHERE word LIKE ?1 ESCAPE '\\'
             ORDER BY frq ASC LIMIT 5000",
        )?;
        let rows = statement.query_map(params![pattern], |row| row.get::<_, String>(0))?;
        let mut best: Option<(usize, String)> = None;
        for row in rows {
            let candidate = row?;
            if candidate.len().abs_diff(needle.len()) > 2 {
                continue;
            }
            let distance = strsim::levenshtein(&needle, &candidate.to_lowercase());
            if distance <= 2
                && best
                    .as_ref()
                    .is_none_or(|(best_distance, _)| distance < *best_distance)
            {
                best = Some((distance, candidate));
            }
        }
        Ok(best.map(|(_, word)| word))
    }
}

struct EcdictRow {
    word: String,
    phonetic: String,
    definition: String,
    translation: String,
}

fn open_database(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "case_sensitive_like", "ON")?;
    let count: i64 =
        connection.query_row("SELECT COUNT(*) FROM ecdict", [], |row| row.get(0))?;
    if count == 0 {
        return Err(MedictError::Message(
            "ECDICT database is empty.".to_string(),
        ));
    }
    Ok(connection)
}

fn build_database_from_csv(source: impl std::io::Read, database_path: &Path) -> Result<i64> {
    let _ = fs::remove_file(database_path);
    let connection = Connection::open(database_path)?;
    connection.execute_batch(
        "CREATE TABLE ecdict (
             word TEXT PRIMARY KEY,
             phonetic TEXT,
             definition TEXT,
             translation TEXT,
             pos TEXT,
             frq INTEGER
         );
         CREATE TABLE medict_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(source);
    let headers = reader.headers()?.clone();
    let column = |name: &str| {
        headers
            .iter()
            .position(|value| value.trim() == name)
            .ok_or_else(|| MedictError::Message(format!("ECDICT is missing the {name} column.")))
    };
    let word_index = column("word")?;
    let phonetic_index = column("phonetic")?;
    let definition_index = column("definition")?;
    let translation_index = column("translation")?;
    let pos_index = column("pos")?;
    let frequency_index = column("frq")?;

    let transaction = connection.unchecked_transaction()?;
    {
        let mut insert = transaction.prepare(
            "INSERT OR REPLACE INTO ecdict(word, phonetic, definition, translation, pos, frq)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for record in reader.records() {
            let record = record.map_err(|error| MedictError::Message(error.to_string()))?;
            let value = |index: usize| record.get(index).unwrap_or_default().trim();
            let word = value(word_index);
            let translation = value(translation_index);
            if word.is_empty() || translation.is_empty() {
                continue;
            }
            let frequency = value(frequency_index).parse::<i64>().unwrap_or_default();
            insert.execute(params![
                word,
                value(phonetic_index),
                value(definition_index),
                translation,
                value(pos_index),
                frequency,
            ])?;
        }
    }
    let count: i64 =
        transaction.query_row("SELECT COUNT(*) FROM ecdict", [], |row| row.get(0))?;
    transaction.execute(
        "INSERT INTO medict_meta(key, value) VALUES
         ('edition', 'full'),
         ('entry_count', ?1),
         ('source_url', ?2),
         ('license', 'MIT'),
         ('copyright', 'Copyright (c) 2025 Linwei')",
        params![count, FULL_SOURCE_URL],
    )?;
    transaction.commit()?;
    connection.execute_batch("PRAGMA optimize;")?;
    Ok(count)
}

fn render_card(word: &str, phonetic: &str, definition: &str, translation: &str) -> String {
    let mut html = String::from(
        "<style>
         .ec-card{max-width:46rem;padding:.25rem .1rem;line-height:1.65}
         .ec-head{display:flex;align-items:baseline;gap:.6rem;flex-wrap:wrap;margin-bottom:.3rem}
         .ec-word{font-size:1.75rem;font-weight:700}
         .ec-phon{color:#2a7de1;font-size:1.05rem}
         .ec-sec-h{font-size:.8rem;opacity:.65;font-weight:600;margin:.9rem 0 .3rem;
                   border-bottom:1px solid color-mix(in srgb,currentColor 18%,transparent);
                   padding-bottom:.15rem}
         .ec-trans{font-size:1.05rem}.ec-def{font-size:.95rem;opacity:.82}
         </style><div class=\"ec-card\"><div class=\"ec-head\">",
    );
    html.push_str(&format!(
        "<span class=\"ec-word\">{}</span>",
        encode_text(word)
    ));
    if !phonetic.trim().is_empty() {
        html.push_str(&format!(
            "<span class=\"ec-phon\">/{}/</span>",
            encode_text(phonetic.trim())
        ));
    }
    html.push_str("</div>");
    if !translation.trim().is_empty() {
        html.push_str("<div class=\"ec-sec-h\">释义</div><div class=\"ec-trans\">");
        html.push_str(&line_breaks(translation));
        html.push_str("</div>");
    }
    if !definition.trim().is_empty() {
        html.push_str("<div class=\"ec-sec-h\">English</div><div class=\"ec-def\">");
        html.push_str(&line_breaks(definition));
        html.push_str("</div>");
    }
    html.push_str("</div>");
    html
}

fn line_breaks(value: &str) -> String {
    encode_text(value)
        .replace("\\n", "<br>")
        .replace('\n', "<br>")
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn dedupe(values: Vec<String>, limit: usize) -> Vec<String> {
    crate::util::dedupe_strings(values, limit)
}

fn lookup_candidates(word: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |value: String| {
        if !value.is_empty() && seen.insert(value.clone()) {
            values.push(value);
        }
    };
    let trimmed = word.trim();
    add(trimmed.to_string());
    add(trimmed.to_lowercase());
    for candidate in lemma_candidates(trimmed) {
        add(candidate);
    }
    values
}

fn lemma_candidates(word: &str) -> Vec<String> {
    let word = word.to_lowercase();
    if !word.bytes().all(|value| value.is_ascii_alphabetic()) {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |value: String| {
        if value.len() >= 2 && seen.insert(value.clone()) {
            result.push(value);
        }
    };

    let irregular = [
        ("ran", "run"),
        ("went", "go"),
        ("gone", "go"),
        ("made", "make"),
        ("said", "say"),
        ("told", "tell"),
        ("took", "take"),
        ("taken", "take"),
        ("saw", "see"),
        ("seen", "see"),
        ("came", "come"),
        ("got", "get"),
        ("gotten", "get"),
        ("gave", "give"),
        ("given", "give"),
        ("found", "find"),
        ("thought", "think"),
        ("felt", "feel"),
        ("kept", "keep"),
        ("held", "hold"),
        ("brought", "bring"),
        ("bought", "buy"),
        ("taught", "teach"),
        ("caught", "catch"),
        ("sat", "sit"),
        ("stood", "stand"),
        ("slept", "sleep"),
        ("met", "meet"),
        ("paid", "pay"),
        ("laid", "lay"),
        ("lost", "lose"),
        ("sent", "send"),
        ("built", "build"),
        ("understood", "understand"),
        ("spoke", "speak"),
        ("spoken", "speak"),
        ("broke", "break"),
        ("broken", "break"),
        ("began", "begin"),
        ("begun", "begin"),
        ("drank", "drink"),
        ("drunk", "drink"),
        ("drove", "drive"),
        ("driven", "drive"),
        ("ate", "eat"),
        ("eaten", "eat"),
        ("fell", "fall"),
        ("fallen", "fall"),
        ("flew", "fly"),
        ("flown", "fly"),
        ("knew", "know"),
        ("known", "know"),
        ("grew", "grow"),
        ("grown", "grow"),
        ("threw", "throw"),
        ("thrown", "throw"),
        ("wore", "wear"),
        ("worn", "wear"),
        ("wrote", "write"),
        ("written", "write"),
        ("chose", "choose"),
        ("chosen", "choose"),
        ("forgot", "forget"),
        ("forgotten", "forget"),
        ("hid", "hide"),
        ("hidden", "hide"),
        ("sang", "sing"),
        ("sung", "sing"),
        ("swam", "swim"),
        ("swum", "swim"),
        ("rose", "rise"),
        ("risen", "rise"),
        ("shook", "shake"),
        ("shaken", "shake"),
    ]
    .into_iter()
    .collect::<std::collections::HashMap<_, _>>();
    if let Some(base) = irregular.get(word.as_str()) {
        add((*base).to_string());
    }
    if word.len() < 4 {
        return result;
    }
    let stem = |count: usize| word[..word.len() - count].to_string();
    let is_doubled = |base: &str| {
        let bytes = base.as_bytes();
        bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2]
    };

    if word.ends_with("ies") && word.len() > 4 {
        add(format!("{}y", stem(3)));
    } else if ["ses", "xes", "zes", "ches", "shes"]
        .iter()
        .any(|suffix| word.ends_with(suffix))
    {
        add(stem(2));
    } else if word.ends_with("ied") {
        add(format!("{}y", stem(3)));
    } else if word.ends_with("es") {
        add(stem(2));
    }
    if word.ends_with('s') && !word.ends_with("ss") {
        add(stem(1));
    }
    if word.ends_with("ed") {
        let base = stem(2);
        if is_doubled(&base) {
            add(base[..base.len() - 1].to_string());
        } else {
            add(base.clone());
        }
        add(format!("{base}e"));
    }
    if word.ends_with("ying") && word.len() > 4 {
        add(format!("{}ie", stem(4)));
    } else if word.ends_with("ing") {
        let base = stem(3);
        if is_doubled(&base) {
            add(base[..base.len() - 1].to_string());
        } else {
            add(base.clone());
            add(format!("{base}e"));
        }
    }
    if word.ends_with("iest") {
        add(format!("{}y", stem(4)));
    } else if word.ends_with("est") {
        let base = stem(3);
        if is_doubled(&base) {
            add(base[..base.len() - 1].to_string());
        } else {
            add(base);
        }
    }
    if word.ends_with("ier") {
        add(format!("{}y", stem(3)));
    } else if word.ends_with("er") {
        let base = stem(2);
        if is_doubled(&base) {
            add(base[..base.len() - 1].to_string());
        } else {
            add(base);
        }
    }
    result
}
