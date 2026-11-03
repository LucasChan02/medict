use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde_json::Value;
use uuid::Uuid;

use crate::anki;
use crate::bookmarks::{BookmarkStore, format_timestamp};
use crate::config::AppConfig;
use crate::dictionary::{
    Dictionary, open_ecdict, open_mdict, open_stardict,
};
use crate::error::{MedictError, Result};
use crate::model::{
    BookmarkView, CssEditorView, DefinitionView, EcdictView, NotebookView, SettingsView, UiSnapshot,
};
use crate::render;
use crate::util::{
    app_config_dir, atomic_write, dedupe_strings, ensure_dir, query_escape, stable_id,
};

const PRESET_CC_MDX: &[u8] = include_bytes!("../../../assets/presets/cc-cedict/cc-cedict.mdx");
const PRESET_CC_MDD: &[u8] = include_bytes!("../../../assets/presets/cc-cedict/cc-cedict.mdd");
const PRESET_CC_CSS: &[u8] = include_bytes!("../../../assets/presets/cc-cedict/cc-cedict.css");
const PRESET_CC_COVER: &[u8] = include_bytes!("../../../assets/presets/cc-cedict/cover.png");
const PRESET_ECDICT: &[u8] = include_bytes!("../../../assets/presets/ecdict/ecdict.db");

pub struct AppCore {
    version: String,
    smoke_test: bool,
    server_token: String,
    base_url: RwLock<String>,
    fatal_error: RwLock<String>,
    runtime: RwLock<Option<Runtime>>,
    lookup_sequence: AtomicU64,
    suggestion_sequence: AtomicU64,
    bootstrapping: AtomicBool,
}

struct Runtime {
    config: AppConfig,
    dictionaries: Vec<Arc<Dictionary>>,
    bookmarks: Mutex<BookmarkStore>,
    user_css: HashMap<String, String>,
    ui: UiState,
}

#[derive(Default)]
struct UiState {
    ready: bool,
    busy: bool,
    status: String,
    error: String,
    selected_ids: Vec<String>,
    current_id: String,
    multi_mode: bool,
    suggestions: Vec<String>,
    matches: Vec<String>,
    current_word: String,
    definitions: Vec<DefinitionView>,
    zoom: i32,
    history: Vec<String>,
    history_position: Option<usize>,
    notebooks: Vec<NotebookView>,
    bookmarks: Vec<BookmarkView>,
    selected_notebook_id: String,
    notebooks_busy: bool,
    ecdict: EcdictView,
    settings: SettingsView,
    css: CssEditorView,
}

impl AppCore {
    pub fn new(version: impl Into<String>, smoke_test: bool) -> Self {
        Self {
            version: version.into(),
            smoke_test,
            server_token: Uuid::new_v4().simple().to_string(),
            base_url: RwLock::new(String::new()),
            fatal_error: RwLock::new(String::new()),
            runtime: RwLock::new(None),
            lookup_sequence: AtomicU64::new(0),
            suggestion_sequence: AtomicU64::new(0),
            bootstrapping: AtomicBool::new(false),
        }
    }

    pub fn server_token(&self) -> &str {
        &self.server_token
    }

    pub fn smoke_test(&self) -> bool {
        self.smoke_test
    }

    pub fn set_base_url(&self, value: String) {
        *write_lock(&self.base_url) = value;
    }

    pub fn bootstrap(&self) -> Result<String> {
        if read_lock(&self.runtime).is_some() {
            return Ok(self.snapshot());
        }
        if self.bootstrapping.swap(true, Ordering::SeqCst) {
            return Ok(self.snapshot());
        }

        let result = self.bootstrap_inner();
        if result.is_err() {
            self.bootstrapping.store(false, Ordering::SeqCst);
        }
        result
    }

    fn bootstrap_inner(&self) -> Result<String> {
        let mut config = AppConfig::load()?;
        let config_dir = app_config_dir()?;
        ensure_dir(&config_dir)?;
        install_presets(&config.base_dict_dir)?;

        let allow_symlinks = config.bool("dictionaryallowsymlinks", true);
        let group_limit = config.integer("dictionarygroupmax", 10).clamp(1, 100);
        let zoom = config
            .integer("dictionarycontentzoom", 100)
            .clamp(80, 160) as i32;
        let multi_mode = config.bool("multimode", false);
        let preferred_id = config.string("selecteddictid", "");
        let saved_ids = config.strings("multidictids");
        let last_word = config.string("lastword", "");

        let (dictionaries, scan_errors) =
            scan_dictionaries(&config.base_dict_dir, allow_symlinks);
        let user_css = load_user_css(&config_dir);
        let bookmarks = BookmarkStore::open(&config_dir)?;

        let mut ui = UiState {
            ready: true,
            status: if dictionaries.is_empty() {
                "No dictionaries found. Add dictionary files to the dictionary folder.".to_string()
            } else {
                "Ready".to_string()
            },
            zoom,
            multi_mode,
            current_word: last_word.clone(),
            settings: SettingsView {
                allow_symlinks,
                group_limit,
            },
            ..UiState::default()
        };
        if !scan_errors.is_empty() {
            ui.status = format!(
                "{} dictionary file(s) could not be loaded.",
                scan_errors.len()
            );
        }
        ui.ecdict = EcdictView {
            edition: "compact".to_string(),
            entry_count: 0,
            status: "ECDICT status is not available.".to_string(),
            busy: false,
            full_installed: false,
        };
        restore_selection(
            &mut ui,
            &dictionaries,
            preferred_id.as_str(),
            &saved_ids,
        );
        config.set_integer("dictionarygroupmax", group_limit);
        config.set_bool("dictionaryallowsymlinks", allow_symlinks);

        let runtime = Runtime {
            config,
            dictionaries,
            bookmarks: Mutex::new(bookmarks),
            user_css,
            ui,
        };
        *write_lock(&self.runtime) = Some(runtime);
        self.reload_notebooks_inner()?;
        self.refresh_ecdict_status_inner()?;
        Ok(self.snapshot())
    }

    pub fn last_word(&self) -> String {
        read_lock(&self.runtime)
            .as_ref()
            .map(|runtime| runtime.ui.current_word.clone())
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> String {
        let base_url = read_lock(&self.base_url).clone();
        let fatal_error = read_lock(&self.fatal_error).clone();
        let guard = read_lock(&self.runtime);
        let Some(runtime) = guard.as_ref() else {
            let mut snapshot = UiSnapshot::new(self.version.clone(), self.smoke_test);
            snapshot.base_url = base_url;
            snapshot.error = fatal_error;
            if !snapshot.error.is_empty() {
                snapshot.status = "The dictionary engine could not start.".to_string();
            }
            return snapshot.to_json();
        };
        let ui = &runtime.ui;
        let dictionaries = runtime
            .dictionaries
            .iter()
            .map(|dictionary| {
                dictionary.view(
                    ui.selected_ids.contains(&dictionary.meta.id),
                    dictionary.meta.id == ui.current_id,
                )
            })
            .collect();
        let can_go_back = ui.history_position.is_some_and(|position| position > 0);
        let can_go_forward = ui
            .history_position
            .is_some_and(|position| position + 1 < ui.history.len());
        UiSnapshot {
            version: self.version.clone(),
            platform: std::env::consts::OS.to_string(),
            smoke_test: self.smoke_test,
            ready: ui.ready,
            busy: ui.busy,
            status: ui.status.clone(),
            error: ui.error.clone(),
            base_url: base_url.clone(),
            base_dict_dir: runtime.config.base_dict_dir.to_string_lossy().into_owned(),
            dictionaries,
            selected_dict_ids: ui.selected_ids.clone(),
            current_dict_id: ui.current_id.clone(),
            multi_mode: ui.multi_mode,
            suggestions: ui.suggestions.clone(),
            matches: ui.matches.clone(),
            current_word: ui.current_word.clone(),
            definitions: ui.definitions.clone(),
            zoom: ui.zoom,
            can_go_back,
            can_go_forward,
            notebooks: ui.notebooks.clone(),
            bookmarks: ui.bookmarks.clone(),
            selected_notebook_id: ui.selected_notebook_id.clone(),
            notebooks_busy: ui.notebooks_busy,
            ecdict: ui.ecdict.clone(),
            settings: ui.settings.clone(),
            css: ui.css.clone(),
        }
        .to_json()
    }

    pub fn begin_lookup(&self, word: &str, record_history: bool) -> (u64, String) {
        let sequence = self.lookup_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.current_word = word.to_string();
                runtime.ui.busy = true;
                runtime.ui.error.clear();
                runtime.ui.status = format!("Searching for {word}...");
                runtime.ui.definitions.clear();
                if record_history {
                    append_history(&mut runtime.ui, word);
                }
                runtime.config.set_string("lastword", word);
                let _ = runtime.config.save();
            }
        }
        (sequence, self.snapshot())
    }

    pub fn lookup_word(&self, word: &str, sequence: u64) -> Result<Option<String>> {
        if self.lookup_sequence.load(Ordering::SeqCst) != sequence {
            return Ok(None);
        }
        let (dictionaries, base_url) = {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            if runtime.ui.selected_ids.is_empty() {
                return Err(MedictError::Message(
                    "Select a dictionary first.".to_string(),
                ));
            }
            let selected = runtime
                .ui
                .selected_ids
                .iter()
                .filter_map(|id| {
                    runtime
                        .dictionaries
                        .iter()
                        .find(|dictionary| &dictionary.meta.id == id)
                        .cloned()
                })
                .collect::<Vec<_>>();
            (selected, read_lock(&self.base_url).clone())
        };

        let mut definitions = Vec::new();
        for dictionary in &dictionaries {
            match dictionary.resolve_entry(word) {
                Ok(Some(_entry)) => definitions.push(DefinitionView {
                    dict_id: dictionary.meta.id.clone(),
                    dict_name: dictionary.meta.name.clone(),
                    url: render::entry_route(&base_url, &dictionary.meta.id, word),
                    has_entry: true,
                    message: String::new(),
                }),
                Ok(None) => definitions.push(DefinitionView {
                    dict_id: dictionary.meta.id.clone(),
                    dict_name: dictionary.meta.name.clone(),
                    url: String::new(),
                    has_entry: false,
                    message: format!("No match for “{word}” in {}.", dictionary.meta.name),
                }),
                Err(error) => definitions.push(DefinitionView {
                    dict_id: dictionary.meta.id.clone(),
                    dict_name: dictionary.meta.name.clone(),
                    url: String::new(),
                    has_entry: false,
                    message: format!("Lookup failed in {}.\n\n{error}", dictionary.meta.name),
                }),
            }
        }

        let primary = {
            let guard = read_lock(&self.runtime);
            guard.as_ref().and_then(|runtime| {
                runtime
                    .dictionaries
                    .iter()
                    .find(|dictionary| dictionary.meta.id == runtime.ui.current_id)
                    .cloned()
            })
        };
        let suggestions = primary
            .as_ref()
            .and_then(|dictionary| dictionary.suggestions(word, 20).ok())
            .unwrap_or_default();

        if self.lookup_sequence.load(Ordering::SeqCst) != sequence {
            return Ok(None);
        }
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                let found = definitions.iter().filter(|value| value.has_entry).count();
                runtime.ui.definitions = definitions;
                runtime.ui.matches = dedupe_strings(suggestions.clone(), 20);
                runtime.ui.suggestions = suggestions;
                runtime.ui.busy = false;
                runtime.ui.status = if found == 0 {
                    format!("No dictionary returned a result for “{word}”.")
                } else if dictionaries.len() == 1 {
                    format!("{word} — {}", dictionaries[0].meta.name)
                } else {
                    format!("{word} — {found} of {} dictionaries", dictionaries.len())
                };
            }
        }
        Ok(Some(self.snapshot()))
    }

    pub fn suggest(&self, prefix: &str) -> Result<String> {
        let sequence = self.suggestion_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let dictionary = {
            let guard = read_lock(&self.runtime);
            let Some(runtime) = guard.as_ref() else {
                return Ok(self.snapshot());
            };
            runtime
                .dictionaries
                .iter()
                .find(|dictionary| dictionary.meta.id == runtime.ui.current_id)
                .cloned()
        };
        let suggestions = if prefix.trim().is_empty() {
            Vec::new()
        } else if let Some(dictionary) = dictionary {
            dictionary.suggestions(prefix, 50).unwrap_or_default()
        } else {
            Vec::new()
        };
        if self.suggestion_sequence.load(Ordering::SeqCst) == sequence {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.suggestions = suggestions.clone();
                runtime.ui.matches = suggestions;
            }
        }
        Ok(self.snapshot())
    }

    pub fn select_dictionary(&self, id: &str) -> Result<String> {
        let mut accepted = false;
        {
            let mut guard = write_lock(&self.runtime);
            let runtime = guard
                .as_mut()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            accepted = runtime
                .dictionaries
                .iter()
                .any(|dictionary| dictionary.meta.id == id);
            if accepted && runtime.ui.multi_mode {
                if let Some(position) = runtime.ui.selected_ids.iter().position(|value| value == id)
                {
                    if runtime.ui.selected_ids.len() > 1 {
                        runtime.ui.selected_ids.remove(position);
                        runtime.ui.current_id = runtime
                            .ui
                            .selected_ids
                            .last()
                            .cloned()
                            .unwrap_or_default();
                    } else {
                        runtime.ui.current_id = id.to_string();
                    }
                } else {
                    runtime.ui.selected_ids.push(id.to_string());
                    runtime.ui.current_id = id.to_string();
                }
            } else if accepted {
                runtime.ui.selected_ids = vec![id.to_string()];
                runtime.ui.current_id = id.to_string();
            }
            if accepted {
                save_selection(runtime);
            }
        }
        Ok(self.snapshot())
    }

    pub fn set_multi_mode(&self, enabled: bool) -> Result<String> {
        {
            let mut guard = write_lock(&self.runtime);
            let runtime = guard
                .as_mut()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime.ui.multi_mode = enabled;
            if !enabled && runtime.ui.selected_ids.len() > 1 {
                let first = runtime.ui.current_id.clone();
                let selected = if runtime.ui.selected_ids.contains(&first) {
                    first
                } else {
                    runtime.ui.selected_ids[0].clone()
                };
                runtime.ui.selected_ids = vec![selected.clone()];
                runtime.ui.current_id = selected;
            }
            save_selection(runtime);
        }
        Ok(self.snapshot())
    }

    pub fn current_word(&self) -> String {
        read_lock(&self.runtime)
            .as_ref()
            .map(|runtime| runtime.ui.current_word.clone())
            .unwrap_or_default()
    }

    pub fn navigate_history(&self, offset: isize) -> (Option<String>, String) {
        let word = {
            let mut guard = write_lock(&self.runtime);
            guard.as_mut().and_then(|runtime| {
                runtime.ui.history_position.and_then(|position| {
                    let next = position as isize + offset;
                    if next < 0 || next as usize >= runtime.ui.history.len() {
                        None
                    } else {
                        runtime.ui.history_position = Some(next as usize);
                        let word = runtime.ui.history[next as usize].clone();
                        runtime.ui.current_word = word.clone();
                        Some(word)
                    }
                })
            })
        };
        (word, self.snapshot())
    }

    pub fn set_zoom(&self, zoom: i32) -> String {
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.zoom = zoom.clamp(80, 160);
                runtime.config.set_integer("dictionarycontentzoom", runtime.ui.zoom as i64);
                let _ = runtime.config.save();
                runtime.ui.status = format!("Definition zoom: {}%", runtime.ui.zoom);
            }
        }
        self.snapshot()
    }

    pub fn refresh_dictionaries(&self) -> Result<String> {
        {
            let mut guard = write_lock(&self.runtime);
            let runtime = guard
                .as_mut()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime.ui.busy = true;
            runtime.ui.status = "Refreshing dictionaries...".to_string();
        }
        let (dictionaries, errors) = {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            scan_dictionaries(
                &runtime.config.base_dict_dir,
                runtime.ui.settings.allow_symlinks,
            )
        };
        {
            let mut guard = write_lock(&self.runtime);
            let runtime = guard
                .as_mut()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            let selected = runtime.ui.selected_ids.clone();
            let current = runtime.ui.current_id.clone();
            runtime.dictionaries = dictionaries;
            restore_selection(&mut runtime.ui, &runtime.dictionaries, &current, &selected);
            runtime.ui.busy = false;
            runtime.ui.status = if errors.is_empty() {
                "Dictionaries refreshed.".to_string()
            } else {
                format!("Dictionaries refreshed; {} could not be loaded.", errors.len())
            };
        }
        self.refresh_ecdict_status_inner()?;
        Ok(self.snapshot())
    }

    pub fn build_current_index(&self) -> Result<String> {
        let dictionary = self.current_dictionary()?;
        dictionary.build_index()?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.status = format!("Index rebuilt for {}.", dictionary.meta.name);
            }
        }
        Ok(self.snapshot())
    }

    pub fn add_bookmark(&self) -> Result<String> {
        let (word, dictionary) = {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            let word = runtime.ui.current_word.trim().to_string();
            if word.is_empty() {
                return Err(MedictError::Message(
                    "Look up a word before you save it.".to_string(),
                ));
            }
            let dictionary = runtime
                .dictionaries
                .iter()
                .find(|dictionary| dictionary.meta.id == runtime.ui.current_id)
                .cloned();
            (word, dictionary)
        };
        let Some(dictionary) = dictionary else {
            return Err(MedictError::Message(
                "Select a dictionary first.".to_string(),
            ));
        };
        let snapshot = self
            .render_snapshot_for(&dictionary, &word)
            .unwrap_or_default();
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime.bookmarks.lock().map_err(|_| {
                MedictError::Message("Bookmark database lock is poisoned.".to_string())
            })?.add(&word, &dictionary.meta.id, &dictionary.meta.name, "", &snapshot)?;
        }
        self.reload_notebooks_inner()?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.status = format!("Saved “{word}” to the default notebook.");
            }
        }
        Ok(self.snapshot())
    }

    pub fn load_notebooks(&self) -> Result<String> {
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.notebooks_busy = true;
            }
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    fn reload_notebooks_inner(&self) -> Result<()> {
        let (notebooks, bookmarks, default_id) = {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            let store = runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?;
            (store.notebooks()?, store.all()?, store.default_notebook_id()?)
        };
        let notebooks = notebooks
            .into_iter()
            .map(|notebook| NotebookView {
                id: notebook.id,
                name: notebook.name.clone(),
                is_default: notebook.is_default,
                created_at: notebook.created_at,
                display_name: if notebook.is_default {
                    format!("{} (default)", notebook.name)
                } else {
                    notebook.name
                },
            })
            .collect::<Vec<_>>();
        let bookmarks = bookmarks
            .into_iter()
            .map(|bookmark| BookmarkView {
                word: bookmark.word,
                dict_id: bookmark.dict_id,
                dict_name: bookmark.dict_name,
                notebook_id: bookmark.notebook_id,
                saved_at: bookmark.saved_at,
                saved_at_text: format_timestamp(bookmark.saved_at),
            })
            .collect::<Vec<_>>();
        let mut guard = write_lock(&self.runtime);
        let runtime = guard
            .as_mut()
            .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
        let selected = runtime.ui.selected_notebook_id.clone();
        runtime.ui.selected_notebook_id = if notebooks.iter().any(|value| value.id == selected) {
            selected
        } else {
            default_id
        };
        runtime.ui.notebooks = notebooks;
        runtime.ui.bookmarks = bookmarks;
        runtime.ui.notebooks_busy = false;
        Ok(())
    }

    pub fn select_notebook(&self, id: &str) -> String {
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.selected_notebook_id = id.to_string();
            }
        }
        self.snapshot()
    }

    pub fn create_notebook(&self, name: &str) -> Result<String> {
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?
                .create_notebook(name)?;
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    pub fn rename_notebook(&self, id: &str, name: &str) -> Result<String> {
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?
                .rename_notebook(id, name)?;
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    pub fn delete_notebook(&self, id: &str) -> Result<String> {
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            let mut store = runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?;
            store.remove_notebook(id)?;
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    pub fn set_default_notebook(&self, id: &str) -> Result<String> {
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            let mut store = runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?;
            store.set_default_notebook(id)?;
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    pub fn remove_bookmark(
        &self,
        word: &str,
        dict_id: &str,
        notebook_id: &str,
    ) -> Result<String> {
        {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?
                .remove(word, dict_id, notebook_id)?;
        }
        self.reload_notebooks_inner()?;
        Ok(self.snapshot())
    }

    pub fn activate_bookmark(&self, word: &str, dict_id: &str) -> Result<(u64, String)> {
        if !dict_id.is_empty() {
            self.select_dictionary(dict_id)?;
        }
        Ok(self.begin_lookup(word, true))
    }

    pub fn export_anki(&self, path: &str, notebook_id: &str) -> Result<String> {
        let rows = {
            let guard = read_lock(&self.runtime);
            let runtime = guard
                .as_ref()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime
                .bookmarks
                .lock()
                .map_err(|_| MedictError::Message("Bookmark database lock is poisoned.".to_string()))?
                .export_rows(notebook_id)?
        };
        let bytes = anki::build(&rows)?;
        atomic_write(Path::new(path), &bytes)?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.status = format!("Exported {path}");
            }
        }
        Ok(self.snapshot())
    }

    pub fn export_current_entry(&self, path: &str) -> Result<String> {
        let word = self.current_word();
        let dictionary = self.current_dictionary()?;
        if word.trim().is_empty() {
            return Err(MedictError::Message(
                "Look up a word before you export it.".to_string(),
            ));
        }
        let html = self.render_snapshot_for(&dictionary, &word)?;
        if html.is_empty() {
            return Err(MedictError::Message(
                "The entry was not found.".to_string(),
            ));
        }
        atomic_write(Path::new(path), html.as_bytes())?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.status = format!("Exported {path}");
            }
        }
        Ok(self.snapshot())
    }

    pub fn prepare_css_editor(&self) -> Result<String> {
        let dictionary = self.current_dictionary()?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.css = CssEditorView {
                    dict_id: dictionary.meta.id.clone(),
                    dict_name: dictionary.meta.name.clone(),
                    text: String::new(),
                    base_text: String::new(),
                    status: "Loading CSS...".to_string(),
                    busy: true,
                };
            }
        }
        let base = dictionary.base_css();
        let saved = read_lock(&self.runtime)
            .as_ref()
            .and_then(|runtime| runtime.user_css.get(&dictionary.meta.id).cloned())
            .unwrap_or_default();
        let text = if saved.is_empty() {
            base.clone()
        } else {
            saved
        };
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.css = CssEditorView {
                    dict_id: dictionary.meta.id.clone(),
                    dict_name: dictionary.meta.name.clone(),
                    text,
                    base_text: base,
                    status: if runtime
                        .user_css
                        .contains_key(&dictionary.meta.id)
                    {
                        "The editor shows the saved override.".to_string()
                    } else {
                        "The editor shows the dictionary CSS. Saving creates an override."
                            .to_string()
                    },
                    busy: false,
                };
            }
        }
        Ok(self.snapshot())
    }

    pub fn save_css(&self, css: &str) -> Result<String> {
        let dict_id = self.css_dictionary_id()?;
        if !safe_id(&dict_id) {
            return Err(MedictError::Message(
                "Invalid dictionary identifier.".to_string(),
            ));
        }
        let directory = app_config_dir()?.join("user_css");
        ensure_dir(&directory)?;
        let path = directory.join(format!("{dict_id}.css"));
        if css.is_empty() {
            if path.exists() {
                fs::remove_file(path)?;
            }
        } else {
            atomic_write(&path, css.as_bytes())?;
        }
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                if css.is_empty() {
                    runtime.user_css.remove(&dict_id);
                } else {
                    runtime.user_css.insert(dict_id.clone(), css.to_string());
                }
                runtime.ui.css.text = css.to_string();
                runtime.ui.css.status = "CSS saved.".to_string();
            }
        }
        Ok(self.snapshot())
    }

    pub fn remove_css(&self) -> Result<String> {
        self.save_css("")
    }

    pub fn restore_base_css(&self) -> String {
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.css.text = runtime.ui.css.base_text.clone();
                runtime.ui.css.status =
                    "The dictionary CSS was restored. Save to keep this result.".to_string();
            }
        }
        self.snapshot()
    }

    pub fn save_settings(&self, allow_symlinks: bool, group_limit: i64) -> Result<String> {
        {
            let mut guard = write_lock(&self.runtime);
            let runtime = guard
                .as_mut()
                .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
            runtime.ui.settings.allow_symlinks = allow_symlinks;
            runtime.ui.settings.group_limit = group_limit.clamp(1, 100);
            runtime
                .config
                .set_bool("dictionaryallowsymlinks", allow_symlinks);
            runtime
                .config
                .set_integer("dictionarygroupmax", runtime.ui.settings.group_limit);
            runtime.config.save()?;
            runtime.ui.status = "Settings saved.".to_string();
        }
        Ok(self.snapshot())
    }

    pub fn mark_ecdict_busy(&self) -> String {
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.ecdict.busy = true;
                runtime.ui.ecdict.status =
                    "Downloading and installing the full ECDICT data...".to_string();
            }
        }
        self.snapshot()
    }

    pub fn install_full_ecdict(&self) -> Result<String> {
        let dictionary = {
            let guard = read_lock(&self.runtime);
            guard.as_ref().and_then(|runtime| {
                runtime
                    .dictionaries
                    .iter()
                    .find(|dictionary| dictionary.meta.dict_type == "ecdict")
                    .cloned()
            })
        }
        .ok_or_else(|| MedictError::Message("ECDICT dictionary is not loaded.".to_string()))?;
        let count = dictionary.install_full_ecdict()?;
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.ecdict = EcdictView {
                    edition: "full".to_string(),
                    entry_count: count,
                    status: format!("Full edition: {count} entries."),
                    busy: false,
                    full_installed: true,
                };
                runtime.ui.status = "Full ECDICT installed.".to_string();
            }
        }
        Ok(self.snapshot())
    }

    pub fn mark_error(&self, error: impl Into<String>) -> String {
        let error = error.into();
        let has_runtime = read_lock(&self.runtime).is_some();
        if !has_runtime {
            *write_lock(&self.fatal_error) = error;
            return self.snapshot();
        }
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.busy = false;
                runtime.ui.notebooks_busy = false;
                runtime.ui.status = "The requested operation failed.".to_string();
                runtime.ui.error = error;
            }
        }
        self.snapshot()
    }

    pub fn clear_error(&self) -> String {
        *write_lock(&self.fatal_error) = String::new();
        {
            let mut guard = write_lock(&self.runtime);
            if let Some(runtime) = guard.as_mut() {
                runtime.ui.error.clear();
            }
        }
        self.snapshot()
    }

    pub fn open_dictionary_folder(&self) -> Result<String> {
        let directory = self
            .base_dictionary_directory()
            .ok_or_else(|| MedictError::Message("The dictionary folder is not ready.".to_string()))?;
        open::that(&directory)?;
        Ok(self.snapshot())
    }

    pub fn entry_html(&self, dict_id: &str, word: &str) -> Result<Option<String>> {
        let dictionary = {
            let guard = read_lock(&self.runtime);
            guard.as_ref().and_then(|runtime| {
                runtime
                    .dictionaries
                    .iter()
                    .find(|dictionary| dictionary.meta.id == dict_id)
                    .cloned()
            })
        };
        let Some(dictionary) = dictionary else {
            return Ok(None);
        };
        let Some(entry) = dictionary.resolve_entry(word)? else {
            return Ok(None);
        };
        let base_url = read_lock(&self.base_url).clone();
        let user_css = read_lock(&self.runtime)
            .as_ref()
            .and_then(|runtime| runtime.user_css.get(dict_id).cloned())
            .unwrap_or_default();
        Ok(Some(render::wrap_content(
            &dictionary,
            &entry.key,
            &entry.html,
            &base_url,
            &user_css,
        )))
    }

    pub fn resource(&self, dict_id: &str, path: &str) -> Option<Vec<u8>> {
        let dictionary = {
            let guard = read_lock(&self.runtime);
            guard.as_ref().and_then(|runtime| {
                runtime
                    .dictionaries
                    .iter()
                    .find(|dictionary| dictionary.meta.id == dict_id)
                    .cloned()
            })
        }?;
        dictionary.resource(path)
    }

    pub fn dictionary_directory(&self, dict_id: &str) -> Option<PathBuf> {
        let guard = read_lock(&self.runtime);
        guard.as_ref().and_then(|runtime| {
            runtime
                .dictionaries
                .iter()
                .find(|dictionary| dictionary.meta.id == dict_id)
                .map(|dictionary| dictionary.meta.directory.clone())
        })
    }

    pub fn base_dictionary_directory(&self) -> Option<PathBuf> {
        read_lock(&self.runtime)
            .as_ref()
            .map(|runtime| runtime.config.base_dict_dir.clone())
    }

    fn current_dictionary(&self) -> Result<Arc<Dictionary>> {
        let guard = read_lock(&self.runtime);
        let runtime = guard
            .as_ref()
            .ok_or_else(|| MedictError::Message("The engine is not ready.".to_string()))?;
        runtime
            .dictionaries
            .iter()
            .find(|dictionary| dictionary.meta.id == runtime.ui.current_id)
            .cloned()
            .ok_or_else(|| MedictError::Message("Select a dictionary first.".to_string()))
    }

    fn css_dictionary_id(&self) -> Result<String> {
        let id = read_lock(&self.runtime)
            .as_ref()
            .map(|runtime| runtime.ui.css.dict_id.clone())
            .unwrap_or_default();
        if id.is_empty() {
            Err(MedictError::Message(
                "Open a dictionary CSS editor first.".to_string(),
            ))
        } else {
            Ok(id)
        }
    }

    fn render_snapshot_for(&self, dictionary: &Dictionary, word: &str) -> Result<String> {
        let Some(entry) = dictionary.resolve_entry(word)? else {
            return Ok(String::new());
        };
        let base_url = read_lock(&self.base_url).clone();
        let user_css = read_lock(&self.runtime)
            .as_ref()
            .and_then(|runtime| runtime.user_css.get(&dictionary.meta.id).cloned())
            .unwrap_or_default();
        let html = render::wrap_content(
            dictionary,
            &entry.key,
            &entry.html,
            &base_url,
            &user_css,
        );
        Ok(render::snapshot_html(dictionary, &html))
    }

    fn refresh_ecdict_status_inner(&self) -> Result<()> {
        let status = {
            let guard = read_lock(&self.runtime);
            let Some(runtime) = guard.as_ref() else {
                return Ok(());
            };
            runtime
                .dictionaries
                .iter()
                .find(|dictionary| dictionary.meta.dict_type == "ecdict")
                .and_then(|dictionary| dictionary.ecdict_status())
        };
        let mut guard = write_lock(&self.runtime);
        if let Some(runtime) = guard.as_mut() {
            match status {
                Some(Ok((edition, count))) => {
                    runtime.ui.ecdict = EcdictView {
                        full_installed: edition == "full",
                        status: if edition == "full" {
                            format!("Full edition: {count} entries.")
                        } else {
                            format!("Compact edition: {count} entries.")
                        },
                        edition,
                        entry_count: count,
                        busy: false,
                    };
                }
                Some(Err(error)) => {
                    runtime.ui.ecdict.status = format!("ECDICT status failed: {error}");
                }
                None => {
                    runtime.ui.ecdict.status = "ECDICT dictionary is not loaded.".to_string();
                }
            }
        }
        Ok(())
    }
}

fn append_history(ui: &mut UiState, word: &str) {
    let word = word.trim();
    if word.is_empty() {
        return;
    }
    if let Some(position) = ui.history_position {
        if ui.history.get(position).is_some_and(|value| value == word) {
            return;
        }
    }
    if let Some(position) = ui.history_position {
        ui.history.truncate(position + 1);
    }
    ui.history.push(word.to_string());
    if ui.history.len() > 100 {
        ui.history.remove(0);
    }
    ui.history_position = Some(ui.history.len().saturating_sub(1));
}

fn restore_selection(
    ui: &mut UiState,
    dictionaries: &[Arc<Dictionary>],
    preferred_id: &str,
    saved_ids: &[String],
) {
    let available = dictionaries
        .iter()
        .map(|dictionary| dictionary.meta.id.clone())
        .collect::<Vec<_>>();
    if available.is_empty() {
        ui.selected_ids.clear();
        ui.current_id.clear();
        return;
    }
    let mut wanted = saved_ids
        .iter()
        .filter(|id| available.contains(id))
        .cloned()
        .collect::<Vec<_>>();
    wanted = dedupe_strings(wanted, 100);
    if ui.multi_mode && !wanted.is_empty() {
        ui.selected_ids = wanted;
        ui.current_id = if available.contains(&preferred_id.to_string()) {
            preferred_id.to_string()
        } else {
            ui.selected_ids[0].clone()
        };
    } else if available.contains(&preferred_id.to_string()) {
        ui.selected_ids = vec![preferred_id.to_string()];
        ui.current_id = preferred_id.to_string();
    } else {
        ui.current_id = available[0].clone();
        ui.selected_ids = vec![ui.current_id.clone()];
    }
}

fn save_selection(runtime: &mut Runtime) {
    runtime
        .config
        .set_string("selecteddictid", runtime.ui.current_id.clone());
    runtime.config.set_bool("multimode", runtime.ui.multi_mode);
    runtime
        .config
        .set_strings("multidictids", &runtime.ui.selected_ids);
    let _ = runtime.config.save();
}

fn scan_dictionaries(base: &Path, allow_symlinks: bool) -> (Vec<Arc<Dictionary>>, Vec<String>) {
    let mut files_by_dir: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let walker = walkdir::WalkDir::new(base)
        .follow_links(allow_symlinks)
        .max_depth(12)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !name.starts_with(".melev")
                && !name.starts_with(".medict")
                && name != "target"
                && name != "node_modules"
        });
    for entry in walker.filter_map(std::result::Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(parent) = entry.path().parent() {
            files_by_dir
                .entry(parent.to_path_buf())
                .or_default()
                .push(entry.path().to_path_buf());
        }
    }

    let mut dictionaries = Vec::new();
    let mut errors = Vec::new();
    for (directory, mut files) in files_by_dir {
        files.sort();
        let ecdict = files
            .iter()
            .find(|path| path.file_name().is_some_and(|name| name == "ecdict.db"));
        if let Some(database) = ecdict {
            match open_ecdict(directory.clone(), database.clone()) {
                Ok(dictionary) => dictionaries.push(Arc::new(dictionary)),
                Err(error) => errors.push(format!("{}: {error}", directory.display())),
            }
            continue;
        }
        if let Some(mdx) = files.iter().find(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("mdx"))
        }) {
            match open_mdict(directory.clone(), mdx.clone()) {
                Ok(dictionary) => dictionaries.push(Arc::new(dictionary)),
                Err(error) => errors.push(format!("{}: {error}", directory.display())),
            }
            continue;
        }
        if let Some(ifo) = files.iter().find(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("ifo"))
        }) {
            match open_stardict(directory.clone(), ifo.clone()) {
                Ok(dictionary) => dictionaries.push(Arc::new(dictionary)),
                Err(error) => errors.push(format!("{}: {error}", directory.display())),
            }
        }
    }
    dictionaries.sort_by(|left, right| {
        left.meta
            .name
            .to_lowercase()
            .cmp(&right.meta.name.to_lowercase())
    });
    (dictionaries, errors)
}

fn install_presets(base: &Path) -> Result<()> {
    write_if_missing(&base.join("cc-cedict").join("cc-cedict.mdx"), PRESET_CC_MDX)?;
    write_if_missing(&base.join("cc-cedict").join("cc-cedict.mdd"), PRESET_CC_MDD)?;
    write_if_missing(&base.join("cc-cedict").join("cc-cedict.css"), PRESET_CC_CSS)?;
    write_if_missing(&base.join("cc-cedict").join("cover.png"), PRESET_CC_COVER)?;
    write_if_missing(&base.join("ecdict").join("ecdict.db"), PRESET_ECDICT)?;
    Ok(())
}

fn write_if_missing(path: &Path, bytes: &[u8]) -> Result<()> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            ensure_dir(parent)?;
        }
        atomic_write(path, bytes)?;
    }
    Ok(())
}

fn load_user_css(config_dir: &Path) -> HashMap<String, String> {
    let directory = config_dir.join("user_css");
    let Ok(entries) = fs::read_dir(directory) else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("css"))
            {
                let id = path.file_stem()?.to_string_lossy().into_owned();
                let css = fs::read_to_string(path).ok()?;
                Some((id, css))
            } else {
                None
            }
        })
        .collect()
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|error| error.into_inner())
}

fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|error| error.into_inner())
}

pub fn version_notes() -> Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "product": "Medict",
        "license": "GPL-3.0-or-later",
    })
}

pub fn dictionary_route(base_url: &str, dict_id: &str, word: &str) -> String {
    format!(
        "{base_url}entry?dict_id={}&word={}",
        query_escape(dict_id),
        query_escape(word)
    )
}

pub fn generated_dictionary_id(path: &Path) -> String {
    stable_id(path)
}
