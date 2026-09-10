import { describe, expect, test } from "bun:test";
import { actorLabel, formatPhase } from "../src/MatchViews";
import { timelineEvent } from "./fixtures";

describe("formatPhase", () => {
  test("joins a distinct phase and step", () => {
    expect(
      formatPhase(
        timelineEvent({
          phase: "Phase_Combat",
          step: "Step_DeclareAttackers",
        }),
      ),
    ).toBe("Combat · DeclareAttackers");
  });

  test("hides a step that repeats the phase name", () => {
    expect(
      formatPhase(timelineEvent({ phase: "Phase_Main", step: "Step_Main" })),
    ).toBe("Main");
  });

  test("falls back to a dash when both are missing", () => {
    expect(formatPhase(timelineEvent({ phase: null, step: null }))).toBe("—");
  });
});

describe("actorLabel", () => {
  test("labels the local player and opponent", () => {
    expect(actorLabel("me")).toBe("You");
    expect(actorLabel("opponent")).toBe("Opp");
    expect(actorLabel("game")).toBe("game");
  });
});
