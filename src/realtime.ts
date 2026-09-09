import Echo from "laravel-echo";
import Pusher, { type ChannelAuthorizationCallback } from "pusher-js";
import { invoke } from "@tauri-apps/api/core";
import { loadMatchReport, type MatchReport } from "./matches";

export type { CoachingStatus, CoachingTip, MatchReport } from "./matches";
export { loadMatchReport };

export type RealtimeConfig = {
  user_id: number;
  channel: string;
  reverb: {
    key: string;
    host: string;
    port: number;
    scheme: string;
  };
};

type PusherChannel = {
  name: string;
};

export async function loadRealtimeConfig(): Promise<RealtimeConfig> {
  return invoke<RealtimeConfig>("get_realtime_config");
}

export function connectRealtime(
  config: RealtimeConfig,
  onReport: (report: MatchReport) => void,
): Echo<"reverb"> {
  const echo = new Echo({
    broadcaster: "reverb",
    Pusher,
    key: config.reverb.key,
    wsHost: config.reverb.host,
    wsPort: config.reverb.port,
    wssPort: config.reverb.port,
    forceTLS: config.reverb.scheme === "https",
    enabledTransports: ["ws", "wss"],
    disableStats: true,
    withoutInterceptors: true,
    authorizer: (channel: PusherChannel) => ({
      authorize: (socketId: string, callback: ChannelAuthorizationCallback) => {
        invoke<{ auth: string }>("authorize_channel", {
          socketId,
          channelName: channel.name,
        })
          .then((data) => callback(null, data))
          .catch((error: unknown) =>
            callback(
              error instanceof Error ? error : new Error(String(error)),
              null,
            ),
          );
      },
    }),
  });

  echo
    .private(config.channel)
    .listen(".match.coached", (payload: MatchReport) => {
      onReport(payload);
    });

  return echo;
}
