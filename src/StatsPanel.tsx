import { winRateLabel, statsCaption, type CompanionStats } from "./local-stats";

export function StatsPanel({
  stats,
  busy,
  onReset,
}: {
  stats: CompanionStats;
  busy?: boolean;
  onReset?: () => void;
}) {
  return (
    <section className="stats-card" aria-labelledby="stats-title">
      <div className="account-heading">
        <div className="small-icon" aria-hidden="true">
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M4 19V9M10 19V5M16 19v-7M22 19H2" />
          </svg>
        </div>
        <div>
          <h2 id="stats-title">Your record</h2>
          <p>{statsCaption(stats)}</p>
        </div>
        {onReset ? (
          <div className="match-actions">
            <button
              className="text-button"
              type="button"
              disabled={busy || stats.games === 0}
              onClick={onReset}
            >
              Reset
            </button>
          </div>
        ) : null}
      </div>
      <dl className="stats-grid">
        <div className="stats-metric">
          <dt>Games</dt>
          <dd>{stats.games}</dd>
        </div>
        <div className="stats-metric win">
          <dt>Wins</dt>
          <dd>{stats.wins}</dd>
        </div>
        <div className="stats-metric loss">
          <dt>Losses</dt>
          <dd>{stats.losses}</dd>
        </div>
        <div className="stats-metric">
          <dt>Win rate</dt>
          <dd>{winRateLabel(stats)}</dd>
        </div>
      </dl>
    </section>
  );
}
