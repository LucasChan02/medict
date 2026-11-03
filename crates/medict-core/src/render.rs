use std::path::Path;
use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use regex::{Captures, Regex};

use crate::dictionary::Dictionary;
use crate::util::{mime_for_path, query_escape};

static ATTRIBUTE_RESOURCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(src|href)\s*=\s*"([^"]+)""#).expect("valid resource attribute regex")
});
static CSS_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"url\(\s*['"]?([^'")]+)['"]?\s*\)"#).expect("valid CSS URL regex"));

pub fn wrap_content(
    dictionary: &Dictionary,
    key: &str,
    definition: &str,
    base_url: &str,
    user_css: &str,
) -> String {
    let dict_id = &dictionary.meta.id;
    let dictionary_base = format!("{base_url}resource/{dict_id}/");
    let base_css = dictionary.base_css();
    let title =
        html_escape::encode_text(&format!("{} — {}", key, dictionary.meta.name)).into_owned();
    let safe_user_css = user_css.replace("</style", r"<\/style");
    let safe_base_css = base_css.replace("</style", r"<\/style");
    let javascript_bridge = format!(
        r#"
(() => {{
  const resourceBase = {resource_base_json};
  function lookupWord(word) {{
    word = String(word || '').trim();
    if (!word) return;
    window.location.href = 'medict-lookup://?word=' + encodeURIComponent(word);
  }}
  document.addEventListener('click', event => {{
    const anchor = event.target && event.target.closest ? event.target.closest('a') : null;
    if (!anchor) return;
    const href = anchor.getAttribute('href') || '';
    if (href.startsWith('sound://')) {{
      event.preventDefault();
      const path = href.slice('sound://'.length);
      try {{
        const audio = new Audio(new URL(path, resourceBase).href);
        audio.play();
      }} catch (_) {{}}
      return;
    }}
    if (href.startsWith('entry://')) {{
      event.preventDefault();
      let word = href.slice('entry://'.length);
      try {{ word = decodeURIComponent(word); }} catch (_) {{}}
      lookupWord(word.split('#')[0]);
    }}
  }});
  document.addEventListener('dblclick', () => {{
    const selection = window.getSelection ? String(window.getSelection()) : '';
    const word = selection.trim();
    if (word && !word.includes('\n')) lookupWord(word);
  }});
}})();
"#,
        resource_base_json = serde_json::to_string(&dictionary_base).unwrap_or_else(|_| "\"\"".into())
    );

    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<base href="{dictionary_base}">
<title>{title}</title>
<style>
html {{ color-scheme: light dark; background: transparent; }}
body {{ margin: 0; padding: .55rem .65rem; background: transparent; }}
body:not([style]) {{ font-family: system-ui, "Noto Sans", "Segoe UI", sans-serif; }}
img, video {{ max-width: 100%; height: auto; }}
::selection {{ background: color-mix(in srgb, Highlight 35%, transparent); }}
</style>
<style id="medict-dictionary-css">{safe_base_css}</style>
<style id="medict-user-css">{safe_user_css}</style>
<script>{javascript_bridge}</script>
</head>
<body>{definition}</body>
</html>"#
    )
}

pub fn snapshot_html(dictionary: &Dictionary, html: &str) -> String {
    inline_resources(html, dictionary)
}

pub fn inline_resources(html: &str, dictionary: &Dictionary) -> String {
    let mut cache = std::collections::HashMap::<String, Option<Vec<u8>>>::new();
    ATTRIBUTE_RESOURCE
        .replace_all(html, |captures: &Captures<'_>| {
            let attribute = &captures[1];
            let key = &captures[2];
            if skip_resource(key) {
                return captures[0].to_string();
            }
            let bytes = cache
                .entry(key.to_string())
                .or_insert_with(|| dictionary.resource(key))
                .clone();
            let Some(bytes) = bytes else {
                return captures[0].to_string();
            };
            if key.to_ascii_lowercase().ends_with(".css") {
                let css = String::from_utf8_lossy(&bytes);
                let css = inline_css_urls(&css, dictionary, Path::new(key).parent());
                return format!(
                    r#"{attribute}="data:text/css;base64,{}""#,
                    BASE64.encode(css.as_bytes())
                );
            }
            format!(
                r#"{attribute}="data:{};base64,{}""#,
                mime_for_path(Path::new(key)),
                BASE64.encode(bytes)
            )
        })
        .into_owned()
}

fn inline_css_urls(css: &str, dictionary: &Dictionary, base: Option<&Path>) -> String {
    CSS_URL
        .replace_all(css, |captures: &Captures<'_>| {
            let key = captures[1].trim();
            if skip_resource(key) {
                return captures[0].to_string();
            }
            let relative = base
                .map(|base| base.join(key))
                .unwrap_or_else(|| Path::new(key).to_path_buf());
            let candidate = relative.to_string_lossy().replace('\\', "/");
            let Some(bytes) = dictionary.resource(&candidate) else {
                return captures[0].to_string();
            };
            format!(
                "url(data:{};base64,{})",
                mime_for_path(Path::new(key)),
                BASE64.encode(bytes)
            )
        })
        .into_owned()
}

fn skip_resource(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.starts_with("data:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("//")
        || lower.starts_with('#')
        || lower.starts_with("javascript:")
        || lower.starts_with("entry://")
        || lower.starts_with("sound://")
}

pub fn definition_body(html: &str) -> String {
    static BODY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<body[^>]*>(.*?)</body>").expect("valid body regex"));
    BODY.captures(html)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .unwrap_or_else(|| html.to_string())
}

pub fn entry_route(base_url: &str, dict_id: &str, word: &str) -> String {
    format!(
        "{base_url}entry?dict_id={}&word={}",
        query_escape(dict_id),
        query_escape(word)
    )
}
