mod config;
mod gre_assembler;
mod log_follower;
mod log_path;
mod match_payload;
mod poster;
mod settings;
mod watcher;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Deserialize;
use tauri::Manager;

use config::platform_name;
use poster::{http_client, login_device, logout_device, PostError};
use settings::Settings;
use watcher::{current_status, resolve_log_path, WatchCommand, WatcherShared, WatcherStatus};

#[tauri::command]
fn get_status(app: tauri::AppHandle) -> WatcherStatus {
    current_status(&app)
}

#[derive(Debug, Deserialize)]
struct SettingsPatch {
    developer_mode: Option<bool>,
    show_debug_info: Option<bool>,
    token: Option<String>,
    log_path: Option<String>,
}

#[tauri::command]
fn update_settings(app: tauri::AppHandle, patch: SettingsPatch) -> Result<WatcherStatus, String> {
    let shared = app.state::<WatcherShared>();
    let mut stored_settings = shared.settings.lock().map_err(|error| error.to_string())?;
    let mut next_settings = stored_settings.clone();
    let settings = &mut next_settings;
    let mode_changed = patch
        .developer_mode
        .map(|enabled| settings.set_developer_mode(enabled))
        .unwrap_or(false);
    if let Some(enabled) = patch.show_debug_info {
        settings.show_debug_info = enabled;
    }
    if let Some(token) = patch.token {
        settings.token = empty_to_none(token);
    }
    if let Some(ref log_path) = patch.log_path {
        settings.log_path = empty_to_none(log_path.clone());
    }
    settings
        .save(&shared.settings_path)
        .map_err(|error| error.to_string())?;
    let snapshot = settings.clone();
    *stored_settings = snapshot.clone();
    drop(stored_settings);
    if let Ok(mut status) = shared.status.lock() {
        status.api_base = snapshot.api_base();
        status.developer_mode = snapshot.developer_mode;
        status.show_debug_info = snapshot.show_debug_info;
        if mode_changed {
            status.host_reachable = None;
            status.last_error = None;
            status.last_match_id = None;
            status.last_upload_status = None;
        }
        status.has_token = snapshot.token().is_some();
        status.signed_in_email = snapshot.user_email();
        let path = resolve_log_path(&snapshot);
        status.log_path = path.display().to_string();
        status.log_exists = path.exists();
    }
    if mode_changed || patch.log_path.is_some() {
        let _ = shared.commands.try_send(WatchCommand::Refresh);
    }
    Ok(current_status(&app))
}

#[derive(Debug, Deserialize)]
struct SignInPayload {
    email: String,
    password: String,
    code: Option<String>,
    recovery_code: Option<String>,
}

#[tauri::command]
async fn sign_in(app: tauri::AppHandle, payload: SignInPayload) -> Result<WatcherStatus, String> {
    let shared = app.state::<WatcherShared>();
    let api_base = {
        let settings = shared.settings.lock().map_err(|error| error.to_string())?;
        settings.api_base()
    };
    let client = http_client().map_err(|error| error.to_string())?;
    let device_name = format!("Arena Coach ({})", platform_name());
    let session = login_device(
        &client,
        &api_base,
        payload.email.trim(),
        &payload.password,
        &device_name,
        payload.code.as_deref(),
        payload.recovery_code.as_deref(),
    )
    .await
    .map_err(|error| match error {
        PostError::TwoFactor => "two_factor".to_string(),
        other => other.to_string(),
    })?;

    let mut settings = shared.settings.lock().map_err(|error| error.to_string())?;
    if settings.api_base() != api_base {
        return Err("Environment changed. Please sign in again.".into());
    }
    settings.token = Some(session.token);
    settings.user_email = Some(session.email);
    settings.user_name = Some(session.name);
    settings
        .save(&shared.settings_path)
        .map_err(|error| error.to_string())?;
    let snapshot = settings.clone();
    drop(settings);
    if let Ok(mut status) = shared.status.lock() {
        status.has_token = true;
        status.signed_in_email = snapshot.user_email();
        status.last_error = None;
    }
    let _ = shared.commands.try_send(WatchCommand::Refresh);
    Ok(current_status(&app))
}

#[tauri::command]
async fn sign_out(app: tauri::AppHandle) -> Result<WatcherStatus, String> {
    let shared = app.state::<WatcherShared>();
    let (api_base, token) = {
        let settings = shared.settings.lock().map_err(|error| error.to_string())?;
        (settings.api_base(), settings.token())
    };
    if let Some(token) = token {
        if let Ok(client) = http_client() {
            let _ = logout_device(&client, &api_base, &token).await;
        }
    }
    let mut settings = shared.settings.lock().map_err(|error| error.to_string())?;
    settings.token = None;
    settings.user_email = None;
    settings.user_name = None;
    settings
        .save(&shared.settings_path)
        .map_err(|error| error.to_string())?;
    drop(settings);
    if let Ok(mut status) = shared.status.lock() {
        status.has_token = false;
        status.signed_in_email = None;
    }
    let _ = shared.commands.try_send(WatchCommand::Refresh);
    Ok(current_status(&app))
}

#[tauri::command]
fn replay_log(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let shared = app.state::<WatcherShared>();
    if !shared
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .developer_mode
    {
        return Err("Enable developer mode to replay matches.".into());
    }
    shared
        .commands
        .try_send(WatchCommand::ReplayFile(PathBuf::from(path)))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn replay_fixture(app: tauri::AppHandle) -> Result<(), String> {
    let shared = app.state::<WatcherShared>();
    if !shared
        .settings
        .lock()
        .map_err(|error| error.to_string())?
        .developer_mode
    {
        return Err("Enable developer mode to replay matches.".into());
    }
    shared
        .commands
        .try_send(WatchCommand::ReplayEmbedded)
        .map_err(|error| error.to_string())
}

fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings_path = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| std::env::temp_dir())
                .join("settings.json");
            let settings = Settings::load(&settings_path);
            let log_path = resolve_log_path(&settings);
            let status = WatcherStatus::from_settings(&settings, log_path);
            let (tx, rx) = tokio::sync::mpsc::channel(8);
            app.manage(WatcherShared {
                status: Mutex::new(status),
                settings: Mutex::new(settings),
                settings_path,
                commands: tx,
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                watcher::run(handle, rx).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            update_settings,
            sign_in,
            sign_out,
            replay_log,
            replay_fixture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
