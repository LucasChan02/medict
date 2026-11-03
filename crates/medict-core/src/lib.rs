pub mod anki;
pub mod bookmarks;
pub mod config;
pub mod dictionary;
pub mod error;
pub mod model;
pub mod render;
pub mod server;
pub mod state;
pub mod util;

pub use error::{MedictError, Result};
pub use state::AppCore;
