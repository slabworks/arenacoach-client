import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type WatchPhase =
  | "starting"
  | "log_missing"
  | "detailed_logs_off"
  | "watching"
  | "match_in_progress"
  | "uploading"
  | "uploaded"
  | "error";

type WatcherStatus = {
  phase: WatchPhase;
  platform: string;
  is_dev: boolean;
  api_base: string;
  has_token: boolean;
  log_path: string;
  log_exists: boolean;
  detailed_logs: boolean | null;
  last_match_id: string | null;
  last_error: string | null;
  last_upload_status: number | null;
  host_reachable: boolean | null;
  entries_seen: number;
};

const PHASE_LABEL: Record<WatchPhase, string> = {
  starting: "Starting",
  log_missing: "Log missing",
  detailed_logs_off: "Detailed Logs off",
  watching: "Watching",
  match_in_progress: "Match in progress",
  uploading: "Uploading",
  uploaded: "Uploaded",
  error: "Error",
};

function App() {
  const [status, setStatus] = useState<WatcherStatus | null>(null);
  const [apiBase, setApiBase] = useState("");
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    invoke<WatcherStatus>("get_status")
      .then((next) => {
        setStatus(next);
        setApiBase(next.api_base);
      })
      .catch(() => undefined);
    listen<WatcherStatus>("watcher-status", (event) => {
      setStatus(event.payload);
      setApiBase((current) => current || event.payload.api_base);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function saveSettings() {
    setBusy(true);
    try {
      const next = await invoke<WatcherStatus>("update_settings", {
        patch: { api_base: apiBase, token },
      });
      setStatus(next);
      setToken("");
    } finally {
      setBusy(false);
    }
  }

  async function replayFixture() {
    setBusy(true);
    try {
      await invoke("replay_fixture");
    } finally {
      setBusy(false);
    }
  }

  if (!status) {
    return (
      <main className="shell">
        <p>Starting watcher…</p>
      </main>
    );
  }

  return (
    <main className="shell">
      <header>
        <p className="eyebrow">Unofficial fan content · not affiliated with Wizards of the Coast</p>
        <h1>Arena Coach</h1>
        <p className={`phase phase-${status.phase}`}>{PHASE_LABEL[status.phase]}</p>
      </header>

      <section className="card">
        <Row label="Platform" value={status.platform} />
        <Row label="Build" value={status.is_dev ? "dev → Herd" : "release"} />
        <Row
          label="API host"
          value={
            status.host_reachable == null
              ? status.api_base
              : `${status.api_base} (${status.host_reachable ? "up" : "down"})`
          }
        />
        <Row label="Log" value={status.log_exists ? status.log_path : "not found"} />
        <Row
          label="Detailed Logs"
          value={
            status.detailed_logs == null
              ? "unknown"
              : status.detailed_logs
                ? "on"
                : "off — Options → Account → Detailed Logs, then restart Arena"
          }
        />
        <Row label="Last match" value={status.last_match_id ?? "—"} />
        <Row
          label="Last upload"
          value={status.last_upload_status ? String(status.last_upload_status) : "—"}
        />
        <Row label="Entries seen" value={String(status.entries_seen)} />
        <Row label="Device token" value={status.has_token ? "set" : "not set"} />
      </section>

      {status.last_error ? <p className="error">{status.last_error}</p> : null}

      <form
        className="card form"
        onSubmit={(event) => {
          event.preventDefault();
          saveSettings();
        }}
      >
        <label>
          API base URL
          <input
            value={apiBase}
            onChange={(event) => setApiBase(event.currentTarget.value)}
            placeholder="https://arenacoach-web.test"
          />
        </label>
        <label>
          Device token
          <input
            type="password"
            value={token}
            onChange={(event) => setToken(event.currentTarget.value)}
            placeholder={status.has_token ? "••••••••" : "optional for now"}
          />
        </label>
        <button type="submit" disabled={busy}>
          Save connection
        </button>
      </form>

      {status.is_dev ? (
        <button className="secondary" disabled={busy} onClick={replayFixture}>
          Replay fixture match
        </button>
      ) : null}
    </main>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export default App;
