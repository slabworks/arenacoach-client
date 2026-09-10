import { beforeEach, describe, expect, mock, test } from "bun:test";

const invoke = mock(async (_command: string, args?: { grpId: number }) => {
  return `https://cards.test/${args?.grpId}.png`;
});

mock.module("@tauri-apps/api/core", () => ({
  invoke,
}));

const { loadCardImage } = await import("../src/card-image");

describe("loadCardImage", () => {
  beforeEach(() => {
    invoke.mockClear();
    invoke.mockImplementation(async (_command, args) => {
      return `https://cards.test/${args?.grpId}.png`;
    });
  });

  test("asks Tauri for a card image once and reuses the result", async () => {
    const first = await loadCardImage(73951);
    const second = await loadCardImage(73951);

    expect(first).toBe("https://cards.test/73951.png");
    expect(second).toBe(first);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("get_card_image", { grpId: 73951 });
  });

  test("shares one in-flight request for the same card", async () => {
    let finish!: (url: string) => void;
    invoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );

    const first = loadCardImage(12);
    const second = loadCardImage(12);
    finish("https://cards.test/shared.png");

    expect(await first).toBe("https://cards.test/shared.png");
    expect(await second).toBe("https://cards.test/shared.png");
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  test("does not cache a missing image", async () => {
    invoke.mockResolvedValue(null);

    expect(await loadCardImage(99)).toBeNull();
    expect(await loadCardImage(99)).toBeNull();
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  test("returns null when the native lookup fails", async () => {
    invoke.mockRejectedValue(new Error("offline"));

    expect(await loadCardImage(404)).toBeNull();
  });
});
