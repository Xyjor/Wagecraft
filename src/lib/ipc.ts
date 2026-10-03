import { invoke } from "@tauri-apps/api/core";

export type AppError = {
  code: string;
  message: string;
  fields: { field: string; message: string }[];
};

/** Fired on window when the backend says the session is gone. AuthGate listens for it. */
export const SIGNED_OUT_EVENT = "wagecraft:signed-out";

const SIGNED_OUT_CODES = new Set(["SESSION_EXPIRED", "UNAUTHENTICATED"]);

function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

/** Calls a Tauri command. Every rejection is an AppError, so screens can show `message`. */
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (!isAppError(e)) {
      console.error(`${cmd} failed`, e);
      const internal: AppError = {
        code: "INTERNAL",
        message: "Something went wrong. Please try again",
        fields: [],
      };
      throw internal;
    }
    if (SIGNED_OUT_CODES.has(e.code)) window.dispatchEvent(new Event(SIGNED_OUT_EVENT));
    throw e;
  }
}
