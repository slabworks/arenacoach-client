use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::api_base_for_mode;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub developer_mode: bool,
    #[serde(default)]
    pub show_debug_info: bool,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user_id: Option<i64>,
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
            .and_then(|text| Self::from_json(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(self).unwrap_or_else(|_| b"{}".to_vec()),
        )
    }

    fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(text)?;
        let mut settings: Self = serde_json::from_value(value.clone())?;
        // Legacy sessions may belong to an arbitrary host or a debug-build default.
        if value.get("developer_mode").is_none() {
            settings.clear_session();
        }
        Ok(settings)
    }

    pub fn set_developer_mode(&mut self, enabled: bool) -> bool {
        if self.developer_mode == enabled {
            return false;
        }
        self.developer_mode = enabled;
        self.clear_session();
        true
    }

    fn clear_session(&mut self) {
        self.token = None;
        self.user_id = None;
        self.user_email = None;
        self.user_name = None;
    }

    pub fn api_base(&self) -> String {
        api_base_for_mode(self.developer_mode)
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

    pub fn user_id(&self) -> Option<i64> {
        self.user_id.filter(|id| *id > 0)
    }

    pub fn user_name(&self) -> Option<String> {
        self.user_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_environment_clears_session_but_preserves_preferences() {
        let mut settings = Settings {
            token: Some("secret".into()),
            user_email: Some("a@b.com".into()),
            show_debug_info: true,
            ..Settings::default()
        };
        assert!(!settings.set_developer_mode(false));
        assert!(settings.token().is_some());
        assert!(settings.set_developer_mode(true));
        assert!(settings.token().is_none());
        assert!(settings.user_email().is_none());
        assert!(settings.show_debug_info);
        assert_eq!(settings.api_base(), "https://arenacoach-web.test");
        settings.set_developer_mode(false);
        assert_eq!(settings.api_base(), "https://arenacoach.com");
    }

    #[test]
    fn legacy_host_cannot_override_mode_or_leak_session() {
        let settings =
            Settings::from_json(r#"{"api_base":"https://old.test","token":"secret"}"#).unwrap();
        assert_eq!(settings.api_base(), "https://arenacoach.com");
        assert!(settings.token().is_none());
        assert!(!settings.show_debug_info);
    }

    #[test]
    fn preferences_and_session_round_trip() {
        let settings = Settings {
            developer_mode: true,
            show_debug_info: true,
            token: Some("secret".into()),
            ..Settings::default()
        };
        let restored = Settings::from_json(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(restored.developer_mode && restored.show_debug_info);
        assert_eq!(restored.token(), settings.token());
    }
}
