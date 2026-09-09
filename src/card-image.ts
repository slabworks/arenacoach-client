import { invoke } from "@tauri-apps/api/core";

const imageCache = new Map<number, string>();
const inflight = new Map<number, Promise<string | null>>();

export function loadCardImage(grpId: number): Promise<string | null> {
  const cached = imageCache.get(grpId);

  if (cached) {
    return Promise.resolve(cached);
  }

  const pending = inflight.get(grpId);

  if (pending) {
    return pending;
  }

  const request = invoke<string | null>("get_card_image", { grpId })
    .then((url) => {
      if (!url) {
        return null;
      }

      imageCache.set(grpId, url);

      return url;
    })
    .catch(() => null)
    .finally(() => {
      inflight.delete(grpId);
    });

  inflight.set(grpId, request);

  return request;
}
