mod controller;

use std::sync::{Arc, OnceLock};

use medict_core::AppCore;
use qtbridge::{QApp, include_bytes_qml};

pub(crate) static CORE: OnceLock<Arc<AppCore>> = OnceLock::new();

fn main() {
    let smoke_test = std::env::args().any(|argument| argument == "--smoke-test");
    let core = Arc::new(AppCore::new(env!("CARGO_PKG_VERSION"), smoke_test));
    if let Err(error) = medict_core::server::start(Arc::clone(&core)) {
        eprintln!("medict: {error}");
        let _ = core.mark_error(error.to_string());
    }
    let _ = CORE.set(core);

    include_bytes_qml!("qml", "qt/qml/medict_app");

    QApp::new()
        .register::<controller::Controller>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file("qrc:/qt/qml/medict_app/Main.qml")
        .run();
}
