use std::path::PathBuf;

use medict_core::bookmarks::BookmarkStore;
use medict_core::dictionary::{open_ecdict, open_mdict, open_stardict};

fn preset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/presets")
        .join(name)
}

#[test]
fn opens_mdict_preset_and_resolves_an_entry() {
    let directory = preset("cc-cedict");
    let dictionary = open_mdict(directory.clone(), directory.join("cc-cedict.mdx")).unwrap();
    assert!(!dictionary.meta.name.is_empty());

    let entry = dictionary.resolve_entry("中国");
    assert!(entry.is_ok());
}

#[test]
fn opens_ecdict_preset_and_renders_a_card() {
    let directory = preset("ecdict");
    let dictionary = open_ecdict(directory.clone(), directory.join("ecdict.db")).unwrap();
    let entry = dictionary.resolve_entry("part").unwrap();
    assert!(entry.is_some());
    assert!(entry.unwrap().html.contains("ec-card"));
}

#[test]
fn opens_stardict_fixture() {
    let directory = preset("dictd-elements");
    let dictionary =
        open_stardict(directory.clone(), directory.join("dictd-elements.ifo")).unwrap();
    let entry = dictionary.resolve_entry("carbon").unwrap();
    assert!(entry.is_some());
}

#[test]
fn stores_and_removes_notebooks_and_bookmarks() {
    let directory = tempfile::tempdir().unwrap();
    let store = BookmarkStore::open(directory.path()).unwrap();
    let default_id = store.default_notebook_id().unwrap();
    assert!(!default_id.is_empty());

    store
        .add("word", "dict", "Dictionary", &default_id, "<p>snapshot</p>")
        .unwrap();
    assert_eq!(store.all().unwrap().len(), 1);
    assert_eq!(
        store.snapshot("word", "dict", &default_id).unwrap(),
        "<p>snapshot</p>"
    );

    let extra = store.create_notebook("Extra").unwrap();
    assert_eq!(store.notebooks().unwrap().len(), 2);
    store.rename_notebook(&extra.id, "Renamed").unwrap();
    store.remove("word", "dict", &default_id).unwrap();
    assert!(store.all().unwrap().is_empty());
}
