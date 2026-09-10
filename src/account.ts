import { invoke } from "@tauri-apps/api/core";
import type {
  CreateAccountPayload,
  DeleteAccountPayload,
  SignInPayload,
  UpdateAccountPayload,
} from "./AccountPanel";
import type { WatcherStatus } from "./game-status";
import { parseInvokeError } from "./matches";

export async function signIn(payload: SignInPayload): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("sign_in", { payload });
}

export async function signOut(): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("sign_out");
}

export async function createAccount(
  payload: CreateAccountPayload,
): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("create_account", { payload });
}

export async function updateAccount(
  payload: UpdateAccountPayload,
): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("update_account", { payload });
}

export async function deleteAccount(
  payload: DeleteAccountPayload,
): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("delete_account", { payload });
}

export async function loadAccount(): Promise<WatcherStatus> {
  return invoke<WatcherStatus>("get_account");
}

export function accountActionError(error: unknown, fallback: string): string {
  const parsed = parseInvokeError(error);
  const fields = Object.values(parsed.errors).flat();
  if (fields.length > 0) {
    return fields.join(" ");
  }
  if (parsed.message && parsed.message !== "error" && !parsed.message.startsWith("HTTP ")) {
    return parsed.message;
  }
  return fallback;
}
