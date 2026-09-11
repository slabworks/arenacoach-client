import { type CompanionStats } from "./local-stats";

export type { CompanionStats } from "./local-stats";

export type WatchPhase =
  | "starting"
  | "log_missing"
  | "detailed_logs_off"
  | "watching"
  | "match_in_progress"
  | "uploading"
  | "uploaded"
  | "error";

export type WatcherStatus = {
  phase: WatchPhase;
  pending_uploads?: number;
  failed_uploads?: number;
  platform: string;
  is_dev: boolean;
  developer_mode: boolean;
  show_debug_info: boolean;
  api_base: string;
  has_token: boolean;
  signed_in_email: string | null;
  signed_in_name: string | null;
  log_path: string;
  log_exists: boolean;
  detailed_logs: boolean | null;
  last_match_id: string | null;
  last_error: string | null;
  last_upload_status: number | null;
  host_reachable: boolean | null;
  entries_seen: number;
  stats: CompanionStats;
};

export function gameStatus(status: WatcherStatus | null) {
  if (!status)
    return {
      title: "Finding your games",
      detail: "Connecting to your Arena companion…",
      tone: "waiting",
    };
  if (!status.log_exists)
    return {
      title: "Waiting for Arena",
      detail:
        "Launch MTG Arena and play a game. We’ll find your game file automatically.",
      tone: "waiting",
    };
  if (status.detailed_logs === false)
    return {
      title: "One small setup step",
      detail:
        "In Arena, open Options → Account and enable Detailed Logs (Plugin Support). Then restart Arena.",
      tone: "waiting",
    };
  if (status.phase === "error")
    return {
      title: "Your companion needs attention",
      detail:
        status.last_error ?? "We couldn’t read or sync your latest game. Check that Arena is running and your connection is available.",
      tone: "waiting",
    };
  if (status.phase === "starting")
    return {
      title: "Found your game file",
      detail: "Getting everything ready to follow your next match.",
      tone: "waiting",
    };
  if (status.phase === "match_in_progress")
    return {
      title: "Following your match",
      detail:
        "Stay focused on your next move. We’re reading the game as you play.",
      tone: "live",
    };
  if (status.phase === "uploading")
    return {
      title: "Syncing your match",
      detail: "Sending your latest game to Arena Coach.",
      tone: "live",
    };
  if (status.phase === "uploaded")
    return {
      title: "Match synced",
      detail: "Your notes will land here as soon as the coach finishes.",
      tone: "live",
    };
  return {
    title: "Reading your games",
    detail:
      "Your Arena game file is connected. Play as usual — we’ll follow along.",
    tone: "live",
  };
}
