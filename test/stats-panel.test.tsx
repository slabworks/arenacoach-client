/// <reference lib="dom" />

import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { StatsPanel } from "../src/StatsPanel";
import { EMPTY_STATS } from "../src/local-stats";

describe("StatsPanel", () => {
  test("is visible with an empty all-time record", () => {
    render(<StatsPanel stats={EMPTY_STATS} />);

    expect(screen.getByRole("heading", { name: "Your record" })).toBeInTheDocument();
    expect(screen.getByText("Games").closest("div")).toHaveTextContent("0");
    expect(screen.getByText("Win rate").closest("div")).toHaveTextContent("—");
  });

  test("shows saved wins and losses", () => {
    render(
      <StatsPanel stats={{ games: 10, wins: 6, losses: 4, unknown: 0 }} />,
    );

    expect(screen.getByText("Games").closest("div")).toHaveTextContent("10");
    expect(screen.getByText("Wins").closest("div")).toHaveTextContent("6");
    expect(screen.getByText("Losses").closest("div")).toHaveTextContent("4");
    expect(screen.getByText("Win rate").closest("div")).toHaveTextContent("60%");
  });

  test("disables reset until a game has been processed", () => {
    const { rerender } = render(
      <StatsPanel stats={EMPTY_STATS} onReset={() => undefined} />,
    );
    expect(screen.getByRole("button", { name: "Reset" })).toBeDisabled();

    rerender(
      <StatsPanel
        stats={{ games: 1, wins: 1, losses: 0, unknown: 0 }}
        onReset={() => undefined}
      />,
    );
    expect(screen.getByRole("button", { name: "Reset" })).toBeEnabled();
  });
});
