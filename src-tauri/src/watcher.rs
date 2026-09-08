use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{is_dev, platform_name};
use crate::gre_assembler::GreAssembler;
use crate::log_follower::{detect_detailed_logs, DetailedLogs, LogFollower};
use crate::log_path::player_log_path;
use crate::match_payload::Match;
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
    pub api_base: String,
    pub has_token: bool,
    pub log_path: String,
    pub log_exists: bool,
    pub detailed_logs: Option<bool>,
    pub last_match_id: Option<String>,
    pub last_error: Option<String>,
    pub last_upload_status: Option<u16>,
    pub host_reachable: Option<bool>,
    pub entries_seen: u64,
}

impl WatcherStatus {
    pub fn from_settings(settings: &Settings, log_path: PathBuf) -> Self {
        let exists = log_path.exists();
        Self {
            phase: if exists {
                WatchPhase::Starting
            } else {
                WatchPhase::LogMissing
            },
            platform: platform_name().to_string(),
            is_dev: is_dev(),
            api_base: settings.api_base(),
            has_token: settings.token().is_some(),
            log_path: log_path.display().to_string(),
            log_exists: exists,
            detailed_logs: None,
            last_match_id: None,
            last_error: None,
            last_upload_status: None,
            host_reachable: None,
            entries_seen: 0,
        }
    }
}

pub enum WatchCommand {
    ReplayFile(PathBuf),
    ReplayEmbedded,
    Refresh,
}

pub struct WatcherShared {
    pub status: Mutex<WatcherStatus>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
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
    let mut retry_queue: VecDeque<Match> = VecDeque::new();
    let mut posted_lens: HashMap<String, usize> = HashMap::new();
    let mut ticks: u64 = 0;
    let mut next_retry_at = Instant::now();

    probe_log_status(&app, &mut follower);
    ping_and_store(&app, &client).await;

    loop {
        ticks += 1;
        if ticks % 25 == 0 {
            ping_and_store(&app, &client).await;
        }

        while let Ok(command) = commands.try_recv() {
            match command {
                WatchCommand::ReplayFile(path) => {
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
                    if let Err(error) = replay_embedded(&app, &client, &mut posted_lens).await {
                        update_status(&app, |status| {
                            status.phase = WatchPhase::Error;
                            status.last_error = Some(error);
                        });
                    }
                }
                WatchCommand::Refresh => {
                    follower = open_follower(&app, true);
                    probe_log_status(&app, &mut follower);
                }
            }
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
                    if let Some(assembled) = assembler.push(&entry) {
                        enqueue_if_new(&mut retry_queue, &posted_lens, assembled);
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

        probe_log_status(&app, &mut follower);
        sync_phase(&app, assembler.in_match());

        if Instant::now() >= next_retry_at {
            if let Some(assembled) = retry_queue.pop_front() {
                let before = retry_queue.len();
                upload_match(&app, &client, assembled, &mut retry_queue, &mut posted_lens).await;
                if retry_queue.len() > before {
                    next_retry_at = Instant::now() + Duration::from_secs(10);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}

fn open_follower(app: &AppHandle, tail: bool) -> LogFollower {
    let settings = app.state::<WatcherShared>().settings.lock().unwrap().clone();
    let path = resolve_log_path(&settings);
    update_status(app, |status| {
        status.log_path = path.display().to_string();
        status.log_exists = path.exists();
        status.api_base = settings.api_base();
        status.has_token = settings.token().is_some();
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
    let sample = std::fs::read_to_string(path).unwrap_or_default();
    let detected = detect_detailed_logs(&sample);
    update_status(app, |status| {
        status.log_exists = true;
        status.detailed_logs = match detected {
            DetailedLogs::On => Some(true),
            DetailedLogs::Off => Some(false),
            DetailedLogs::Unknown => None,
        };
        if detected == DetailedLogs::Off && status.phase == WatchPhase::Watching {
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
        if detected == DetailedLogs::Unknown && status.phase == WatchPhase::Starting {
            status.phase = WatchPhase::Watching;
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
            && status.last_upload_status.is_some_and(|code| (200..300).contains(&code))
        {
            WatchPhase::Uploaded
        } else {
            WatchPhase::Watching
        };
    });
}

fn enqueue_if_new(
    queue: &mut VecDeque<Match>,
    posted_lens: &HashMap<String, usize>,
    assembled: Match,
) {
    if posted_lens.get(&assembled.client_match_id) == Some(&assembled.timeline.len()) {
        return;
    }
    if queue
        .iter()
        .any(|queued| queued.client_match_id == assembled.client_match_id)
    {
        if let Some(existing) = queue
            .iter_mut()
            .find(|queued| queued.client_match_id == assembled.client_match_id)
        {
            *existing = assembled;
        }
        return;
    }
    queue.push_back(assembled);
}

async fn upload_match(
    app: &AppHandle,
    client: &reqwest::Client,
    assembled: Match,
    queue: &mut VecDeque<Match>,
    posted_lens: &mut HashMap<String, usize>,
) {
    let (api_base, token) = {
        let settings = app.state::<WatcherShared>().settings.lock().unwrap().clone();
        (settings.api_base(), settings.token())
    };
    update_status(app, |status| {
        status.phase = WatchPhase::Uploading;
        status.last_match_id = Some(assembled.client_match_id.clone());
        status.last_error = None;
    });
    match post_match_with_retry(client, &api_base, token.as_deref(), &assembled).await {
        Ok(result) => {
            posted_lens.insert(assembled.client_match_id.clone(), assembled.timeline.len());
            update_status(app, |status| {
                status.phase = WatchPhase::Uploaded;
                status.last_upload_status = Some(result.status);
                status.last_error = None;
            });
        }
        Err(error) => {
            queue.push_back(assembled);
            update_status(app, |status| {
                status.phase = WatchPhase::Error;
                status.last_error = Some(format!("Upload failed: {error}"));
            });
        }
    }
}

async fn replay_embedded(
    app: &AppHandle,
    client: &reqwest::Client,
    posted_lens: &mut HashMap<String, usize>,
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
    let assembled = finished.ok_or_else(|| "Embedded fixture did not produce a match".to_string())?;
    let mut queue = VecDeque::new();
    upload_match(app, client, assembled, &mut queue, posted_lens).await;
    Ok(())
}

async fn ping_and_store(app: &AppHandle, client: &reqwest::Client) {
    let api_base = app.state::<WatcherShared>().settings.lock().unwrap().api_base();
    let reachable = ping_host(client, &api_base)
        .await
        .ok()
        .is_some_and(|status| (200..400).contains(&status));
    update_status(app, |status| status.host_reachable = Some(reachable));
}

fn update_status(app: &AppHandle, mutate: impl FnOnce(&mut WatcherStatus)) {
    let shared = app.state::<WatcherShared>();
    let snapshot = {
        let mut status = shared.status.lock().unwrap();
        mutate(&mut status);
        status.clone()
    };
    let _ = app.emit("watcher-status", snapshot);
}

pub fn current_status(app: &AppHandle) -> WatcherStatus {
    app.state::<WatcherShared>().status.lock().unwrap().clone()
}
