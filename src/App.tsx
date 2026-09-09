import { useEffect, useRef, useState } from "react";
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
  developer_mode: boolean;
  show_debug_info: boolean;
  api_base: string;
  has_token: boolean;
  signed_in_email: string | null;
  log_path: string;
  log_exists: boolean;
  detailed_logs: boolean | null;
  last_match_id: string | null;
  last_error: string | null;
  last_upload_status: number | null;
  host_reachable: boolean | null;
  entries_seen: number;
};

function gameStatus(status: WatcherStatus | null) {
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
        "We couldn’t read or sync your latest game. Check that Arena is running and your connection is available.",
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
  return {
    title: "Reading your games",
    detail:
      "Your Arena game file is connected. Play as usual — we’ll follow along.",
    tone: "live",
  };
}

function App() {
  const [status, setStatus] = useState<WatcherStatus | null>(null);
  const [connectionError, setConnectionError] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [needsTwoFactor, setNeedsTwoFactor] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const settingsDialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    async function connect() {
      try {
        const stop = await listen<WatcherStatus>("watcher-status", (event) => {
          if (!disposed) setStatus(event.payload);
        });
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        const next = await invoke<WatcherStatus>("get_status");
        if (!disposed) setStatus(next);
      } catch {
        if (!disposed) setConnectionError(true);
      }
    }
    void connect();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (settingsOpen) settingsDialog.current?.showModal();
    else settingsDialog.current?.close();
  }, [settingsOpen]);

  async function updateSetting(patch: {
    developer_mode?: boolean;
    show_debug_info?: boolean;
  }) {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await invoke<WatcherStatus>("update_settings", { patch }));
      if (patch.developer_mode !== undefined) {
        setPassword("");
        setCode("");
        setNeedsTwoFactor(false);
      }
    } catch {
      setActionError("Couldn’t save your settings. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function signIn() {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(
        await invoke<WatcherStatus>("sign_in", {
          payload: { email, password, code: needsTwoFactor ? code : null },
        }),
      );
      setPassword("");
      setCode("");
      setNeedsTwoFactor(false);
    } catch (error) {
      if (String(error) === "two_factor") {
        setNeedsTwoFactor(true);
        setActionError("Enter the code from your authenticator to continue.");
      } else {
        setActionError(
          "Couldn’t sign in. Check your details and connection, then try again.",
        );
      }
    } finally {
      setBusy(false);
    }
  }

  async function signOut() {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await invoke<WatcherStatus>("sign_out"));
    } catch {
      setActionError("Couldn’t sign out. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function replayFixture() {
    setBusy(true);
    setActionError(null);
    try {
      await invoke("replay_fixture");
    } catch {
      setActionError("Couldn’t replay the fixture. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  const game = gameStatus(status);
  const synced =
    status?.last_upload_status != null &&
    status.last_upload_status >= 200 &&
    status.last_upload_status < 300;

  return (
    <main className="shell">
      <header className="app-header">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            A<span>✦</span>
          </div>
          <div>
            <span className="brand-name">
              arena<span>coach</span>
            </span>
            <span className="brand-caption">YOUR ARENA COMPANION</span>
          </div>
        </div>
        <button
          className="icon-button"
          aria-label="Open settings"
          title="Settings"
          onClick={() => setSettingsOpen(true)}
          disabled={!status}
        >
          <Icon name="settings" />
        </button>
      </header>

      <div className="main-content">
        <div className="section-heading">
          <span className="eyebrow">LET’S MAKE EVERY GAME COUNT</span>
          {status?.developer_mode ? (
            <span className="dev-badge">Developer mode</span>
          ) : null}
        </div>
        <section
          className={`game-card ${game.tone}`}
          aria-labelledby="game-title"
        >
          <div className="game-topline">
            <span className="status-pill">
              <span className="status-dot" />
              {connectionError
                ? "Disconnected"
                : game.tone === "live"
                  ? "Companion active"
                  : "Getting connected"}
            </span>
            <span className="game-label">MTG ARENA</span>
          </div>
          <div className="signal-art" aria-hidden="true">
            <div className="signal-ring ring-one" />
            <div className="signal-ring ring-two" />
            <div className="card-shape card-back" />
            <div className="card-shape card-front">
              <span>✦</span>
            </div>
            <i className="spark spark-one" />
            <i className="spark spark-two" />
          </div>
          <div className="game-copy" role="status">
            <h1 id="game-title">
              {connectionError ? "Open the desktop app" : game.title}
            </h1>
            <p>
              {connectionError
                ? "The game reader is available inside the Arena Coach desktop app. Reopen it to reconnect."
                : game.detail}
            </p>
          </div>
          <div className="file-status">
            <Icon name="file" />
            <span>
              {status?.log_exists
                ? "Arena game file found"
                : "Looking for your Arena game file"}
            </span>
            {status?.log_exists ? (
              <span className="file-check">✓</span>
            ) : (
              <span className="ellipsis">···</span>
            )}
          </div>
        </section>

        <section className="account-card" aria-labelledby="account-title">
          <div className="account-heading">
            <div className="small-icon">
              <Icon name="account" />
            </div>
            <div>
              <h2 id="account-title">
                {status?.has_token
                  ? "You’re connected"
                  : "Connect your account"}
              </h2>
              <p>
                {status?.has_token
                  ? (status.signed_in_email ?? "Signed in to Arena Coach")
                  : "Bring your games and your coaching together."}
              </p>
            </div>
            {status?.has_token ? (
              <span className="connected-dot" aria-label="Signed in" />
            ) : null}
          </div>
          {status?.has_token ? (
            <div className="signed-in-details">
              <span>
                {synced
                  ? "Your latest match is synced."
                  : "Your next completed match will sync automatically."}
              </span>
              <button className="text-button" disabled={busy} onClick={signOut}>
                Sign out
              </button>
            </div>
          ) : (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void signIn();
              }}
            >
              <label htmlFor="email">Email address</label>
              <input
                id="email"
                type="email"
                placeholder="you@example.com"
                value={email}
                onChange={(e) => setEmail(e.currentTarget.value)}
                autoComplete="username"
                required
                disabled={busy}
              />
              <label htmlFor="password">Password</label>
              <input
                id="password"
                type="password"
                placeholder="Your password"
                value={password}
                onChange={(e) => setPassword(e.currentTarget.value)}
                autoComplete="current-password"
                required
                disabled={busy}
              />
              {needsTwoFactor ? (
                <>
                  <label htmlFor="code">Authenticator code</label>
                  <input
                    id="code"
                    value={code}
                    onChange={(e) => setCode(e.currentTarget.value)}
                    autoComplete="one-time-code"
                    inputMode="numeric"
                    required
                    autoFocus
                  />
                </>
              ) : null}
              <button
                className="primary-button"
                type="submit"
                disabled={busy || !status}
              >
                {busy
                  ? "Connecting…"
                  : needsTwoFactor
                    ? "Verify & connect"
                    : "Sign in to Arena Coach"}
                <span aria-hidden="true">↗</span>
              </button>
            </form>
          )}
          {actionError && !settingsOpen ? (
            <p className="error" role="alert">
              {actionError}
            </p>
          ) : null}
        </section>

        <div className="companion-note">
          <Icon name="activity" />
          <p>
            You play. We take notes.
            <span>Keep your companion open while you’re in Arena.</span>
          </p>
        </div>

        {status?.show_debug_info ? (
          <section className="debug-card" aria-labelledby="debug-title">
            <h2 id="debug-title">Debug information</h2>
            <dl>
              <Row
                label="Platform / build"
                value={`${status.platform} / ${status.is_dev ? "debug" : "release"}`}
              />
              <Row label="API endpoint" value={status.api_base} />
              <Row
                label="Host reachable"
                value={
                  status.host_reachable == null
                    ? "Checking"
                    : status.host_reachable
                      ? "Yes"
                      : "No"
                }
              />
              <Row label="Watcher phase" value={status.phase} />
              <Row label="Game file" value={status.log_path} />
              <Row
                label="Detailed logs"
                value={
                  status.detailed_logs == null
                    ? "Not yet detected"
                    : status.detailed_logs
                      ? "On"
                      : "Off"
                }
              />
              <Row label="Entries read" value={String(status.entries_seen)} />
              <Row label="Last match" value={status.last_match_id ?? "None"} />
              <Row
                label="Upload status"
                value={String(status.last_upload_status ?? "None")}
              />
              <Row label="Last error" value={status.last_error ?? "None"} />
            </dl>
          </section>
        ) : null}
      </div>

      <footer>
        <span className="footer-brand">ARENA COACH</span>
        <p>Unofficial fan content. Not affiliated with Wizards of the Coast.</p>
      </footer>

      <dialog
        ref={settingsDialog}
        onCancel={() => setSettingsOpen(false)}
        onClose={() => setSettingsOpen(false)}
        onClick={(e) => {
          if (e.target === e.currentTarget) setSettingsOpen(false);
        }}
        aria-labelledby="settings-title"
      >
        <div className="settings-header">
          <div>
            <span className="eyebrow">MAKE IT YOURS</span>
            <h2 id="settings-title">Settings</h2>
          </div>
          <button
            className="icon-button close-button"
            aria-label="Close settings"
            onClick={() => setSettingsOpen(false)}
          >
            ×
          </button>
        </div>
        <label className="setting">
          <span>
            <strong>Developer mode</strong>
            <small>
              Connect to the local development server. Changing this signs you
              out.
            </small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={status?.developer_mode ?? false}
            disabled={busy || !status}
            onChange={(e) =>
              void updateSetting({ developer_mode: e.currentTarget.checked })
            }
          />
        </label>
        <label className="setting">
          <span>
            <strong>Show debug info</strong>
            <small>
              Show file paths, connection details, and game-reader diagnostics.
            </small>
          </span>
          <input
            type="checkbox"
            role="switch"
            checked={status?.show_debug_info ?? false}
            disabled={busy || !status}
            onChange={(e) =>
              void updateSetting({ show_debug_info: e.currentTarget.checked })
            }
          />
        </label>
        {status?.developer_mode ? (
          <div className="developer-tools">
            <span className="eyebrow">DEVELOPMENT SERVER</span>
            <code>{status.api_base}</code>
            <button
              className="secondary-button"
              disabled={busy}
              onClick={replayFixture}
            >
              Replay fixture match <span aria-hidden="true">↻</span>
            </button>
          </div>
        ) : null}
        {actionError ? (
          <p className="error" role="alert">
            {actionError}
          </p>
        ) : null}
        <p className="settings-note">
          Your preferences are saved automatically.
        </p>
      </dialog>
    </main>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="debug-row">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}
function Icon({
  name,
}: {
  name: "settings" | "file" | "account" | "activity";
}) {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {name === "settings" ? (
        <>
          <path d="m10 3-.6 2.2-2 .9L5.3 5.5 3.5 8.6 5 10.2v2.5l-1.5 1.6 1.8 3.1 2.1-.6 2 .9.6 2.3h3.6l.6-2.3 2-.9 2.1.6 1.8-3.1-1.5-1.6v-2.5l1.5-1.6-1.8-3.1-2.1.6-2-.9-.6-2.2Z" />
          <circle cx="11.8" cy="11.5" r="3" />
        </>
      ) : name === "file" ? (
        <>
          <path d="M14 3H6v18h12V7l-4-4Z" />
          <path d="M14 3v5h4M9 12h6M9 16h4" />
        </>
      ) : name === "account" ? (
        <>
          <circle cx="12" cy="8" r="3" />
          <path d="M5 21v-3a7 7 0 0 1 14 0v3" />
        </>
      ) : (
        <path d="M3 12h4l3-7 4 14 3-7h4" />
      )}
    </svg>
  );
}
export default App;
