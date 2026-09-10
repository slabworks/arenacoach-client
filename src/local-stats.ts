export type CompanionStats = {
  games: number;
  wins: number;
  losses: number;
  unknown: number;
};

export const EMPTY_STATS: CompanionStats = {
  games: 0,
  wins: 0,
  losses: 0,
  unknown: 0,
};

export function winRateLabel(stats: CompanionStats): string {
  const decided = stats.wins + stats.losses;
  if (decided === 0) {
    return "—";
  }
  return `${Math.round((stats.wins / decided) * 100)}%`;
}

export function statsCaption(stats: CompanionStats): string {
  if (stats.games === 0) {
    return "Games processed while this companion is open are saved here.";
  }
  if (stats.unknown > 0) {
    return `All-time record · ${stats.unknown} without a result yet.`;
  }
  return "All-time record from games this companion processed.";
}
