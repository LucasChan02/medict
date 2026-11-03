use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryView {
    pub id: String,
    pub name: String,
    pub dict_type: String,
    pub directory: String,
    pub description: String,
    pub selected: bool,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionView {
    pub dict_id: String,
    pub dict_name: String,
    pub url: String,
    pub has_entry: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookView {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub created_at: i64,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkView {
    pub word: String,
    pub dict_id: String,
    pub dict_name: String,
    pub notebook_id: String,
    pub saved_at: i64,
    pub saved_at_text: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EcdictView {
    pub edition: String,
    pub entry_count: i64,
    pub status: String,
    pub busy: bool,
    pub full_installed: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub allow_symlinks: bool,
    pub group_limit: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CssEditorView {
    pub dict_id: String,
    pub dict_name: String,
    pub text: String,
    pub base_text: String,
    pub status: String,
    pub busy: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSnapshot {
    pub version: String,
    pub platform: String,
    pub smoke_test: bool,
    pub ready: bool,
    pub busy: bool,
    pub status: String,
    pub error: String,
    pub base_url: String,
    pub base_dict_dir: String,
    pub dictionaries: Vec<DictionaryView>,
    pub selected_dict_ids: Vec<String>,
    pub current_dict_id: String,
    pub multi_mode: bool,
    pub suggestions: Vec<String>,
    pub matches: Vec<String>,
    pub current_word: String,
    pub definitions: Vec<DefinitionView>,
    pub zoom: i32,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub notebooks: Vec<NotebookView>,
    pub bookmarks: Vec<BookmarkView>,
    pub selected_notebook_id: String,
    pub notebooks_busy: bool,
    pub ecdict: EcdictView,
    pub settings: SettingsView,
    pub css: CssEditorView,
}

impl UiSnapshot {
    pub fn new(version: impl Into<String>, smoke_test: bool) -> Self {
        Self {
            version: version.into(),
            platform: std::env::consts::OS.to_string(),
            smoke_test,
            ready: false,
            busy: false,
            status: "Starting the dictionary engine...".to_string(),
            error: String::new(),
            base_url: String::new(),
            base_dict_dir: String::new(),
            dictionaries: Vec::new(),
            selected_dict_ids: Vec::new(),
            current_dict_id: String::new(),
            multi_mode: false,
            suggestions: Vec::new(),
            matches: Vec::new(),
            current_word: String::new(),
            definitions: Vec::new(),
            zoom: 100,
            can_go_back: false,
            can_go_forward: false,
            notebooks: Vec::new(),
            bookmarks: Vec::new(),
            selected_notebook_id: String::new(),
            notebooks_busy: false,
            ecdict: EcdictView {
                edition: "compact".to_string(),
                entry_count: 0,
                status: "ECDICT status is not available.".to_string(),
                busy: false,
                full_installed: false,
            },
            settings: SettingsView {
                allow_symlinks: true,
                group_limit: 10,
            },
            css: CssEditorView {
                dict_id: String::new(),
                dict_name: String::new(),
                text: String::new(),
                base_text: String::new(),
                status: String::new(),
                busy: false,
            },
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}
