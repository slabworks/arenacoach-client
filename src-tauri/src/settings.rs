use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::api_base_for_mode;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(skip)]
    pub generation: u64,
    #[serde(default)]
    pub developer_mode: bool,
    #[serde(default)]
    pub show_debug_info: bool,
    #[serde(default, skip_serializing)]
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
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(error),
        };
        let mut settings = Self::from_json(&text).map_err(std::io::Error::other)?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(std::io::Error::other)?;
        if settings.token().is_some() {
            settings.save(path)?;
        } else if value.get("session_saved").and_then(|v| v.as_bool()) == Some(true) {
            let entry = credential_entry()?;
            match entry.get_password() {
                Ok(secret) => {
                    let session: SavedSession = serde_json::from_str(&secret).map_err(|_| std::io::Error::other("Invalid saved session"))?;
                    if session.api_base == settings.api_base() {
                        settings.token = Some(session.token);
                        settings.user_id = Some(session.user_id);
                        settings.user_name = session.user_name;
                        settings.user_email = session.user_email;
                    }
                }
                Err(keyring::Error::NoEntry) => settings.clear_session(),
                Err(_) => return Err(std::io::Error::other("Cannot access your system credential store. Unlock it and restart Arena Coach.")),
            }
        }
        Ok(settings)
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        let entry = credential_entry()?;
        if let Some(token) = self.token() {
            let session = SavedSession {
                api_base: self.api_base(),
                token,
                user_id: self.user_id.unwrap_or(0),
                user_email: self.user_email.clone(),
                user_name: self.user_name.clone(),
            };
            entry
                .set_password(&serde_json::to_string(&session).map_err(std::io::Error::other)?)
                .map_err(|_| {
                    std::io::Error::other("Cannot save your session in the system credential store")
                })?;
        }
        let mut value = serde_json::to_value(self).map_err(std::io::Error::other)?;
        value["session_saved"] = serde_json::Value::Bool(self.token().is_some());
        crate::outbox::atomic_write(
            path,
            &serde_json::to_vec_pretty(&value).map_err(std::io::Error::other)?,
        )?;
        if self.token().is_none() {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(_) => {
                    return Err(std::io::Error::other(
                        "Signed out locally, but the saved credential could not be removed",
                    ))
                }
            }
        }
        Ok(())
    }

    pub fn session_key(&self) -> (String, Option<i64>, u64, Option<String>) {
        (
            self.api_base(),
            self.user_id(),
            self.generation,
            self.token(),
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

    pub fn clear_session(&mut self) {
        self.generation = self.generation.wrapping_add(1);
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
    fn logout_invalidates_captured_session_keys() {
        let mut settings = Settings {
            token: Some("token".into()),
            user_id: Some(1),
            ..Settings::default()
        };
        let key = settings.session_key();
        settings.clear_session();
        assert_ne!(settings.session_key(), key);
    }

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
        assert!(restored.token().is_none());
        assert!(!serde_json::to_string(&settings).unwrap().contains("secret"));
    }
}

#[derive(Serialize, Deserialize)]
struct SavedSession {
    api_base: String,
    token: String,
    user_id: i64,
    user_email: Option<String>,
    user_name: Option<String>,
}

fn credential_entry() -> std::io::Result<keyring::Entry> {
    if !cfg!(any(target_os = "macos", target_os = "windows")) {
        return Err(std::io::Error::other(
            "Secure sign-in is supported on macOS and Windows.",
        ));
    }
    keyring::Entry::new("com.chris.arenacoach-local", "session")
        .map_err(|_| std::io::Error::other("Cannot open the system credential store"))
}
