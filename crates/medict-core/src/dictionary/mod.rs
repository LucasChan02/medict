mod ecdict;
mod mdict;
mod stardict;

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;

use crate::error::{MedictError, Result};
use crate::model::DictionaryView;

use self::ecdict::EcdictBackend;
use self::mdict::MdictBackend;
use self::stardict::StarDictBackend;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryMeta {
    pub id: String,
    pub name: String,
    pub dict_type: String,
    pub directory: PathBuf,
    pub description: String,
}

pub struct ResolvedEntry {
    pub key: String,
    pub html: String,
}

pub enum Backend {
    Mdict(MdictBackend),
    StarDict(StarDictBackend),
    Ecdict(EcdictBackend),
}

pub struct Dictionary {
    pub meta: DictionaryMeta,
    backend: Mutex<Backend>,
    base_css: OnceLock<String>,
}

impl Dictionary {
    pub fn new(meta: DictionaryMeta, backend: Backend) -> Self {
        Self {
            meta,
            backend: Mutex::new(backend),
            base_css: OnceLock::new(),
        }
    }

    pub fn view(&self, selected: bool, current: bool) -> DictionaryView {
        DictionaryView {
            id: self.meta.id.clone(),
            name: self.meta.name.clone(),
            dict_type: self.meta.dict_type.clone(),
            directory: self.meta.directory.to_string_lossy().into_owned(),
            description: self.meta.description.clone(),
            selected,
            current,
        }
    }

    pub fn resolve_entry(&self, word: &str) -> Result<Option<ResolvedEntry>> {
        self.resolve_entry_inner(word, 0, &mut std::collections::HashSet::new())
    }

    fn resolve_entry_inner(
        &self,
        word: &str,
        depth: usize,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<Option<ResolvedEntry>> {
        if depth > 8 || word.trim().is_empty() {
            return Ok(None);
        }
        let Some(key) = self.find_key(word)? else {
            return Ok(None);
        };
        let Some(html) = self.definition(&key)? else {
            return Ok(None);
        };
        let targets = link_targets(&html);
        if targets.is_empty() {
            return Ok(Some(ResolvedEntry { key, html }));
        }

        let mut parts = Vec::new();
        for target in targets {
            if !visited.insert(target.clone()) {
                continue;
            }
            if let Some(entry) = self.resolve_entry_inner(&target, depth + 1, visited)? {
                parts.push(entry.html);
            }
        }
        if parts.is_empty() {
            return Ok(Some(ResolvedEntry { key, html }));
        }
        Ok(Some(ResolvedEntry {
            key,
            html: parts.join("<hr>"),
        }))
    }

    pub fn find_key(&self, word: &str) -> Result<Option<String>> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| MedictError::Message("Dictionary lock is poisoned.".to_string()))?;
        backend.find_key(word)
    }

    pub fn definition(&self, key: &str) -> Result<Option<String>> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| MedictError::Message("Dictionary lock is poisoned.".to_string()))?;
        backend.definition(key)
    }

    pub fn suggestions(&self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| MedictError::Message("Dictionary lock is poisoned.".to_string()))?;
        backend.suggestions(prefix, limit)
    }

    pub fn resource(&self, path: &str) -> Option<Vec<u8>> {
        let mut backend = self.backend.lock().ok()?;
        backend.resource(path)
    }

    pub fn build_index(&self) -> Result<()> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| MedictError::Message("Dictionary lock is poisoned.".to_string()))?;
        backend.build_index()
    }

    pub fn base_css(&self) -> String {
        self.base_css
            .get_or_init(|| {
                self.backend
                    .lock()
                    .map(|mut backend| backend.base_css())
                    .unwrap_or_default()
            })
            .clone()
    }

    pub fn ecdict_status(&self) -> Option<Result<(String, i64)>> {
        let mut backend = self.backend.lock().ok()?;
        match &mut *backend {
            Backend::Ecdict(value) => Some(value.status()),
            _ => None,
        }
    }

    pub fn install_full_ecdict(&self) -> Result<i64> {
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| MedictError::Message("Dictionary lock is poisoned.".to_string()))?;
        match &mut *backend {
            Backend::Ecdict(value) => value.install_full(),
            _ => Err(MedictError::Message(
                "ECDICT dictionary is not loaded.".to_string(),
            )),
        }
    }
}

fn link_targets(html: &str) -> Vec<String> {
    let normalized = html.trim().replace("</>", "\n");
    let mut targets = Vec::new();
    for line in normalized.lines() {
        let line = line.trim();
        if let Some(target) = line.strip_prefix("@@@LINK=") {
            let target = target.trim();
            if !target.is_empty() {
                targets.push(target.to_string());
            }
        }
    }
    targets
}

pub fn open_mdict(directory: PathBuf, mdx_path: PathBuf) -> Result<Dictionary> {
    let backend = MdictBackend::open(&directory, &mdx_path)?;
    let meta = DictionaryMeta {
        id: crate::util::stable_id(&directory),
        name: backend.title().to_string(),
        dict_type: "mdict".to_string(),
        directory,
        description: backend.description().to_string(),
    };
    Ok(Dictionary::new(meta, Backend::Mdict(backend)))
}

pub fn open_stardict(directory: PathBuf, ifo_path: PathBuf) -> Result<Dictionary> {
    let backend = StarDictBackend::open(&ifo_path)?;
    let meta = DictionaryMeta {
        id: crate::util::stable_id(&directory),
        name: backend.title().to_string(),
        dict_type: "stardict".to_string(),
        directory,
        description: backend.description().to_string(),
    };
    Ok(Dictionary::new(meta, Backend::StarDict(backend)))
}

pub fn open_ecdict(directory: PathBuf, database_path: PathBuf) -> Result<Dictionary> {
    let backend = EcdictBackend::open(&database_path)?;
    let (edition, count) = backend.status()?;
    let description = if edition == "full" {
        format!("ECDICT offline English–Chinese dictionary ({count} entries)")
    } else {
        "ECDICT offline English–Chinese dictionary".to_string()
    };
    let meta = DictionaryMeta {
        id: crate::util::stable_id(&directory),
        name: "简明英汉词典".to_string(),
        dict_type: "ecdict".to_string(),
        directory,
        description,
    };
    Ok(Dictionary::new(meta, Backend::Ecdict(backend)))
}

impl Backend {
    fn find_key(&mut self, word: &str) -> Result<Option<String>> {
        match self {
            Self::Mdict(value) => value.find_key(word),
            Self::StarDict(value) => value.find_key(word),
            Self::Ecdict(value) => value.find_key(word),
        }
    }

    fn definition(&mut self, key: &str) -> Result<Option<String>> {
        match self {
            Self::Mdict(value) => value.definition(key),
            Self::StarDict(value) => value.definition(key),
            Self::Ecdict(value) => value.definition(key),
        }
    }

    fn suggestions(&mut self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        match self {
            Self::Mdict(value) => value.suggestions(prefix, limit),
            Self::StarDict(value) => value.suggestions(prefix, limit),
            Self::Ecdict(value) => value.suggestions(prefix, limit),
        }
    }

    fn resource(&mut self, path: &str) -> Option<Vec<u8>> {
        match self {
            Self::Mdict(value) => value.resource(path),
            Self::StarDict(value) => value.resource(path),
            Self::Ecdict(_) => None,
        }
    }

    fn build_index(&mut self) -> Result<()> {
        match self {
            Self::Mdict(value) => value.build_index(),
            Self::StarDict(value) => value.build_index(),
            Self::Ecdict(value) => value.build_index(),
        }
    }

    fn base_css(&mut self) -> String {
        match self {
            Self::Mdict(value) => value.base_css(),
            Self::StarDict(value) => value.base_css(),
            Self::Ecdict(value) => value.base_css(),
        }
    }
}
