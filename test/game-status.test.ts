import { describe, expect, test } from "bun:test";
import { gameStatus } from "../src/game-status";
import { watcherStatus } from "./fixtures";

describe("gameStatus", () => {
  test("asks the player to wait while the companion connects", () => {
    expect(gameStatus(null)).toEqual({
      title: "Finding your games",
      detail: "Connecting to your Arena companion…",
      tone: "waiting",
    });
  });

  test("waits for Arena when the game file is missing", () => {
    expect(gameStatus(watcherStatus({ log_exists: false }))).toMatchObject({
      title: "Waiting for Arena",
      tone: "waiting",
    });
  });

  test("explains the Detailed Logs setup step", () => {
    expect(
      gameStatus(watcherStatus({ log_exists: true, detailed_logs: false })),
    ).toMatchObject({
      title: "One small setup step",
      tone: "waiting",
    });
  });

  test("surfaces a watcher error after the log is found", () => {
    expect(
      gameStatus(watcherStatus({ phase: "error", log_exists: true })),
    ).toMatchObject({
      title: "Your companion needs attention",
      tone: "waiting",
    });
  });

  test("treats a live match, upload, and synced match as active", () => {
    expect(
      gameStatus(watcherStatus({ phase: "match_in_progress" })).tone,
    ).toBe("live");
    expect(gameStatus(watcherStatus({ phase: "uploading" })).title).toBe(
      "Syncing your match",
    );
    expect(gameStatus(watcherStatus({ phase: "uploaded" })).title).toBe(
      "Match synced",
    );
  });

  test("falls back to the connected watching copy", () => {
    expect(gameStatus(watcherStatus({ phase: "watching" }))).toEqual({
      title: "Reading your games",
      detail:
        "Your Arena game file is connected. Play as usual — we’ll follow along.",
      tone: "live",
    });
  });
});
