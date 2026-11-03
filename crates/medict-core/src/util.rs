use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

pub fn home_dir() -> io::Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("USERPROFILE") {
            return Ok(PathBuf::from(path));
        }
    }
    if let Some(path) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(path));
    }
    std::env::current_dir()
}

pub fn app_config_dir() -> io::Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(path).join("medict"));
        }
        if let Some(path) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(path).join("medict"));
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(path).join("medict"));
        }
        return Ok(home_dir()?.join(".config").join("medict"));
    }

    #[allow(unreachable_code)]
    Ok(home_dir()?.join(".medict"))
}

pub fn stable_id(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("{:x}", md5::compute(canonical.to_string_lossy().as_bytes()))
}

pub fn path_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn ensure_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(data)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .map(|_| ())
}

pub fn safe_join(base: &Path, key: &str) -> Option<PathBuf> {
    let key = key.replace('\\', "/");
    let key = key.trim_start_matches('/');
    if key.is_empty() {
        return None;
    }
    let relative = Path::new(key);
    for component in relative.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => return None,
        }
    }
    Some(base.join(relative))
}

pub fn query_escape(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub fn local_path_from_url(value: &str) -> String {
    if let Ok(url) = url::Url::parse(value) {
        if url.scheme() == "file" {
            if let Ok(path) = url.to_file_path() {
                return path.to_string_lossy().into_owned();
            }
        }
    }
    value.to_string()
}

pub fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "css" | "scss" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "eot" => "application/vnd.ms-fontobject",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn temp_os_string(value: &str) -> OsString {
    OsString::from(value)
}

pub fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn dedupe_strings(values: impl IntoIterator<Item = String>, limit: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in values {
        if result.len() >= limit {
            break;
        }
        if value.is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        result.push(value);
    }
    result
}
