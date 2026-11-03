use std::sync::Arc;
use std::thread;

use medict_core::AppCore;
use medict_core::util::local_path_from_url;
use qtbridge::{QThreadHandle, QmlObject, qobject, qproperty, qsignal, qslot};

use crate::CORE;

pub struct Controller {
    core: Arc<AppCore>,
    state_json: String,
    smoke_test: bool,
}

impl Controller {
    fn apply_result(&mut self, result: medict_core::Result<String>) {
        let state = match result {
            Ok(state) => state,
            Err(error) => self.core.mark_error(error.to_string()),
        };
        self.set_state_json(state);
    }

    fn launch_lookup(&mut self, word: String, append_history: bool) {
        let word = word.trim().to_string();
        if word.is_empty() {
            return;
        }
        let (sequence, state) = self.core.begin_lookup(&word, append_history);
        self.set_state_json(state);
        let core = Arc::clone(&self.core);
        let handle = self.get_thread_handle();
        thread::spawn(move || {
            let state = match core.lookup_word(&word, sequence) {
                Ok(Some(state)) => state,
                Ok(None) => return,
                Err(error) => core.mark_error(error.to_string()),
            };
            handle.invoke_mut(move |controller| controller.set_state_json(state));
        });
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl Controller {
    qproperty!(
        "stateJson",
        Read = state_json,
        Write = set_state_json,
        Notify = state_changed
    );
    qproperty!("smokeTest", Member = smoke_test, Constant);

    #[qsignal]
    fn state_changed(&mut self);

    fn state_json(&self) -> String {
        self.state_json.clone()
    }

    fn set_state_json(&mut self, value: String) {
        self.state_json = value;
        self.state_changed();
    }

    #[qslot]
    fn start(&mut self) {
        let state = self.core.snapshot();
        self.set_state_json(state);
        let core = Arc::clone(&self.core);
        let handle = self.get_thread_handle();
        thread::spawn(move || {
            let state = match core.bootstrap() {
                Ok(state) => state,
                Err(error) => core.mark_error(error.to_string()),
            };
            handle.invoke_mut(move |controller| controller.set_state_json(state));

            let word = core.last_word();
            if !word.is_empty() {
                let (sequence, state) = core.begin_lookup(&word, true);
                handle.invoke_mut(move |controller| controller.set_state_json(state));
                let state = match core.lookup_word(&word, sequence) {
                    Ok(Some(state)) => state,
                    Ok(None) => return,
                    Err(error) => core.mark_error(error.to_string()),
                };
                handle.invoke_mut(move |controller| controller.set_state_json(state));
            }
        });
    }

    #[qslot]
    fn lookup(&mut self, word: String) {
        self.launch_lookup(word, true);
    }

    #[qslot]
    fn suggest(&mut self, prefix: String) {
        let core = Arc::clone(&self.core);
        let handle = self.get_thread_handle();
        thread::spawn(move || {
            let state = match core.suggest(&prefix) {
                Ok(state) => state,
                Err(error) => core.mark_error(error.to_string()),
            };
            handle.invoke_mut(move |controller| controller.set_state_json(state));
        });
    }

    #[qslot]
    fn select_dictionary(&mut self, id: String) {
        let result = self.core.select_dictionary(&id);
        self.apply_result(result);
        let word = self.core.current_word();
        if !word.is_empty() {
            self.launch_lookup(word, false);
        }
    }

    #[qslot]
    fn set_multi_mode(&mut self, enabled: bool) {
        let result = self.core.set_multi_mode(enabled);
        self.apply_result(result);
        let word = self.core.current_word();
        if !word.is_empty() {
            self.launch_lookup(word, false);
        }
    }

    #[qslot]
    fn refresh_dictionaries(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.refresh_dictionaries()
        });
    }

    #[qslot]
    fn rebuild_index(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.build_current_index()
        });
    }

    #[qslot]
    fn back(&mut self) {
        let (word, state) = self.core.navigate_history(-1);
        self.set_state_json(state);
        if let Some(word) = word {
            self.launch_lookup(word, false);
        }
    }

    #[qslot]
    fn forward(&mut self) {
        let (word, state) = self.core.navigate_history(1);
        self.set_state_json(state);
        if let Some(word) = word {
            self.launch_lookup(word, false);
        }
    }

    #[qslot]
    fn add_bookmark(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.add_bookmark()
        });
    }

    #[qslot]
    fn load_notebooks(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.load_notebooks()
        });
    }

    #[qslot]
    fn select_notebook(&mut self, id: String) {
        let state = self.core.select_notebook(&id);
        self.set_state_json(state);
    }

    #[qslot]
    fn create_notebook(&mut self, name: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.create_notebook(&name)
        });
    }

    #[qslot]
    fn rename_notebook(&mut self, id: String, name: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.rename_notebook(&id, &name)
        });
    }

    #[qslot]
    fn delete_notebook(&mut self, id: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.delete_notebook(&id)
        });
    }

    #[qslot]
    fn set_default_notebook(&mut self, id: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.set_default_notebook(&id)
        });
    }

    #[qslot]
    fn remove_bookmark(&mut self, word: String, dict_id: String, notebook_id: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.remove_bookmark(&word, &dict_id, &notebook_id)
        });
    }

    #[qslot]
    fn activate_bookmark(&mut self, word: String, dict_id: String) {
        if !dict_id.is_empty() {
            let result = self.core.select_dictionary(&dict_id);
            self.apply_result(result);
        }
        self.launch_lookup(word, true);
    }

    #[qslot]
    fn export_anki(&mut self, path: String, notebook_id: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.export_anki(&path, &notebook_id)
        });
    }

    #[qslot]
    fn export_entry(&mut self, path: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.export_current_entry(&path)
        });
    }

    #[qslot]
    fn prepare_css_editor(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.prepare_css_editor()
        });
    }

    #[qslot]
    fn save_css(&mut self, text: String) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.save_css(&text)
        });
    }

    #[qslot]
    fn remove_css(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.remove_css()
        });
    }

    #[qslot]
    fn restore_base_css(&mut self) {
        let state = self.core.restore_base_css();
        self.set_state_json(state);
    }

    #[qslot]
    fn save_settings(&mut self, allow_symlinks: bool, group_limit: i32) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), move |core| {
            core.save_settings(allow_symlinks, group_limit as i64)
        });
    }

    #[qslot]
    fn install_ecdict(&mut self) {
        let state = self.core.mark_ecdict_busy();
        self.set_state_json(state);
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.install_full_ecdict()
        });
    }

    #[qslot]
    fn open_dictionary_folder(&mut self) {
        spawn_result(self.get_thread_handle(), Arc::clone(&self.core), |core| {
            core.open_dictionary_folder()
        });
    }

    #[qslot]
    fn set_zoom(&mut self, zoom: i32) {
        let state = self.core.set_zoom(zoom);
        self.set_state_json(state);
    }

    #[qslot]
    fn dismiss_error(&mut self) {
        let state = self.core.clear_error();
        self.set_state_json(state);
    }

    #[qslot]
    fn local_file(&self, value: String) -> String {
        local_path_from_url(&value)
    }

}

fn spawn_result<F>(
    handle: QThreadHandle<Controller>,
    core: Arc<AppCore>,
    action: F,
) where
    F: FnOnce(&AppCore) -> medict_core::Result<String> + Send + 'static,
{
    thread::spawn(move || {
        let state = match action(&core) {
            Ok(state) => state,
            Err(error) => core.mark_error(error.to_string()),
        };
        handle.invoke_mut(move |controller| controller.set_state_json(state));
    });
}

impl Default for Controller {
    fn default() -> Self {
        let core = CORE
            .get()
            .cloned()
            .unwrap_or_else(|| Arc::new(AppCore::new(env!("CARGO_PKG_VERSION"), false)));
        let state_json = core.snapshot();
        let smoke_test = core.smoke_test();
        Self {
            core,
            state_json,
            smoke_test,
        }
    }
}
