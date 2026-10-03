import { invoke } from "@tauri-apps/api/core";

/** The shape of every error a Rust command rejects with. */
export type AppError = {
  code: string;
  message: string;
  fields: { field: string; message: string }[];
};

/** Calls a Rust command. A rejected promise carries an AppError. */
export function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args);
}
