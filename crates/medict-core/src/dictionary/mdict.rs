use std::fs;
use std::path::{Path, PathBuf};

use mdictlib::{KeyEntry, MddFile, MdxFile};

use crate::error::Result;
use crate::util::{dedupe_strings, mime_for_path, safe_join};

pub struct MdictBackend {
    directory: PathBuf,
    mdx: MdxFile,
    mdd: Vec<MddFile>,
    key_cache: Option<Vec<KeyEntry>>,
    title: String,
    description: String,
    css: String,
}

impl MdictBackend {
    pub fn open(directory: &Path, mdx_path: &Path) -> Result<Self> {
        let mdx = MdxFile::open(mdx_path)?;
        let stem = mdx_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("dictionary");
        let title = mdx
            .header()
            .title()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(stem)
            .to_string();
        let description = mdx.header().description().unwrap_or_default().to_string();

        let mut mdd = Vec::new();
        if let Ok(entries) = fs::read_dir(directory) {
            let mut paths: Vec<_> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case("mdd"))
                })
                .collect();
            paths.sort();
            for path in paths {
                if let Ok(resource) = MddFile::open(&path) {
                    mdd.push(resource);
                }
            }
        }

        let mut backend = Self {
            directory: directory.to_path_buf(),
            mdx,
            mdd,
            key_cache: None,
            title,
            description,
            css: String::new(),
        };
        backend.css = backend.load_base_css();
        Ok(backend)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn find_key(&mut self, word: &str) -> Result<Option<String>> {
        let word = word.trim();
        if word.is_empty() {
            return Ok(None);
        }
        if let Some(entry) = self.mdx.lookup(word)? {
            return Ok(Some(entry.key().to_string()));
        }
        let prefix = self.mdx.prefix_keys(word, 16)?;
        if let Some(entry) = prefix.into_iter().next() {
            return Ok(Some(entry.key().to_string()));
        }
        self.fuzzy_key(word)
    }

    pub fn definition(&mut self, key: &str) -> Result<Option<String>> {
        Ok(self.mdx.lookup(key)?.map(|entry| entry.text().to_string()))
    }

    pub fn suggestions(&mut self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        let mut keys = self
            .mdx
            .prefix_keys(prefix, limit.saturating_mul(2).max(limit))?
            .into_iter()
            .map(|entry| entry.into_key())
            .collect::<Vec<_>>();
        if keys.is_empty() {
            if let Some(key) = self.fuzzy_key(prefix)? {
                keys.push(key);
            }
        }
        Ok(dedupe_strings(keys, limit))
    }

    pub fn resource(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(path) = safe_join(&self.directory, key) {
            if path.is_file() {
                if let Ok(bytes) = fs::read(path) {
                    return Some(bytes);
                }
            }
        }

        for candidate in resource_candidates(key) {
            for resource in &self.mdd {
                if let Ok(Some(value)) = resource.lookup(&candidate) {
                    return Some(value.bytes().to_vec());
                }
            }
        }
        None
    }

    pub fn build_index(&mut self) -> Result<()> {
        self.key_cache = Some(self.read_keys()?);
        Ok(())
    }

    pub fn base_css(&mut self) -> String {
        self.css.clone()
    }

    fn fuzzy_key(&mut self, word: &str) -> Result<Option<String>> {
        if self.key_cache.is_none() {
            self.key_cache = Some(self.read_keys()?);
        }
        let needle = word.to_lowercase();
        let mut matches = Vec::new();
        if let Some(keys) = &self.key_cache {
            for entry in keys {
                let candidate = entry.key();
                if candidate.len().abs_diff(needle.len()) > 2 {
                    continue;
                }
                let lower = candidate.to_lowercase();
                let distance = strsim::levenshtein(&needle, &lower);
                if distance <= 2 {
                    matches.push((distance, candidate.to_string()));
                }
            }
        }
        matches.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.to_lowercase().cmp(&right.1.to_lowercase()))
        });
        Ok(matches.into_iter().next().map(|(_, key)| key))
    }

    fn read_keys(&self) -> Result<Vec<KeyEntry>> {
        let mut keys = Vec::new();
        for item in self.mdx.keys() {
            if let Ok(entry) = item {
                keys.push(entry);
            }
        }
        Ok(keys)
    }

    fn load_base_css(&mut self) -> String {
        let mut css = String::new();
        let mut seen = std::collections::HashSet::new();
        for entry in walkdir::WalkDir::new(&self.directory)
            .max_depth(4)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("css"))
            {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                if seen.insert(content.clone()) {
                    css.push_str(&format!("\n/* {} */\n{}", path.display(), content));
                }
            }
            if css.len() > 4 * 1024 * 1024 || seen.len() > 64 {
                break;
            }
        }

        let mdx_name = self
            .mdx
            .header()
            .title()
            .map(|value| format!("{value}.css"))
            .unwrap_or_default();
        let mut names = vec![
            Path::new(&self.title)
                .with_extension("css")
                .to_string_lossy()
                .into_owned(),
        ];
        if !mdx_name.is_empty() {
            names.push(mdx_name);
        }
        for name in names {
            for candidate in resource_candidates(&name) {
                for resource in &self.mdd {
                    if let Ok(Some(value)) = resource.lookup(&candidate) {
                        if let Ok(text) = String::from_utf8(value.bytes().to_vec()) {
                            if seen.insert(text.clone()) {
                                css.push_str(&text);
                            }
                        }
                        break;
                    }
                }
            }
        }
        css
    }
}

fn resource_candidates(key: &str) -> Vec<String> {
    let trimmed = key.trim_start_matches(['/', '\\']);
    let backslash = trimmed.replace('/', "\\");
    dedupe_strings(
        [
            key.to_string(),
            trimmed.to_string(),
            backslash.clone(),
            format!("\\{backslash}"),
            format!("/{trimmed}"),
        ],
        8,
    )
}

#[allow(dead_code)]
fn _content_type_hint(path: &str) -> &'static str {
    mime_for_path(Path::new(path))
}
