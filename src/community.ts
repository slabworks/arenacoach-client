export const community = {
  discord: "https://discord.gg/9AjGBjGp6Q",
  patreon: "https://www.patreon.com/c/slabworks",
} as const;

export async function openCommunity(url: string) {
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}
