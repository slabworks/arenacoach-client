import { describe, expect, test } from "bun:test";
import { community } from "../src/community";

describe("community", () => {
  test("points Discord and Patreon at the public channels", () => {
    expect(community.discord).toBe("https://discord.gg/9AjGBjGp6Q");
    expect(community.patreon).toBe("https://www.patreon.com/c/slabworks");
  });
});
