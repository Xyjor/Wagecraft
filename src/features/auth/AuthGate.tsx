import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import type { Me } from "@/bindings/Me";
import type { SetupStatus } from "@/bindings/SetupStatus";
import { call, SIGNED_OUT_EVENT } from "@/lib/ipc";
import { ChangePasswordPage } from "./ChangePasswordPage";
import { LoginPage } from "./LoginPage";
import { SessionContext } from "./session";
import { SetupWizard } from "./SetupWizard";

type State =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "signedOut"; notice?: string }
  | { kind: "signedIn"; me: Me };

/**
 * Decides what the window shows: first-run setup, sign-in, the forced password
 * change, or the app itself. The backend still checks every command; this only
 * keeps people from seeing screens they can't use.
 */
export function AuthGate({ children }: { children: ReactNode }) {
  const [state, setState] = useState<State>({ kind: "loading" });

  useEffect(() => {
    let live = true;
    (async () => {
      const status = await call<SetupStatus>("auth_setup_status");
      if (status.needsSetup) return { kind: "setup" } as const;
      try {
        return { kind: "signedIn", me: await call<Me>("auth_me") } as const;
      } catch {
        return { kind: "signedOut" } as const;
      }
    })()
      .catch(() => ({ kind: "signedOut" }) as const)
      .then((next) => live && setState(next));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    const onSignedOut = () =>
      setState((s) =>
        s.kind === "signedIn"
          ? { kind: "signedOut", notice: "You were signed out. Please sign in again." }
          : s,
      );
    window.addEventListener(SIGNED_OUT_EVENT, onSignedOut);
    return () => window.removeEventListener(SIGNED_OUT_EVENT, onSignedOut);
  }, []);

  const signIn = useCallback((me: Me) => setState({ kind: "signedIn", me }), []);
  const signOut = useCallback(async () => {
    try {
      await call("auth_logout");
    } finally {
      setState({ kind: "signedOut" });
    }
  }, []);

  const me = state.kind === "signedIn" ? state.me : null;
  const session = useMemo(() => (me ? { me, signOut } : null), [me, signOut]);

  switch (state.kind) {
    case "loading":
      return null;
    case "setup":
      return <SetupWizard onDone={signIn} />;
    case "signedOut":
      return <LoginPage onSignedIn={signIn} notice={state.notice} />;
    case "signedIn":
      if (state.me.mustChangePassword) return <ChangePasswordPage me={state.me} onDone={signIn} />;
      return <SessionContext.Provider value={session}>{children}</SessionContext.Provider>;
  }
}
