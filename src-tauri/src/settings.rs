use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::default_api_base;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user_email: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub log_path: Option<String>,
}

impl Settings {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self).unwrap_or_else(|_| b"{}".to_vec()))
    }

    pub fn api_base(&self) -> String {
        self.api_base
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_api_base)
    }

    pub fn token(&self) -> Option<String> {
        self.token
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    pub fn user_email(&self) -> Option<String> {
        self.user_email
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}
