//! Persist the compare selection across launches.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::data::Catalog;
use crate::ui::theme::ThemeId;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub selected: Vec<String>,
    #[serde(default)]
    pub theme: ThemeId,
}

impl Session {
    pub fn load() -> Self {
        Self::load_from(&session_path())
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        let path = session_path();
        self.save_to(&path)?;
        Ok(path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        fs::write(path, text).map_err(|err| format!("write {}: {err}", path.display()))
    }

    pub fn into_existing(self, catalog: &Catalog) -> Vec<String> {
        self.selected
            .into_iter()
            .filter(|id| catalog.model_by_id(id).is_some())
            .collect()
    }
}

pub fn session_path() -> PathBuf {
    if let Ok(path) = env::var("COPILOT_PRICING_SESSION") {
        return PathBuf::from(path);
    }
    let pricing = crate::data::preferred_write_path();
    match pricing.file_name() {
        Some(_) => pricing.with_file_name("session.json"),
        None => Path::new("data/session.json").to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Catalog;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn isolated_path() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("copilot-pricing-session-{n}.json"))
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = isolated_path();
        let session = Session {
            selected: vec!["gpt-5.6-sol-default".into(), "claude-opus-5".into()],
            theme: crate::ui::theme::ThemeId::Dracula,
        };
        session.save_to(&path).unwrap();
        let loaded = Session::load_from(&path);
        assert_eq!(loaded.selected, session.selected);
        assert_eq!(loaded.theme, session.theme);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn drops_ids_missing_from_catalog() {
        let catalog = Catalog::bundled();
        let known = catalog.models[0].id.clone();
        let session = Session {
            selected: vec![known.clone(), "not-a-real-model".into()],
            theme: crate::ui::theme::ThemeId::GitHub,
        };
        let kept = session.into_existing(&catalog);
        assert_eq!(kept, vec![known]);
    }
}
