use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use toml::Table;
use toml::Value as TomlValue;

use crate::util::{app_config_dir, atomic_write, ensure_dir, home_dir};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub path: PathBuf,
    pub base_dict_dir: PathBuf,
    values: BTreeMap<String, TomlValue>,
}

impl AppConfig {
    pub fn load() -> io::Result<Self> {
        let config_dir = app_config_dir()?;
        ensure_dir(&config_dir)?;
        let path = config_dir.join("medict.toml");
        if !path.exists() {
            atomic_write(&path, b"BaseDictDir=\"$HOME/dicts\"\n")?;
        }

        let text = fs::read_to_string(&path)?;
        let table = text.parse::<Table>().unwrap_or_default();
        let mut values = BTreeMap::new();
        let mut configured_dict_dir = String::new();
        for (key, value) in table {
            if key.eq_ignore_ascii_case("BaseDictDir") {
                if let TomlValue::String(value) = &value {
                    configured_dict_dir = value.clone();
                }
            }
            values.insert(key.to_ascii_lowercase(), value);
        }

        let base_dict_dir = resolve_config_path(&configured_dict_dir, &config_dir)?;
        ensure_dir(&base_dict_dir)?;
        Ok(Self {
            path,
            base_dict_dir,
            values,
        })
    }

    pub fn save(&self) -> io::Result<()> {
        let mut table = Table::new();
        table.insert(
            "BaseDictDir".to_string(),
            TomlValue::String(self.base_dict_dir.to_string_lossy().into_owned()),
        );
        let mut keys: Vec<_> = self.values.keys().cloned().collect();
        keys.sort();
        for key in keys {
            if key == "basedictdir" {
                continue;
            }
            if let Some(value) = self.values.get(&key) {
                table.insert(key, value.clone());
            }
        }
        let text = toml::to_string_pretty(&table)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        atomic_write(&self.path, text.as_bytes())
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.values
            .get(&key.to_ascii_lowercase())
            .and_then(toml_bool)
            .unwrap_or(default)
    }

    pub fn integer(&self, key: &str, default: i64) -> i64 {
        self.values
            .get(&key.to_ascii_lowercase())
            .and_then(toml_integer)
            .unwrap_or(default)
    }

    pub fn string(&self, key: &str, default: &str) -> String {
        self.values
            .get(&key.to_ascii_lowercase())
            .and_then(TomlValue::as_str)
            .unwrap_or(default)
            .to_string()
    }

    pub fn strings(&self, key: &str) -> Vec<String> {
        self.values
            .get(&key.to_ascii_lowercase())
            .and_then(TomlValue::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(TomlValue::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.values.insert(
            key.to_ascii_lowercase(),
            TomlValue::String(if value { "true" } else { "false" }.to_string()),
        );
    }

    pub fn set_integer(&mut self, key: &str, value: i64) {
        self.values
            .insert(key.to_ascii_lowercase(), TomlValue::Integer(value));
    }

    pub fn set_string(&mut self, key: &str, value: impl Into<String>) {
        self.values
            .insert(key.to_ascii_lowercase(), TomlValue::String(value.into()));
    }

    pub fn set_strings(&mut self, key: &str, values: &[String]) {
        self.values.insert(
            key.to_ascii_lowercase(),
            TomlValue::Array(values.iter().cloned().map(TomlValue::String).collect()),
        );
    }

    pub fn as_json(&self) -> Map<String, Value> {
        let mut result = Map::new();
        result.insert(
            "basedictdir".to_string(),
            Value::String(self.base_dict_dir.to_string_lossy().into_owned()),
        );
        for (key, value) in &self.values {
            if let Some(value) = toml_to_json(value) {
                result.insert(key.clone(), value);
            }
        }
        result
    }
}

fn resolve_config_path(value: &str, config_dir: &Path) -> io::Result<PathBuf> {
    let home = home_dir()?;
    let replaced = value
        .replace("$HOME", &home.to_string_lossy())
        .replace("$APPCONFDIR", &config_dir.to_string_lossy());
    if replaced.trim().is_empty() {
        return Ok(home.join("dicts"));
    }
    let path = PathBuf::from(replaced);
    Ok(if path.is_absolute() {
        path
    } else {
        home.join(path)
    })
}

fn toml_bool(value: &TomlValue) -> Option<bool> {
    match value {
        TomlValue::Boolean(value) => Some(*value),
        TomlValue::String(value) => match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        TomlValue::Integer(value) => Some(*value != 0),
        _ => None,
    }
}

fn toml_integer(value: &TomlValue) -> Option<i64> {
    match value {
        TomlValue::Integer(value) => Some(*value),
        TomlValue::String(value) => value.parse().ok(),
        TomlValue::Float(value) => Some(*value as i64),
        _ => None,
    }
}

fn toml_to_json(value: &TomlValue) -> Option<Value> {
    match value {
        TomlValue::String(value) => Some(Value::String(value.clone())),
        TomlValue::Integer(value) => Some(Value::Number((*value).into())),
        TomlValue::Float(value) => serde_json::Number::from_f64(*value).map(Value::Number),
        TomlValue::Boolean(value) => Some(Value::Bool(*value)),
        TomlValue::Datetime(value) => Some(Value::String(value.to_string())),
        TomlValue::Array(values) => Some(Value::Array(
            values.iter().filter_map(toml_to_json).collect(),
        )),
        TomlValue::Table(table) => {
            let mut object = Map::new();
            for (key, value) in table {
                if let Some(value) = toml_to_json(value) {
                    object.insert(key.clone(), value);
                }
            }
            Some(Value::Object(object))
        }
    }
}
