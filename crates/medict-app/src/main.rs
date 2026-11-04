mod controller;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use medict_core::AppCore;
use qtbridge::{QApp, include_bytes_qml};

pub(crate) static CORE: OnceLock<Arc<AppCore>> = OnceLock::new();
pub(crate) static UI_READY: AtomicBool = AtomicBool::new(false);
pub(crate) static SMOKE_COMPLETE: AtomicBool = AtomicBool::new(false);

fn main() {
    let smoke_test = std::env::args().any(|argument| argument == "--smoke-test")
        || std::env::var_os("MEDICT_SMOKE_TEST").is_some();
    let core = Arc::new(AppCore::new(env!("CARGO_PKG_VERSION"), smoke_test));
    if smoke_test {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(75));
            eprintln!(
                "medict: smoke watchdog expired (ui_ready={}, complete={})",
                UI_READY.load(Ordering::SeqCst),
                SMOKE_COMPLETE.load(Ordering::SeqCst)
            );
            std::process::exit(3);
        });
    }
    if let Err(error) = medict_core::server::start(Arc::clone(&core)) {
        eprintln!("medict: {error}");
        let _ = core.mark_error(error.to_string());
    }
    let _ = CORE.set(core);

    include_bytes_qml!("qml", "qt");

    let exit_code = QApp::new()
        .register::<controller::Controller>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file("qrc:/qt/qml/Main.qml")
        .run();

    if !UI_READY.load(Ordering::SeqCst) {
        eprintln!("medict: the QML interface did not load");
        std::process::exit(2);
    }
    if smoke_test && !SMOKE_COMPLETE.load(Ordering::SeqCst) {
        eprintln!("medict: the QML feature smoke test did not complete");
        std::process::exit(3);
    }
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
