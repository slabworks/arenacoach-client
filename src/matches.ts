import { invoke } from "@tauri-apps/api/core";

export type CoachingStatus = "pending" | "ready" | "empty" | "failed";

export type CoachingTip = {
  turn: number;
  title: string;
  body: string;
  better_line?: string;
  cite: number;
};

export type CardRef = {
  name: string;
  type_line?: string | null;
  oracle_text?: string | null;
};

export type LegalAction = {
  action: string;
  grp_id?: number;
};

export type DecisionContext = {
  my_hand?: number[];
  my_board?: number[];
  opp_board?: number[];
  opp_hand?: number[];
  opp_hand_count?: number;
  my_life?: number;
  opp_life?: number;
  legal?: LegalAction[];
};

export type TimelineEvent = {
  t_ms: number;
  turn: number | null;
  phase: string | null;
  step?: string | null;
  actor: "me" | "opponent" | "game";
  kind:
    | "cast"
    | "land"
    | "attack"
    | "block"
    | "damage"
    | "life"
    | "mulligan"
    | "keep"
    | "draw"
    | "resolve"
    | "die"
    | "zone"
    | "pass"
    | "other";
  card_ids: number[];
  note: string;
  context?: DecisionContext;
};

export type MatchReport = {
  id: number;
  client_match_id: string;
  event_id: string;
  format: string;
  result: string;
  coaching_status: CoachingStatus;
  analysis: string | null;
  tips: CoachingTip[] | null;
  coaching_error: string | null;
  player_seat?: number | null;
  deck_grp_ids?: number[] | null;
  timeline?: TimelineEvent[] | null;
  cards?: Record<string, CardRef>;
  created_at?: string | null;
  updated_at?: string | null;
};

export type MatchPage = {
  data: MatchReport[];
  meta: {
    current_page: number;
    last_page: number;
    total: number;
  };
};

export type FieldErrors = Record<string, string[]>;

export function parseInvokeError(error: unknown): {
  message: string;
  errors: FieldErrors;
} {
  const raw = error instanceof Error ? error.message : String(error);
  const jsonStart = raw.indexOf("{");
  if (jsonStart >= 0) {
    try {
      const parsed = JSON.parse(raw.slice(jsonStart)) as {
        message?: string;
        errors?: FieldErrors;
      };
      return {
        message:
          typeof parsed.message === "string" ? parsed.message : raw,
        errors: parsed.errors ?? {},
      };
    } catch {
      // The Rust error string is not always JSON.
    }
  }
  return { message: raw, errors: {} };
}

export async function listMatches(page = 1): Promise<MatchPage> {
  return invoke<MatchPage>("list_matches", { page });
}

export async function loadMatchReport(
  clientMatchId: string,
): Promise<MatchReport> {
  return invoke<MatchReport>("get_match_report", {
    clientMatchId,
  });
}

export async function removeMatch(clientMatchId: string): Promise<void> {
  await invoke("remove_match", { clientMatchId });
}
