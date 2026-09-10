/// <reference lib="dom" />

import { describe, expect, test } from "bun:test";
import { render, screen } from "@testing-library/react";
import { CoachingBody } from "../src/MatchViews";
import { matchReport } from "./fixtures";

describe("CoachingBody", () => {
  test("shows a waiting note while coaching is still pending", () => {
    render(
      <CoachingBody
        report={matchReport({ coaching_status: "pending" })}
        waiting
      />,
    );

    expect(
      screen.getByText("Notes will appear here instantly."),
    ).toBeInTheDocument();
  });

  test("explains an empty timeline", () => {
    render(
      <CoachingBody
        report={matchReport({ coaching_status: "empty" })}
        waiting={false}
      />,
    );

    expect(
      screen.getByText(/No timeline to coach/),
    ).toBeInTheDocument();
  });

  test("surfaces a coaching failure", () => {
    render(
      <CoachingBody
        report={matchReport({
          coaching_status: "failed",
          coaching_error: "model timed out",
        })}
        waiting={false}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Coaching failed: model timed out",
    );
  });

  test("renders analysis and tips when coaching is ready", () => {
    render(
      <CoachingBody
        report={matchReport({
          coaching_status: "ready",
          analysis: "Keep Shock and the mountain.",
          tips: [
            {
              turn: 3,
              title: "Hold the removal",
              body: "Shock the flyer next turn.",
              better_line: "Wait for the second creature.",
              cite: 12,
            },
          ],
        })}
        cards={{ "10": { name: "Shock" } }}
        waiting={false}
      />,
    );

    expect(screen.getByText(/Keep/)).toBeInTheDocument();
    expect(screen.getAllByText("Shock").length).toBeGreaterThan(0);
    expect(screen.getByText("Turn 3")).toBeInTheDocument();
    expect(screen.getByText("Hold the removal")).toBeInTheDocument();
    expect(screen.getByText("Better line")).toBeInTheDocument();
    expect(screen.getByText("Wait for the second creature.")).toBeInTheDocument();
  });

  test("prompts the player to finish a match when there is nothing to show", () => {
    render(<CoachingBody report={null} waiting={false} />);

    expect(
      screen.getByText("Your next completed match will sync automatically."),
    ).toBeInTheDocument();
  });
});
