import type { NewUser } from "@/bindings/NewUser";
import type { Role } from "@/bindings/Role";
import type { UserSummary } from "@/bindings/UserSummary";
import { call } from "@/lib/ipc";

export const listUsers = () => call<UserSummary[]>("user_list");
export const createUser = (input: NewUser) => call<UserSummary>("user_create", { input });
export const setRole = (id: number, role: Role) => call<void>("user_update", { id, role });
export const setActive = (id: number, active: boolean) =>
  call<void>("user_set_active", { id, active });
export const resetPassword = (id: number, temporaryPassword: string) =>
  call<void>("user_reset_password", { id, temporaryPassword });
