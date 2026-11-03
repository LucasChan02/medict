use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use html_escape::encode_text;

use crate::error::{MedictError, Result};
use crate::util::{dedupe_strings, safe_join};

struct IndexEntry {
    word: String,
    lower: String,
    offset: u64,
    size: u64,
}

enum DataSource {
    File(File),
    Memory(Vec<u8>),
}

pub struct StarDictBackend {
    directory: PathBuf,
    resource_dir: PathBuf,
    book_name: String,
    description: String,
    same_type_sequence: Option<String>,
    entries: Vec<IndexEntry>,
    data: DataSource,
    css: String,
}

impl StarDictBackend {
    pub fn open(ifo_path: &Path) -> Result<Self> {
        let directory = ifo_path
            .parent()
            .ok_or_else(|| MedictError::Message("StarDict path has no parent.".to_string()))?
            .to_path_buf();
        let metadata = parse_ifo(ifo_path)?;
        let stem = ifo_path
            .file_stem()
            .ok_or_else(|| MedictError::Message("StarDict file has no name.".to_string()))?;
        let idx_path = directory.join(format!("{}.idx", stem.to_string_lossy()));
        let mut data_path = directory.join(format!("{}.dict", stem.to_string_lossy()));
        if !data_path.is_file() {
            data_path = directory.join(format!("{}.dict.dz", stem.to_string_lossy()));
        }
        if !idx_path.is_file() || !data_path.is_file() {
            return Err(MedictError::Message(format!(
                "StarDict index or data file is missing next to {}.",
                ifo_path.display()
            )));
        }

        let idx_bytes = fs::read(&idx_path)?;
        let entries = parse_idx(&idx_bytes, metadata.offset_bits)?;
        if entries.is_empty() {
            return Err(MedictError::Message(
                "StarDict index contains no words.".to_string(),
            ));
        }
        let data = if data_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("dz"))
        {
            let compressed = fs::read(&data_path)?;
            let mut decoder = GzDecoder::new(Cursor::new(compressed));
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)?;
            DataSource::Memory(decoded)
        } else {
            DataSource::File(File::open(data_path)?)
        };

        let resource_dir = directory.join(format!("{}.res", stem.to_string_lossy()));
        let mut backend = Self {
            directory,
            resource_dir,
            book_name: metadata.book_name,
            description: metadata.description,
            same_type_sequence: metadata.same_type_sequence,
            entries,
            data,
            css: String::new(),
        };
        backend.entries.sort_by(|left, right| {
            left.lower
                .cmp(&right.lower)
                .then_with(|| left.word.cmp(&right.word))
        });
        backend.css = backend.load_base_css();
        Ok(backend)
    }

    pub fn title(&self) -> &str {
        &self.book_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn find_key(&mut self, word: &str) -> Result<Option<String>> {
        let needle = word.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(None);
        }
        if let Ok(index) = self.entries.binary_search_by(|entry| entry.lower.cmp(&needle)) {
            return Ok(Some(self.entries[index].word.clone()));
        }
        let prefix = self.suggestions(&needle, 1)?;
        if let Some(key) = prefix.into_iter().next() {
            return Ok(Some(key));
        }
        let mut best: Option<(usize, String)> = None;
        for entry in &self.entries {
            if entry.word.len().abs_diff(needle.len()) > 2 {
                continue;
            }
            let distance = strsim::levenshtein(&needle, &entry.lower);
            if distance <= 2
                && best
                    .as_ref()
                    .is_none_or(|(best_distance, _)| distance < *best_distance)
            {
                best = Some((distance, entry.word.clone()));
            }
        }
        Ok(best.map(|(_, word)| word))
    }

    pub fn definition(&mut self, key: &str) -> Result<Option<String>> {
        let needle = key.to_lowercase();
        let Ok(index) = self.entries.binary_search_by(|entry| entry.lower.cmp(&needle)) else {
            return Ok(None);
        };
        let entry = &self.entries[index];
        let mut bytes = vec![0u8; entry.size as usize];
        match &mut self.data {
            DataSource::File(file) => read_exact_at(file, &mut bytes, entry.offset)?,
            DataSource::Memory(data) => {
                let start = entry.offset as usize;
                let end = start.saturating_add(entry.size as usize);
                if end > data.len() {
                    return Err(MedictError::Message(
                        "StarDict entry is outside the data file.".to_string(),
                    ));
                }
                bytes.copy_from_slice(&data[start..end]);
            }
        }
        Ok(Some(render_article(
            &bytes,
            self.same_type_sequence.as_deref(),
        )))
    }

    pub fn suggestions(&mut self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        let needle = prefix.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let start = self
            .entries
            .partition_point(|entry| entry.lower.as_str() < needle.as_str());
        let matches = self.entries[start..]
            .iter()
            .take_while(|entry| entry.lower.starts_with(&needle))
            .take(limit.saturating_mul(2).max(limit))
            .map(|entry| entry.word.clone())
            .collect::<Vec<_>>();
        Ok(dedupe_strings(matches, limit))
    }

    pub fn resource(&mut self, key: &str) -> Option<Vec<u8>> {
        for base in [&self.resource_dir, &self.directory] {
            if let Some(path) = safe_join(base, key) {
                if path.is_file() {
                    if let Ok(bytes) = fs::read(path) {
                        return Some(bytes);
                    }
                }
            }
        }
        None
    }

    pub fn build_index(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn base_css(&mut self) -> String {
        self.css.clone()
    }

    fn load_base_css(&self) -> String {
        let mut parts = Vec::new();
        for base in [&self.directory, &self.resource_dir] {
            let Ok(entries) = fs::read_dir(base) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("css"))
                {
                    if let Ok(css) = fs::read_to_string(path) {
                        parts.push(css);
                    }
                }
            }
        }
        parts.join("\n")
    }
}

struct IfoMetadata {
    book_name: String,
    description: String,
    same_type_sequence: Option<String>,
    offset_bits: u8,
}

fn parse_ifo(path: &Path) -> Result<IfoMetadata> {
    let text = fs::read_to_string(path)?;
    let mut fields = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        fields.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let book_name = fields
        .get("bookname")
        .cloned()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            path.file_stem()
                .map(|value| value.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "StarDict dictionary".to_string());
    let description = fields
        .get("description")
        .cloned()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "StarDict dictionary".to_string());
    let offset_bits = fields
        .get("idxoffsetbits")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(32);
    Ok(IfoMetadata {
        book_name,
        description,
        same_type_sequence: fields.get("sametypesequence").cloned(),
        offset_bits,
    })
}

fn parse_idx(bytes: &[u8], offset_bits: u8) -> Result<Vec<IndexEntry>> {
    let offset_size = match offset_bits {
        64 => 8,
        32 => 4,
        _ => {
            return Err(MedictError::Message(format!(
                "Unsupported StarDict offset size: {offset_bits}."
            )));
        }
    };
    let record_size = offset_size + 4;
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(terminator) = bytes[cursor..].iter().position(|value| *value == 0) else {
            break;
        };
        let word_end = cursor + terminator;
        let word = String::from_utf8_lossy(&bytes[cursor..word_end]).into_owned();
        let metadata_start = word_end + 1;
        let metadata_end = metadata_start.saturating_add(record_size);
        if metadata_end > bytes.len() {
            break;
        }
        let offset = if offset_size == 8 {
            u64::from_be_bytes(
                bytes[metadata_start..metadata_start + 8]
                    .try_into()
                    .unwrap_or_default(),
            )
        } else {
            u32::from_be_bytes(
                bytes[metadata_start..metadata_start + 4]
                    .try_into()
                    .unwrap_or_default(),
            ) as u64
        };
        let size_start = metadata_start + offset_size;
        let size = u32::from_be_bytes(
            bytes[size_start..size_start + 4]
                .try_into()
                .unwrap_or_default(),
        ) as u64;
        entries.push(IndexEntry {
            lower: word.to_lowercase(),
            word,
            offset,
            size,
        });
        cursor = metadata_end;
    }
    Ok(entries)
}

fn render_article(bytes: &[u8], same_type_sequence: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(sequence) = same_type_sequence.filter(|value| !value.is_empty()) {
        if sequence.len() == 1 {
            parts.push(render_field(sequence.as_bytes()[0], bytes));
        } else {
            let mut cursor = 0usize;
            for (index, data_type) in sequence.bytes().enumerate() {
                if cursor >= bytes.len() {
                    break;
                }
                let end = if index + 1 == sequence.len() {
                    bytes.len()
                } else {
                    bytes[cursor..]
                        .iter()
                        .position(|value| *value == 0)
                        .map(|position| cursor + position)
                        .unwrap_or(bytes.len())
                };
                parts.push(render_field(data_type, &bytes[cursor..end]));
                cursor = end.saturating_add(1);
            }
        }
    } else {
        let mut cursor = 0usize;
        while cursor < bytes.len() {
            let data_type = bytes[cursor];
            cursor += 1;
            let end = bytes[cursor..]
                .iter()
                .position(|value| *value == 0)
                .map(|position| cursor + position)
                .unwrap_or(bytes.len());
            parts.push(render_field(data_type, &bytes[cursor..end]));
            cursor = end.saturating_add(1);
        }
    }
    format!(
        "<style>.sd-entry{{line-height:1.6}} .sd-phonetic{{color:#2a7de1;font-size:1.05em}} \
         .sd-field{{margin:.35em 0}}</style><div class=\"sd-entry\">{}</div>",
        parts.join("")
    )
}

fn render_field(data_type: u8, bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    match data_type {
        b'h' | b'g' => text.into_owned(),
        b'x' => text
            .replace("<k>", "<b>")
            .replace("</k>", "</b>")
            .replace("<tr>", "<div>")
            .replace("</tr>", "</div>")
            .replace("<c>", "<span>")
            .replace("</c>", "</span>"),
        b't' | b'y' => format!(
            "<div class=\"sd-field sd-phonetic\">/{}/</div>",
            encode_text(text.trim())
        ),
        b'r' => String::new(),
        _ => format!(
            "<div class=\"sd-field\">{}</div>",
            encode_text(&text).replace("\\n", "<br>").replace('\n', "<br>")
        ),
    }
}

#[cfg(windows)]
fn read_exact_at(file: &File, buffer: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    let mut written = 0usize;
    while written < buffer.len() {
        let read = file.seek_read(&mut buffer[written..], offset)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "StarDict data ended unexpectedly",
            ));
        }
        written += read;
        offset += read as u64;
    }
    Ok(())
}

#[cfg(unix)]
fn read_exact_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buffer, offset)
}
