import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import {
  accountActionError,
  createAccount,
  deleteAccount,
  loadAccount,
  signIn,
  signOut,
  updateAccount,
} from "./account";
import {
  AccountPanel,
  type CreateAccountPayload,
  type DeleteAccountPayload,
  type SignInPayload,
  type UpdateAccountPayload,
} from "./AccountPanel";
import { gameStatus, type WatcherStatus } from "./game-status";
import { community, openCommunity } from "./community";
import { EMPTY_STATS } from "./local-stats";
import { CoachingBody, MatchList, MatchShow } from "./MatchViews";
import { StatsPanel } from "./StatsPanel";
import {
  connectRealtime,
  loadMatchReport,
  loadRealtimeConfig,
  type MatchReport,
} from "./realtime";

type View =
  | { name: "home" }
  | { name: "matches"; page: number }
  | { name: "show"; id: string };

function App() {
  const [status, setStatus] = useState<WatcherStatus | null>(null);
  const [connectionError, setConnectionError] = useState(false);
  const [needsTwoFactor, setNeedsTwoFactor] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [report, setReport] = useState<MatchReport | null>(null);
  const [view, setView] = useState<View>({ name: "home" });
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

  useEffect(() => {
    if (!status?.has_token) {
      return;
    }
    let disposed = false;
    let echo: ReturnType<typeof connectRealtime> | undefined;
    void loadRealtimeConfig()
      .then((config) => {
        if (disposed) {
          return;
        }
        echo = connectRealtime(config, (next) => {
          if (!disposed) {
            setReport(next);
          }
        });
      })
      .catch(() => {
        // Polling still recovers the report if Reverb is down.
      });
    return () => {
      disposed = true;
      echo?.disconnect();
    };
  }, [status?.has_token, status?.api_base]);

  useEffect(() => {
    const matchId = status?.last_match_id;
    if (!status?.has_token || !matchId) {
      return;
    }
    if (
      report?.client_match_id === matchId &&
      report.coaching_status !== "pending"
    ) {
      return;
    }
    let disposed = false;
    const load = () => {
      void loadMatchReport(matchId)
        .then((next) => {
          if (!disposed) {
            setReport(next);
          }
        })
        .catch(() => {
          // Keep waiting; the next tick or Reverb event will retry.
        });
    };
    load();
    const timer = window.setInterval(load, 3000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [
    status?.has_token,
    status?.last_match_id,
    report?.client_match_id,
    report?.coaching_status,
  ]);

  useEffect(() => {
    if (!status?.has_token) {
      setReport(null);
      setView({ name: "home" });
    }
  }, [status?.has_token]);

  useEffect(() => {
    if (!status?.has_token) {
      return;
    }
    let disposed = false;
    void loadAccount()
      .then((next) => {
        if (!disposed) {
          setStatus(next);
        }
      })
      .catch(() => {
        // Keep the cached profile if the host is unreachable.
      });
    return () => {
      disposed = true;
    };
  }, [status?.has_token]);

  async function updateSetting(patch: {
    developer_mode?: boolean;
    show_debug_info?: boolean;
  }) {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await invoke<WatcherStatus>("update_settings", { patch }));
      if (patch.developer_mode !== undefined) {
        setNeedsTwoFactor(false);
      }
    } catch {
      setActionError("Couldn’t save your settings. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function connectAccount(payload: SignInPayload) {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await signIn(payload));
      setNeedsTwoFactor(false);
    } catch (error) {
      if (String(error) === "two_factor") {
        setNeedsTwoFactor(true);
        setActionError("Enter the code from your authenticator to continue.");
      } else {
        setActionError(
          accountActionError(
            error,
            "Couldn’t sign in. Check your details and connection, then try again.",
          ),
        );
      }
    } finally {
      setBusy(false);
    }
  }

  async function registerAccount(payload: CreateAccountPayload) {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await createAccount(payload));
      setNeedsTwoFactor(false);
    } catch (error) {
      setActionError(
        accountActionError(
          error,
          "Couldn’t create your account. Check your details and try again.",
        ),
      );
    } finally {
      setBusy(false);
    }
  }

  async function saveAccount(payload: UpdateAccountPayload) {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await updateAccount(payload));
    } catch (error) {
      setActionError(
        accountActionError(
          error,
          "Couldn’t update your account. Check your details and try again.",
        ),
      );
    } finally {
      setBusy(false);
    }
  }

  async function removeAccount(payload: DeleteAccountPayload) {
    if (
      !window.confirm(
        "Delete your Arena Coach account? This cannot be undone.",
      )
    ) {
      return;
    }
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await deleteAccount(payload));
    } catch (error) {
      setActionError(
        accountActionError(
          error,
          "Couldn’t delete your account. Check your password and try again.",
        ),
      );
    } finally {
      setBusy(false);
    }
  }

  async function disconnectAccount() {
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await signOut());
    } catch {
      setActionError("Couldn’t sign out. Please try again.");
    } finally {
      setBusy(false);
    }
  }

  async function resetStats() {
    if (
      !window.confirm(
        "Reset your all-time record? This only clears local companion stats.",
      )
    ) {
      return;
    }
    setBusy(true);
    setActionError(null);
    try {
      setStatus(await invoke<WatcherStatus>("reset_stats"));
    } catch {
      setActionError("Couldn’t reset your record. Please try again.");
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
        <button
          className="brand"
          type="button"
          onClick={() => setView({ name: "home" })}
        >
          <div className="brand-mark" aria-hidden="true">
            A<span>✦</span>
          </div>
          <div>
            <span className="brand-name">
              arena<span>coach</span>
            </span>
            <span className="brand-caption">YOUR ARENA COMPANION</span>
          </div>
        </button>
        <div className="header-actions">
          {status?.has_token ? (
            <button
              className="text-button"
              type="button"
              onClick={() => setView({ name: "matches", page: 1 })}
            >
              Matches
            </button>
          ) : null}
          <a
            className="icon-button"
            href={community.discord}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Join Discord"
            title="Discord"
            onClick={(event) => {
              event.preventDefault();
              void openCommunity(community.discord);
            }}
          >
            <Icon name="discord" />
          </a>
          <a
            className="icon-button"
            href={community.patreon}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Support on Patreon"
            title="Patreon"
            onClick={(event) => {
              event.preventDefault();
              void openCommunity(community.patreon);
            }}
          >
            <Icon name="patreon" />
          </a>
          <button
            className="icon-button"
            aria-label="Open settings"
            title="Settings"
            onClick={() => setSettingsOpen(true)}
            disabled={!status}
          >
            <Icon name="settings" />
          </button>
        </div>
      </header>

      <div className="main-content">
        {view.name === "home" ? (
          <div className="home-grid">
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

        {status?.has_token ? (
          <ReportCard
            report={
              report?.client_match_id === status.last_match_id ? report : null
            }
            waiting={
              Boolean(status.last_match_id) &&
              (report?.client_match_id !== status.last_match_id ||
                report?.coaching_status === "pending")
            }
            onOpenAll={() => setView({ name: "matches", page: 1 })}
            onOpenLatest={
              status.last_match_id
                ? () => setView({ name: "show", id: status.last_match_id! })
                : undefined
            }
          />
        ) : null}

        <StatsPanel
          stats={status?.stats ?? EMPTY_STATS}
          busy={busy || !status}
          onReset={() => void resetStats()}
        />

        <AccountPanel
          signedIn={Boolean(status?.has_token)}
          name={status?.signed_in_name ?? ""}
          email={status?.signed_in_email ?? ""}
          synced={synced}
          busy={busy}
          ready={Boolean(status)}
          error={settingsOpen ? null : actionError}
          needsTwoFactor={needsTwoFactor}
          onSignIn={(payload) => void connectAccount(payload)}
          onCreate={(payload) => void registerAccount(payload)}
          onUpdate={(payload) => void saveAccount(payload)}
          onDelete={(payload) => void removeAccount(payload)}
          onSignOut={() => void disconnectAccount()}
        />

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
        ) : status?.has_token ? (
          view.name === "matches" ? (
            <MatchList
              page={view.page}
              onOpen={(id) => setView({ name: "show", id })}
              onBack={() => setView({ name: "home" })}
              onPage={(next) => setView({ name: "matches", page: next })}
            />
          ) : (
            <MatchShow
              clientMatchId={view.id}
              liveReport={report}
              onBack={() => setView({ name: "matches", page: 1 })}
              onDeleted={() => setView({ name: "matches", page: 1 })}
            />
          )
        ) : null}
      </div>

      <footer>
        <div className="footer-links">
          <a
            href={community.discord}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Join Discord"
            title="Discord"
            onClick={(event) => {
              event.preventDefault();
              void openCommunity(community.discord);
            }}
          >
            <Icon name="discord" />
          </a>
          <a
            href={community.patreon}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="Support on Patreon"
            title="Patreon"
            onClick={(event) => {
              event.preventDefault();
              void openCommunity(community.patreon);
            }}
          >
            <Icon name="patreon" />
          </a>
        </div>
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
        <div className="setting">
          <span><strong>Match uploads</strong><small>{status?.pending_uploads ?? 0} pending · {status?.failed_uploads ?? 0} rejected</small></span>
          <button type="button" className="secondary-button" disabled={busy} onClick={() => { void invoke("manage_uploads", { discardFailed: false }).catch(() => setActionError("Could not resume uploads.")); }}>Resume uploads</button>
          {(status?.failed_uploads ?? 0) > 0 ? <button type="button" className="secondary-button" onClick={() => { if (window.confirm("Remove rejected uploads from this device?")) { void invoke("manage_uploads", { discardFailed: true }).catch(() => setActionError("Could not remove rejected uploads.")); } }}>Remove rejected</button> : null}
        </div>
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

function ReportCard({
  report,
  waiting,
  onOpenAll,
  onOpenLatest,
}: {
  report: MatchReport | null;
  waiting: boolean;
  onOpenAll: () => void;
  onOpenLatest?: () => void;
}) {
  return (
    <section className="report-card" aria-labelledby="report-title">
      <div className="account-heading">
        <div className="small-icon">
          <Icon name="activity" />
        </div>
        <div>
          <h2 id="report-title">Match report</h2>
          <p>
            {waiting
              ? "The coach is writing notes for your latest game."
              : report?.event_id ?? "Play a match to see notes here."}
          </p>
        </div>
        <div className="match-actions">
          {onOpenLatest ? (
            <button className="text-button" type="button" onClick={onOpenLatest}>
              Open
            </button>
          ) : null}
          <button className="text-button" type="button" onClick={onOpenAll}>
            All matches
          </button>
        </div>
      </div>
      <CoachingBody
        report={report}
        cards={report?.cards ?? {}}
        waiting={waiting}
      />
    </section>
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
  name: "settings" | "file" | "activity" | "discord" | "patreon";
}) {
  if (name === "discord" || name === "patreon") {
    return (
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="currentColor"
        aria-hidden="true"
      >
        {name === "discord" ? (
          <path d="M20.317 4.37a19.8 19.8 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.3 18.3 0 0 0-5.487 0 12.6 12.6 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.7 19.7 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14 14 0 0 0 1.226-1.994.076.076 0 0 0-.042-.106 13 13 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10 10 0 0 0 .372-.291.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.3 12.3 0 0 1-1.873.892.077.077 0 0 0-.041.106c.36.698.772 1.363 1.225 1.994a.076.076 0 0 0 .084.028 19.8 19.8 0 0 0 6.002-3.03.077.077 0 0 0 .032-.055c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03M8.02 15.331c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418m7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418" />
        ) : (
          <path d="M0 .48v23.04h4.22V.48zm15.385 0c-4.764 0-8.641 3.88-8.641 8.65 0 4.755 3.877 8.623 8.641 8.623 4.75 0 8.615-3.868 8.615-8.623C24 4.36 20.136.48 15.385.48" />
        )}
      </svg>
    );
  }

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
      ) : (
        <path d="M3 12h4l3-7 4 14 3-7h4" />
      )}
    </svg>
  );
}
export default App;
