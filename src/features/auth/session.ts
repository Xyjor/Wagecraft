import { createContext, useContext } from "react";
import type { Me } from "@/bindings/Me";

export type SessionValue = { me: Me; signOut: () => Promise<void> };

export const SessionContext = createContext<SessionValue | null>(null);

/** The signed-in user. Only usable inside AuthGate's children. */
export function useSession(): SessionValue {
  const value = useContext(SessionContext);
  if (!value) throw new Error("useSession must be used inside <AuthGate>");
  return value;
}
