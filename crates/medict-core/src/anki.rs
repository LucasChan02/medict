use std::io::Write;
use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use regex::{Captures, Regex};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

use crate::bookmarks::ExportRow;
use crate::error::{MedictError, Result};
use crate::render::definition_body;

const ANKI_SCHEMA_VERSION: i64 = 11;
const MODEL_ID: i64 = 1_700_000_000_119;

static DATA_MEDIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(src|href)="data:([^;]+);base64,([A-Za-z0-9+/=]+)""#)
        .expect("valid data media regex")
});
static TAGS_OR_MEDIA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\[\[type:[^]]+\]\]|<[^>]+>|\[sound:[^]]*\]")
        .expect("valid Anki cleanup regex")
});
static WHITESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s").expect("valid whitespace regex"));

struct PreparedRow {
    word: String,
    dict_name: String,
    notebook: String,
    definition: String,
}

struct MediaFile {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct MediaCollector {
    files: Vec<MediaFile>,
}

impl MediaCollector {
    fn rewrite(&mut self, html: &str) -> String {
        DATA_MEDIA
            .replace_all(html, |captures: &Captures<'_>| {
                let attribute = &captures[1];
                let mime = &captures[2];
                let Ok(bytes) = BASE64.decode(&captures[3]) else {
                    return captures[0].to_string();
                };
                let extension = extension_for_mime(mime);
                let name = format!("medict-media-{}.{}", self.files.len(), extension);
                self.files.push(MediaFile {
                    name: name.clone(),
                    bytes,
                });
                format!(r#"{attribute}="{name}""#)
            })
            .into_owned()
    }
}

pub fn build(rows: &[ExportRow]) -> Result<Vec<u8>> {
    if rows.is_empty() {
        return Err(MedictError::Message(
            "There are no saved words to export.".to_string(),
        ));
    }
    let mut media = MediaCollector::default();
    let prepared = rows
        .iter()
        .map(|row| PreparedRow {
            word: row.word.clone(),
            dict_name: row.dict_name.clone(),
            notebook: if row.notebook_name.is_empty() {
                "Default".to_string()
            } else {
                row.notebook_name.clone()
            },
            definition: media.rewrite(&definition_body(&row.html)),
        })
        .collect::<Vec<_>>();

    let now = chrono::Utc::now();
    let collection = build_collection(&prepared, now.timestamp_millis(), now.timestamp())?;
    zip_package(&collection, &media.files)
}

fn build_collection(
    rows: &[PreparedRow],
    modified_ms: i64,
    created_seconds: i64,
) -> Result<Vec<u8>> {
    let temporary = tempfile::NamedTempFile::new()?;
    let path = temporary.path().to_path_buf();
    drop(temporary);

    let connection = Connection::open(&path)?;
    connection.execute_batch(
        "PRAGMA journal_mode = OFF;
         PRAGMA synchronous = OFF;
         CREATE TABLE col (
             crt INTEGER NOT NULL, mod INTEGER NOT NULL, scm INTEGER NOT NULL,
             ver INTEGER NOT NULL, dty INTEGER NOT NULL, usn INTEGER NOT NULL,
             ls INTEGER NOT NULL, conf TEXT NOT NULL, models TEXT NOT NULL,
             decks TEXT NOT NULL, dconf TEXT NOT NULL, tags TEXT NOT NULL
         );
         CREATE TABLE notes (
             id INTEGER PRIMARY KEY, guid TEXT NOT NULL, mid INTEGER NOT NULL,
             mod INTEGER NOT NULL, usn INTEGER NOT NULL, tags TEXT NOT NULL,
             flds TEXT NOT NULL, sfld INTEGER NOT NULL, csum INTEGER NOT NULL,
             flags INTEGER NOT NULL, data TEXT NOT NULL
         );
         CREATE TABLE cards (
             id INTEGER PRIMARY KEY, nid INTEGER NOT NULL, did INTEGER NOT NULL,
             ord INTEGER NOT NULL, mod INTEGER NOT NULL, usn INTEGER NOT NULL,
             type INTEGER NOT NULL, queue INTEGER NOT NULL, due INTEGER NOT NULL,
             ivl INTEGER NOT NULL, factor INTEGER NOT NULL, reps INTEGER NOT NULL,
             lapses INTEGER NOT NULL, left INTEGER NOT NULL, odue INTEGER NOT NULL,
             odid INTEGER NOT NULL, flags INTEGER NOT NULL, data TEXT NOT NULL
         );
         CREATE TABLE revlog (
             id INTEGER PRIMARY KEY, cid INTEGER NOT NULL, usn INTEGER NOT NULL,
             ease INTEGER NOT NULL, ivl INTEGER NOT NULL, lastIvl INTEGER NOT NULL,
             factor INTEGER NOT NULL, time INTEGER NOT NULL, type INTEGER NOT NULL
         );
         CREATE TABLE graves (
             usn INTEGER NOT NULL, oid INTEGER NOT NULL, type INTEGER NOT NULL
         );
         CREATE INDEX ix_notes_usn ON notes (usn);
         CREATE INDEX ix_notes_csum ON notes (csum);
         CREATE INDEX ix_notes_dup ON notes (mid, csum);
         CREATE INDEX ix_cards_nid ON cards (nid);
         CREATE INDEX ix_cards_sched ON cards (did, queue, due);",
    )?;

    let mut decks = serde_json::Map::new();
    decks.insert("1".to_string(), deck_json(1, "Default", modified_ms));
    for notebook in unique_notebooks(rows) {
        let id = deck_id(&notebook);
        decks.insert(
            id.to_string(),
            deck_json(id, &format!("Medict::{notebook}"), modified_ms),
        );
    }

    let mut models = serde_json::Map::new();
    models.insert(
        MODEL_ID.to_string(),
        json!({
            "id": MODEL_ID,
            "name": "Medict Word",
            "type": 0,
            "mod": modified_ms,
            "usn": -1,
            "sortf": 0,
            "did": Value::Null,
            "tmpls": [{
                "name": "Word-Definition",
                "ord": 0,
                "qfmt": "{{Word}}",
                "afmt": "{{FrontSide}}<hr id=\"answer\">{{Definition}}<br><div style=\"color:#999;font-size:14px\">{{Dict}}</div>",
                "bqfmt": "",
                "bafmt": "",
                "did": Value::Null,
                "mode": 0
            }],
            "flds": [
                {"name": "Word", "ord": 0, "sticky": false, "rtl": false, "font": "Arial", "size": 28, "media": []},
                {"name": "Definition", "ord": 1, "sticky": false, "rtl": false, "font": "Arial", "size": 20, "media": []},
                {"name": "Dict", "ord": 2, "sticky": false, "rtl": false, "font": "Arial", "size": 14, "media": []}
            ],
            "css": ".card{font-family:Arial;font-size:20px;text-align:left;line-height:1.5;}",
            "req": [["Word-Definition", "any", [0]]],
            "latexPostamble": "",
            "latexPreamble": "",
            "latexsvg": false,
            "vers": [],
            "tags": [],
            "version": 0
        }),
    );
    let models = Value::Object(models);
    let config = json!({
        "activeDecks": [1],
        "curDeck": 1,
        "curModel": Value::Null,
        "dueCounts": true,
        "estTimes": true,
        "newBury": true,
        "newSpread": 0,
        "nextPos": rows.len() as i64,
        "sortBackwards": false,
        "schedVer": 1
    });
    let deck_config = json!({
        "1": {
            "id": 1, "mod": modified_ms, "usn": -1, "name": "Default",
            "autoplay": true, "dyn": 0, "maxTaken": 60, "timer": 0, "replayq": true,
            "hash": "37c3a0581407a8d3",
            "new": {
                "bury": false, "delays": [1, 10], "initialFactor": 2500,
                "ints": [1, 4, 7], "order": 1, "perDay": 20, "separate": false
            },
            "rev": {
                "bury": false, "ease4": 1.3, "fuzz": 0.05, "ivlFct": 1.0,
                "maxIvl": 36500, "minSpace": 1, "perDay": 200, "hardFactor": 1.2
            },
            "lapse": {"delays": [10], "leechAction": 1, "leechFails": 8, "minInt": 1, "mult": 0.5}
        }
    });

    connection.execute(
        "INSERT INTO col(crt, mod, scm, ver, dty, usn, ls, conf, models, decks, dconf, tags)
         VALUES(?1, ?2, ?3, ?4, 0, -1, 0, ?5, ?6, ?7, ?8, '{}')",
        params![
            created_seconds,
            modified_ms,
            created_seconds.saturating_mul(1000),
            ANKI_SCHEMA_VERSION,
            config.to_string(),
            models.to_string(),
            Value::Object(decks).to_string(),
            deck_config.to_string(),
        ],
    )?;

    let transaction = connection.unchecked_transaction()?;
    {
        let mut note = transaction.prepare(
            "INSERT INTO notes(id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data)
             VALUES(?1, ?2, ?3, ?4, -1, ?5, ?6, ?7, ?8, 0, '')",
        )?;
        let mut card = transaction.prepare(
            "INSERT INTO cards(id, nid, did, ord, mod, usn, type, queue, due, ivl, factor,
                               reps, lapses, left, odue, odid, flags, data)
             VALUES(?1, ?2, ?3, 0, ?4, -1, 0, 0, ?5, 0, 0, 0, 0, 0, 0, 0, 0, '')",
        )?;
        let base_id = modified_ms;
        for (index, row) in rows.iter().enumerate() {
            let note_id = base_id + index as i64 * 2;
            let card_id = note_id + 1;
            let tags = clean_tag(&format!("{} {}", row.notebook, row.dict_name));
            let fields = format!(
                "{}\u{1f}{}\u{1f}{}",
                row.word, row.definition, row.dict_name
            );
            note.execute(params![
                note_id,
                guid(&row.word, &row.notebook, &row.dict_name),
                MODEL_ID,
                modified_ms,
                tags,
                fields,
                row.word,
                field_checksum(&row.word),
            ])?;
            card.execute(params![
                card_id,
                note_id,
                deck_id(&row.notebook),
                modified_ms,
                index as i64,
            ])?;
        }
    }
    transaction.commit()?;
    connection.close().map_err(|(_, error)| error)?;
    let bytes = std::fs::read(&path)?;
    let _ = std::fs::remove_file(path);
    Ok(bytes)
}

fn zip_package(collection: &[u8], media: &[MediaFile]) -> Result<Vec<u8>> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("collection.anki2", options)?;
        zip.write_all(collection)?;

        let mut mapping = serde_json::Map::new();
        for (index, file) in media.iter().enumerate() {
            mapping.insert(index.to_string(), Value::String(file.name.clone()));
        }
        zip.start_file("media", options)?;
        zip.write_all(serde_json::to_string(&mapping)?.as_bytes())?;
        for (index, file) in media.iter().enumerate() {
            zip.start_file(index.to_string(), options)?;
            zip.write_all(&file.bytes)?;
        }
        zip.finish()?;
    }
    Ok(buffer.into_inner())
}

fn field_checksum(value: &str) -> u32 {
    let stripped = TAGS_OR_MEDIA.replace_all(value, "");
    let compact = WHITESPACE.replace_all(&stripped, "");
    let digest = Sha1::digest(compact.as_bytes());
    u32::from_le_bytes(digest[..4].try_into().unwrap_or_default())
}

fn guid(word: &str, notebook: &str, dictionary: &str) -> String {
    let digest = Sha1::digest(format!("{word}\u{1f}{notebook}\u{1f}{dictionary}").as_bytes());
    format!("{:x}", Sha1::digest(digest))
        .chars()
        .take(10)
        .collect()
}

fn deck_id(name: &str) -> i64 {
    let digest = Sha1::digest(format!("deck:{name}").as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    let id = (u64::from_be_bytes(bytes) % 1_000_000_000_000) as i64;
    id.max(1)
}

fn deck_json(id: i64, name: &str, modified_ms: i64) -> Value {
    json!({
        "id": id,
        "name": name,
        "mod": modified_ms,
        "usn": -1,
        "desc": "",
        "dyn": 0,
        "collapsed": false,
        "browserCollapsed": false,
        "extendNew": 10,
        "extendRev": 50,
        "conf": 1,
        "newToday": [0, 0],
        "revToday": [0, 0],
        "lrnToday": [0, 0],
        "timeToday": [0, 0]
    })
}

fn unique_notebooks(rows: &[PreparedRow]) -> Vec<String> {
    let mut names = rows
        .iter()
        .map(|row| row.notebook.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn clean_tag(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(' ', "_")
        .replace('/', "_")
}

fn extension_for_mime(mime: &str) -> &'static str {
    let mime = mime.to_ascii_lowercase();
    if mime.contains("png") {
        "png"
    } else if mime.contains("jpeg") || mime.contains("jpg") {
        "jpg"
    } else if mime.contains("gif") {
        "gif"
    } else if mime.contains("webp") {
        "webp"
    } else if mime.contains("svg") {
        "svg"
    } else if mime.contains("mpeg") || mime.contains("mp3") {
        "mp3"
    } else if mime.contains("wav") {
        "wav"
    } else if mime.contains("ogg") {
        "ogg"
    } else if mime.contains("mp4") {
        "m4a"
    } else if mime.contains("woff2") {
        "woff2"
    } else if mime.contains("woff") {
        "woff"
    } else if mime.contains("ttf") {
        "ttf"
    } else if mime.contains("otf") {
        "otf"
    } else {
        "bin"
    }
}

