import { describe, expect, test } from "bun:test";
import {
  EMPTY_STATS,
  statsCaption,
  winRateLabel,
} from "../src/local-stats";

describe("local stats", () => {
  test("hides a win rate until a game has a result", () => {
    expect(winRateLabel(EMPTY_STATS)).toBe("—");
    expect(winRateLabel({ ...EMPTY_STATS, games: 2, unknown: 2 })).toBe("—");
  });

  test("uses decided games only for win rate", () => {
    expect(
      winRateLabel({ games: 5, wins: 2, losses: 2, unknown: 1 }),
    ).toBe("50%");
    expect(
      winRateLabel({ games: 3, wins: 2, losses: 1, unknown: 0 }),
    ).toBe("67%");
  });

  test("explains empty and all-time records", () => {
    expect(statsCaption(EMPTY_STATS)).toMatch(/processed while this companion/);
    expect(
      statsCaption({ games: 4, wins: 3, losses: 1, unknown: 0 }),
    ).toMatch(/All-time record/);
    expect(
      statsCaption({ games: 4, wins: 2, losses: 1, unknown: 1 }),
    ).toMatch(/1 without a result/);
  });
});
