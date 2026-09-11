use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{is_dev, platform_name};
use crate::gre_assembler::GreAssembler;
use crate::local_stats::{CompanionStats, LocalStats};
use crate::log_follower::{detect_detailed_logs, DetailedLogs, LogFollower};
use crate::log_path::player_log_path;
use crate::match_payload::Match;
use crate::outbox::Outbox;
use crate::poster::{http_client, ping_host, post_match_with_retry};
use crate::settings::Settings;

const EMBEDDED_FIXTURE: &str = include_str!("../fixtures/match_bo1.log");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchPhase {
    Starting,
    LogMissing,
    DetailedLogsOff,
    Watching,
    MatchInProgress,
    Uploading,
    Uploaded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherStatus {
    pub phase: WatchPhase,
    pub platform: String,
    pub is_dev: bool,
    pub developer_mode: bool,
    pub show_debug_info: bool,
    pub api_base: String,
    pub has_token: bool,
    pub signed_in_email: Option<String>,
    pub signed_in_name: Option<String>,
    pub log_path: String,
    pub log_exists: bool,
    pub detailed_logs: Option<bool>,
    pub last_match_id: Option<String>,
    pub last_error: Option<String>,
    pub last_upload_status: Option<u16>,
    pub host_reachable: Option<bool>,
    pub entries_seen: u64,
    pub stats: CompanionStats,
    pub pending_uploads: usize,
    pub failed_uploads: usize,
}

impl WatcherStatus {
    pub fn from_settings(settings: &Settings, log_path: PathBuf, stats: CompanionStats) -> Self {
        let exists = log_path.exists();
        Self {
            phase: if exists {
                WatchPhase::Starting
            } else {
                WatchPhase::LogMissing
            },
            platform: platform_name().to_string(),
            is_dev: is_dev(),
            developer_mode: settings.developer_mode,
            show_debug_info: settings.show_debug_info,
            api_base: settings.api_base(),
            has_token: settings.token().is_some(),
            signed_in_email: settings.user_email(),
            signed_in_name: settings.user_name(),
            log_path: log_path.display().to_string(),
            log_exists: exists,
            detailed_logs: None,
            last_match_id: None,
            last_error: None,
            last_upload_status: None,
            host_reachable: None,
            entries_seen: 0,
            stats,
            pending_uploads: 0,
            failed_uploads: 0,
        }
    }
}

pub enum WatchCommand {
    ReplayFile(PathBuf),
    ReplayEmbedded,
    Refresh,
    RetryFailed,
    DiscardFailed,
}

pub struct WatcherShared {
    pub status: Mutex<WatcherStatus>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub stats: Mutex<LocalStats>,
    pub stats_path: PathBuf,
    pub commands: tokio::sync::mpsc::Sender<WatchCommand>,
}

pub fn resolve_log_path(settings: &Settings) -> PathBuf {
    if let Some(override_path) = settings
        .log_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(override_path);
    }
    player_log_path().unwrap_or_else(|| PathBuf::from("Player.log"))
}

pub async fn run(app: AppHandle, mut commands: tokio::sync::mpsc::Receiver<WatchCommand>) {
    let client = match http_client() {
        Ok(client) => client,
        Err(error) => {
            update_status(&app, |status| {
                status.phase = WatchPhase::Error;
                status.last_error = Some(format!("HTTP client: {error}"));
            });
            return;
        }
    };

    let mut assembler = GreAssembler::new();
    let mut follower = open_follower(&app, true);
    let mut outbox = Outbox::default();
    let mut outbox_path = None;
    let mut outbox_error = false;
    let mut upload: Option<(
        Match,
        tokio::task::JoinHandle<Result<crate::poster::PostResult, crate::poster::PostError>>,
    )> = None;
    let mut health: Option<tokio::task::JoinHandle<()>> = None;
    let mut auth_blocked = false;
    let mut retry_delay = 10u64;
    let mut posted_lens: HashMap<String, usize> = HashMap::new();
    let mut session = None;
    let mut ticks: u64 = 0;
    let mut next_retry_at = Instant::now();

    probe_log_status(&app, &mut follower);

    loop {
        let settings = app
            .state::<WatcherShared>()
            .settings
            .lock()
            .unwrap()
            .clone();
        let current_session = settings.session_key();
        if session.as_ref() != Some(&current_session) {
            if let Some((_, task)) = upload.take() {
                task.abort();
            }
            session = Some(current_session.clone());
            assembler = GreAssembler::new();
            posted_lens.clear();
            auth_blocked = false;
            retry_delay = 10;
            next_retry_at = Instant::now();
            follower = open_follower(&app, true);
            let shared = app.state::<WatcherShared>();
            outbox_path = Outbox::path(
                shared.settings_path.parent().unwrap(),
                settings.developer_mode,
                settings.user_id(),
            );
            outbox_error = false;
            outbox = match outbox_path
                .as_ref()
                .map(|path| Outbox::load(path))
                .transpose()
            {
                Ok(queue) => queue.unwrap_or_default(),
                Err(error) => {
                    outbox_error = true;
                    update_status(&app, |status| {
                        status.phase = WatchPhase::Error;
                        status.last_error = Some(format!("Cannot load pending uploads: {error}"));
                    });
                    Outbox::default()
                }
            };
            update_status(&app, |status| {
                status.last_match_id = None;
                status.last_upload_status = None;
            });
        }
        ticks += 1;
        if (ticks == 1 || ticks % 25 == 0) && health.as_ref().is_none_or(|task| task.is_finished())
        {
            let app = app.clone();
            let client = client.clone();
            health = Some(tokio::spawn(async move {
                ping_and_store(&app, &client).await;
            }));
        }

        if upload.as_ref().is_some_and(|(_, task)| task.is_finished()) {
            let (payload, task) = upload.take().unwrap();
            match task.await {
                Ok(Ok(result)) => {
                    outbox.finish(&payload, None);
                    retry_delay = 10;
                    update_status(&app, |status| {
                        status.phase = WatchPhase::Uploaded;
                        status.last_upload_status = Some(result.status);
                        status.last_error = None;
                    });
                }
                Ok(Err(error)) => {
                    auth_blocked = matches!(
                        &error,
                        crate::poster::PostError::Unauthenticated
                            | crate::poster::PostError::Http {
                                status: 401 | 403,
                                ..
                            }
                    );
                    if !auth_blocked && !crate::poster::should_retry(&error) {
                        outbox.finish(&payload, Some("The server rejected this match.".into()));
                    }
                    next_retry_at = Instant::now() + Duration::from_secs(retry_delay);
                    retry_delay = (retry_delay * 2).min(300);
                    update_status(&app, |status| {
                        status.phase = WatchPhase::Error;
                        status.last_error = Some(if auth_blocked {
                            "Sign in again or verify your email on the website, then refresh to resume uploads.".into()
                        } else {
                            "Could not sync a match. Temporary failures retry automatically; rejected matches remain saved on this device.".into()
                        });
                    });
                }
                Err(_) => {
                    next_retry_at = Instant::now() + Duration::from_secs(10);
                }
            }
            if let Some(path) = &outbox_path {
                if let Err(error) = outbox.save(path) {
                    outbox_error = true;
                    update_status(&app, |status| {
                        status.phase = WatchPhase::Error;
                        status.last_error = Some(format!("Cannot save pending uploads: {error}"));
                    });
                }
            }
        }

        while let Ok(command) = commands.try_recv() {
            match command {
                WatchCommand::ReplayFile(path) => {
                    if !app
                        .state::<WatcherShared>()
                        .settings
                        .lock()
                        .unwrap()
                        .developer_mode
                    {
                        continue;
                    }
                    assembler = GreAssembler::new();
                    follower = LogFollower::from_start(path);
                    update_status(&app, |status| {
                        status.log_path = follower.path().display().to_string();
                        status.log_exists = follower.path().exists();
                        status.phase = WatchPhase::Watching;
                        status.last_error = None;
                    });
                }
                WatchCommand::ReplayEmbedded => {
                    if !app
                        .state::<WatcherShared>()
                        .settings
                        .lock()
                        .unwrap()
                        .developer_mode
                    {
                        continue;
                    }
                    if let Err(error) = replay_embedded(&app, &client, &mut posted_lens).await {
                        update_status(&app, |status| {
                            status.phase = WatchPhase::Error;
                            status.last_error = Some(error);
                        });
                    }
                }
                WatchCommand::RetryFailed | WatchCommand::DiscardFailed => {
                    if matches!(command, WatchCommand::RetryFailed) {
                        outbox.retry_failed();
                    } else {
                        outbox.discard_failed();
                    }
                    if let Some(path) = &outbox_path {
                        if outbox.save(path).is_ok() {
                            outbox_error = false;
                            auth_blocked = false;
                            next_retry_at = Instant::now();
                        }
                    }
                }
                WatchCommand::Refresh => {
                    auth_blocked = false;
                    follower = open_follower(&app, true);
                    probe_log_status(&app, &mut follower);
                }
            }
        }

        if app
            .state::<WatcherShared>()
            .settings
            .lock()
            .unwrap()
            .session_key()
            != current_session
        {
            continue;
        }

        match follower.poll() {
            Ok(entries) => {
                if !entries.is_empty() {
                    update_status(&app, |status| {
                        status.entries_seen += entries.len() as u64;
                        status.log_exists = true;
                    });
                }
                for entry in entries {
                    if detect_detailed_logs(&entry.body) == DetailedLogs::On {
                        update_status(&app, |status| {
                            status.detailed_logs = Some(true);
                        });
                    }
                    if let Some(assembled) = assembler.push(&entry) {
                        record_processed_match(&app, &assembled);
                        if !outbox_error {
                            if let Some(path) = &outbox_path {
                                if let Err(error) =
                                    outbox.enqueue(assembled).and_then(|()| outbox.save(path))
                                {
                                    outbox_error = true;
                                    update_status(&app, |status| {
                                        status.phase = WatchPhase::Error;
                                        status.last_error =
                                            Some(format!("Could not save this match: {error}"));
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                update_status(&app, |status| {
                    status.phase = WatchPhase::LogMissing;
                    status.log_exists = false;
                    status.last_error = Some("Player.log is missing. Play Arena once, or enable Detailed Logs and restart it.".into());
                });
            }
            Err(error) => {
                update_status(&app, |status| {
                    status.phase = WatchPhase::Error;
                    status.last_error = Some(error.to_string());
                });
            }
        }

        if ticks % 25 == 0 {
            probe_log_status(&app, &follower);
        }
        sync_phase(&app, assembler.in_match());

        if !outbox_error && !auth_blocked && upload.is_none() && Instant::now() >= next_retry_at {
            if let (Some(payload), Some(token)) = (outbox.next(), settings.token()) {
                let client = client.clone();
                let api_base = settings.api_base();
                let sending = payload.clone();
                update_status(&app, |status| {
                    status.phase = WatchPhase::Uploading;
                    status.last_match_id = Some(payload.client_match_id.clone());
                });
                upload = Some((
                    payload,
                    tokio::spawn(async move {
                        crate::poster::post_match(&client, &api_base, Some(&token), &sending).await
                    }),
                ));
            }
        }
        update_status(&app, |status| {
            status.pending_uploads = outbox.pending();
            status.failed_uploads = outbox.failed();
        });

        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}

fn open_follower(app: &AppHandle, tail: bool) -> LogFollower {
    let settings = app
        .state::<WatcherShared>()
        .settings
        .lock()
        .unwrap()
        .clone();
    let path = resolve_log_path(&settings);
    update_status(app, |status| {
        status.log_path = path.display().to_string();
        status.log_exists = path.exists();
        status.api_base = settings.api_base();
        status.has_token = settings.token().is_some();
        status.signed_in_email = settings.user_email();
        status.signed_in_name = settings.user_name();
    });
    if tail {
        LogFollower::tail(path)
    } else {
        LogFollower::from_start(path)
    }
}

fn probe_log_status(app: &AppHandle, follower: &LogFollower) {
    let path = follower.path();
    if !path.exists() {
        update_status(app, |status| {
            status.log_exists = false;
            status.detailed_logs = None;
            if status.phase != WatchPhase::Uploading {
                status.phase = WatchPhase::LogMissing;
            }
        });
        return;
    }
    let sample = match (|| -> std::io::Result<String> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        file.seek(SeekFrom::Start(len.saturating_sub(65_536)))?;
        let mut bytes = Vec::new();
        file.take(65_536).read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    })() {
        Ok(sample) => sample,
        Err(error) => {
            update_status(app, |status| {
                status.phase = WatchPhase::Error;
                status.last_error = Some(format!("Cannot read game file: {error}"));
            });
            return;
        }
    };
    let detected = detect_detailed_logs(&sample);
    update_status(app, |status| {
        status.log_exists = true;
        status.detailed_logs = match detected {
            DetailedLogs::On => Some(true),
            DetailedLogs::Off => Some(false),
            DetailedLogs::Unknown => status.detailed_logs,
        };
        if detected == DetailedLogs::Off
            && matches!(
                status.phase,
                WatchPhase::Watching | WatchPhase::Starting | WatchPhase::LogMissing
            )
        {
            status.phase = WatchPhase::DetailedLogsOff;
            status.last_error = Some(
                "Detailed Logs are off. In Arena: Options → Account → Detailed Logs (Plugin Support), then restart Arena.".into(),
            );
        }
        if detected == DetailedLogs::On
            && matches!(
                status.phase,
                WatchPhase::DetailedLogsOff | WatchPhase::LogMissing | WatchPhase::Starting
            )
        {
            status.phase = WatchPhase::Watching;
            status.last_error = None;
        }
        if detected == DetailedLogs::Unknown
            && matches!(
                status.phase,
                WatchPhase::Starting | WatchPhase::LogMissing | WatchPhase::DetailedLogsOff
            )
        {
            status.phase = WatchPhase::Watching;
            status.last_error = None;
        }
    });
}

fn sync_phase(app: &AppHandle, in_match: bool) {
    update_status(app, |status| {
        if matches!(
            status.phase,
            WatchPhase::Error
                | WatchPhase::Uploading
                | WatchPhase::LogMissing
                | WatchPhase::DetailedLogsOff
        ) {
            return;
        }
        status.phase = if in_match {
            WatchPhase::MatchInProgress
        } else if status.last_match_id.is_some()
            && status
                .last_upload_status
                .is_some_and(|code| (200..300).contains(&code))
        {
            WatchPhase::Uploaded
        } else {
            WatchPhase::Watching
        };
    });
}

async fn replay_embedded(
    app: &AppHandle,
    client: &reqwest::Client,
    _posted_lens: &mut HashMap<String, usize>,
) -> Result<(), String> {
    let dir = std::env::temp_dir().join("arenacoach-embedded-fixture.log");
    std::fs::write(&dir, EMBEDDED_FIXTURE).map_err(|error| error.to_string())?;
    let mut follower = LogFollower::from_start(dir);
    let mut assembler = GreAssembler::new();
    let entries = follower.poll().map_err(|error| error.to_string())?;
    let mut finished = None;
    for entry in entries {
        if let Some(assembled) = assembler.push(&entry) {
            finished = Some(assembled);
        }
    }
    let mut assembled =
        finished.ok_or_else(|| "Embedded fixture did not produce a match".to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    assembled.client_match_id = format!("match-fixture-{stamp}");
    record_processed_match(app, &assembled);
    let session = app
        .state::<WatcherShared>()
        .settings
        .lock()
        .unwrap()
        .clone();
    let result = post_match_with_retry(
        client,
        &session.api_base(),
        session.token().as_deref(),
        &assembled,
    )
    .await
    .map_err(|error| error.to_string())?;
    if app
        .state::<WatcherShared>()
        .settings
        .lock()
        .unwrap()
        .session_key()
        == session.session_key()
    {
        update_status(app, |status| {
            status.phase = WatchPhase::Uploaded;
            status.last_match_id = Some(assembled.client_match_id);
            status.last_upload_status = Some(result.status);
        });
    }
    Ok(())
}

async fn ping_and_store(app: &AppHandle, client: &reqwest::Client) {
    let api_base = app
        .state::<WatcherShared>()
        .settings
        .lock()
        .unwrap()
        .api_base();
    let reachable = ping_host(client, &api_base)
        .await
        .ok()
        .is_some_and(|status| (200..400).contains(&status));
    if app
        .state::<WatcherShared>()
        .settings
        .lock()
        .unwrap()
        .api_base()
        == api_base
    {
        update_status(app, |status| status.host_reachable = Some(reachable));
    }
}

fn record_processed_match(app: &AppHandle, assembled: &Match) {
    let shared = app.state::<WatcherShared>();
    let snapshot = {
        let mut stats = shared.stats.lock().unwrap();
        if !stats.record(assembled) {
            return;
        }
        let _ = stats.save(&shared.stats_path);
        stats.summary()
    };
    update_status(app, |status| {
        status.stats = snapshot;
    });
}

pub fn reset_local_stats(app: &AppHandle) -> std::io::Result<()> {
    let shared = app.state::<WatcherShared>();
    let snapshot = {
        let mut stats = shared.stats.lock().unwrap();
        stats.reset();
        stats.save(&shared.stats_path)?;
        stats.summary()
    };
    update_status(app, |status| {
        status.stats = snapshot;
    });
    Ok(())
}

fn update_status(app: &AppHandle, mutate: impl FnOnce(&mut WatcherStatus)) {
    let shared = app.state::<WatcherShared>();
    let snapshot = {
        let mut status = shared.status.lock().unwrap();
        let before = serde_json::to_value(&*status).ok();
        mutate(&mut status);
        if before == serde_json::to_value(&*status).ok() {
            return;
        }
        status.clone()
    };
    let _ = app.emit("watcher-status", snapshot);
}

pub fn current_status(app: &AppHandle) -> WatcherStatus {
    app.state::<WatcherShared>().status.lock().unwrap().clone()
}
