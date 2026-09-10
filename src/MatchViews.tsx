import { useEffect, useState } from "react";
import CardName from "./card-name";
import CardRichText from "./card-rich-text";
import {
  listMatches,
  loadMatchReport,
  parseInvokeError,
  removeMatch,
  type CardRef,
  type MatchPage,
  type MatchReport,
  type TimelineEvent,
} from "./matches";

export function MatchList({
  page,
  onOpen,
  onBack,
  onPage,
}: {
  page: number;
  onOpen: (clientMatchId: string) => void;
  onBack: () => void;
  onPage: (page: number) => void;
}) {
  const [listing, setListing] = useState<MatchPage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    setError(null);
    void listMatches(page)
      .then((next) => {
        if (!disposed) {
          setListing(next);
        }
      })
      .catch((cause) => {
        if (!disposed) {
          setError(parseInvokeError(cause).message);
        }
      });
    return () => {
      disposed = true;
    };
  }, [page]);

  return (
    <section className="report-card" aria-labelledby="matches-title">
      <div className="account-heading">
        <div>
          <button className="text-button" type="button" onClick={onBack}>
            Back
          </button>
          <h2 id="matches-title">Matches</h2>
          <p>Parsed Arena games and coaching status.</p>
        </div>
      </div>
      {error ? (
        <p className="error" role="alert">
          {error}
        </p>
      ) : listing == null ? (
        <p className="report-waiting">Loading your matches…</p>
      ) : listing.data.length === 0 ? (
        <p className="report-waiting">
          No matches yet. Play a game with the companion to see it here.
        </p>
      ) : (
        <ul className="match-list">
          {listing.data.map((match) => (
            <li key={match.id}>
              <button
                className="match-row"
                type="button"
                onClick={() => onOpen(match.client_match_id)}
              >
                <strong>{match.event_id}</strong>
                <span className={`result-pill ${match.result}`}>
                  {match.result}
                </span>
                <span className="match-row-meta">
                  {match.format} · {(match.timeline ?? []).length} events
                </span>
                <span className="match-row-id">{match.client_match_id}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {listing && listing.meta.last_page > 1 ? (
        <div className="match-pager">
          <button
            className="text-button"
            type="button"
            disabled={listing.meta.current_page <= 1}
            onClick={() => onPage(listing.meta.current_page - 1)}
          >
            Previous
          </button>
          <span>
            {listing.meta.current_page} / {listing.meta.last_page}
          </span>
          <button
            className="text-button"
            type="button"
            disabled={listing.meta.current_page >= listing.meta.last_page}
            onClick={() => onPage(listing.meta.current_page + 1)}
          >
            Next
          </button>
        </div>
      ) : null}
    </section>
  );
}

export function MatchShow({
  clientMatchId,
  liveReport,
  onBack,
  onDeleted,
}: {
  clientMatchId: string;
  liveReport: MatchReport | null;
  onBack: () => void;
  onDeleted: () => void;
}) {
  const [match, setMatch] = useState<MatchReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let disposed = false;
    const load = () => {
      void loadMatchReport(clientMatchId)
        .then((next) => {
          if (!disposed) {
            setMatch(next);
            setError(null);
          }
        })
        .catch((cause) => {
          if (!disposed) {
            setError(parseInvokeError(cause).message);
          }
        });
    };
    load();
    const timer = window.setInterval(() => {
      if (match?.coaching_status === "pending") {
        load();
      }
    }, 3000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [clientMatchId, match?.coaching_status]);

  useEffect(() => {
    if (
      liveReport &&
      liveReport.client_match_id === clientMatchId
    ) {
      setMatch((current) =>
        current
          ? {
              ...current,
              ...liveReport,
              player_seat: liveReport.player_seat ?? current.player_seat,
              deck_grp_ids: liveReport.deck_grp_ids ?? current.deck_grp_ids,
              timeline: liveReport.timeline ?? current.timeline,
              cards: {
                ...(current.cards ?? {}),
                ...(liveReport.cards ?? {}),
              },
            }
          : liveReport,
      );
    }
  }, [clientMatchId, liveReport]);

  async function onDelete() {
    if (!window.confirm("Delete this match? This cannot be undone.")) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await removeMatch(clientMatchId);
      onDeleted();
    } catch (cause) {
      setError(parseInvokeError(cause).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="report-card" aria-labelledby="match-title">
      <div className="account-heading">
        <div>
          <button className="text-button" type="button" onClick={onBack}>
            Back
          </button>
          <h2 id="match-title">{match?.event_id ?? "Match"}</h2>
          <p>{clientMatchId}</p>
        </div>
        <div className="match-actions">
          <button
            className="text-button danger"
            type="button"
            disabled={busy}
            onClick={() => void onDelete()}
          >
            Delete
          </button>
        </div>
      </div>
      {error ? (
        <p className="error" role="alert">
          {error}
        </p>
      ) : null}
      {match == null ? (
        <p className="report-waiting">Loading this match…</p>
      ) : (
        <>
          <dl className="match-meta">
            <div>
              <dt>Result</dt>
              <dd className={`result-pill ${match.result}`}>{match.result}</dd>
            </div>
            <div>
              <dt>Format</dt>
              <dd>{match.format}</dd>
            </div>
            <div>
              <dt>Player seat</dt>
              <dd>{match.player_seat ?? "—"}</dd>
            </div>
            <div>
              <dt>Coaching</dt>
              <dd>{match.coaching_status}</dd>
            </div>
          </dl>
          <CoachingBody
            report={match}
            cards={match.cards ?? {}}
            waiting={match.coaching_status === "pending"}
          />
          <TimelineTable events={match.timeline ?? []} cards={match.cards ?? {}} />
        </>
      )}
    </section>
  );
}

export function CoachingBody({
  report,
  cards = {},
  waiting,
}: {
  report: MatchReport | null;
  cards?: Record<string, CardRef>;
  waiting: boolean;
}) {
  if (waiting) {
    return <p className="report-waiting">Notes will appear here instantly.</p>;
  }
  if (report?.coaching_status === "empty") {
    return (
      <p className="report-waiting">
        No timeline to coach. Enable Detailed Logs and play another game.
      </p>
    );
  }
  if (report?.coaching_status === "failed") {
    return (
      <p className="error" role="alert">
        Coaching failed
        {report.coaching_error ? `: ${report.coaching_error}` : "."}
      </p>
    );
  }
  if (report?.coaching_status === "ready") {
    return (
      <div className="report-body">
        {report.analysis ? (
          <p className="report-analysis">
            <CardRichText text={report.analysis} cards={cards} />
          </p>
        ) : null}
        {(report.tips ?? []).length > 0 ? (
          <ol className="report-tips">
            {(report.tips ?? []).map((tip, index) => (
              <li key={`${tip.turn}-${index}`}>
                <div className="report-tip-head">
                  <span>Turn {tip.turn}</span>
                  <strong>{tip.title}</strong>
                </div>
                <p>
                  <CardRichText text={tip.body} cards={cards} />
                </p>
                {tip.better_line ? (
                  <p className="report-better">
                    <span>Better line</span>
                    <CardRichText text={tip.better_line} cards={cards} />
                  </p>
                ) : null}
              </li>
            ))}
          </ol>
        ) : null}
      </div>
    );
  }
  return (
    <p className="report-waiting">
      Your next completed match will sync automatically.
    </p>
  );
}

function TimelineTable({
  events,
  cards,
}: {
  events: TimelineEvent[];
  cards: Record<string, CardRef>;
}) {
  if (events.length === 0) {
    return <p className="report-waiting">No timeline events.</p>;
  }

  return (
    <div className="timeline-wrap">
      <h3>Timeline</h3>
      <table className="timeline-table">
        <thead>
          <tr>
            <th>#</th>
            <th>Turn</th>
            <th>Phase</th>
            <th>Who</th>
            <th>Action</th>
            <th>Cards</th>
          </tr>
        </thead>
        <tbody>
          {events.map((event, index) => {
            const newTurn = index === 0 || event.turn !== events[index - 1]?.turn;
            return (
              <tr key={index} className={newTurn ? "new-turn" : undefined}>
                <td>{index}</td>
                <td>{newTurn ? (event.turn ?? "—") : ""}</td>
                <td>{formatPhase(event)}</td>
                <td>{actorLabel(event.actor)}</td>
                <td>{event.kind}</td>
                <td>
                  <CardLabels ids={event.card_ids} cards={cards} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function CardLabels({
  ids,
  cards,
}: {
  ids: number[];
  cards: Record<string, CardRef>;
}) {
  if (ids.length === 0) {
    return "—";
  }

  return (
    <>
      {ids.map((id, index) => (
        <span key={`${id}-${index}`}>
          {index > 0 ? ", " : ""}
          <CardName grpId={id} label={cards[String(id)]?.name ?? `#${id}`} />
        </span>
      ))}
    </>
  );
}

export function formatPhase(event: TimelineEvent): string {
  const phase = event.phase?.replace("Phase_", "") ?? null;
  const step = event.step?.replace("Step_", "") ?? null;
  if (phase && step && step !== phase) {
    return `${phase} · ${step}`;
  }
  return phase ?? step ?? "—";
}

export function actorLabel(actor: TimelineEvent["actor"]): string {
  if (actor === "me") {
    return "You";
  }
  if (actor === "opponent") {
    return "Opp";
  }
  return "game";
}

