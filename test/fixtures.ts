import { EMPTY_STATS } from "../src/local-stats";
import type { MatchReport, TimelineEvent } from "../src/matches";
import type { WatcherStatus } from "../src/game-status";

export function watcherStatus(
  overrides: Partial<WatcherStatus> = {},
): WatcherStatus {
  return {
    phase: "watching",
    platform: "macos",
    is_dev: true,
    developer_mode: false,
    show_debug_info: false,
    api_base: "https://arenacoach.com",
    has_token: true,
    signed_in_email: "you@example.com",
    signed_in_name: "You",
    log_path: "/tmp/Player.log",
    log_exists: true,
    detailed_logs: true,
    last_match_id: null,
    last_error: null,
    last_upload_status: null,
    host_reachable: true,
    entries_seen: 0,
    stats: EMPTY_STATS,
    ...overrides,
  };
}

export function matchReport(
  overrides: Partial<MatchReport> = {},
): MatchReport {
  return {
    id: 1,
    client_match_id: "match-1",
    event_id: "PremierDraft",
    format: "Draft",
    result: "win",
    coaching_status: "ready",
    analysis: null,
    tips: null,
    coaching_error: null,
    ...overrides,
  };
}

export function timelineEvent(
  overrides: Partial<TimelineEvent> = {},
): TimelineEvent {
  return {
    t_ms: 0,
    turn: 1,
    phase: "Phase_Main",
    actor: "me",
    kind: "cast",
    card_ids: [],
    note: "",
    ...overrides,
  };
}
