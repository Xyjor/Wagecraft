import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router";
import { router } from "./app/router";
import { AuthGate } from "./features/auth/AuthGate";
import { ThemeSync } from "./lib/useTheme";
import "@fontsource-variable/ibm-plex-sans";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeSync />
    <AuthGate onUserChanged={() => router.navigate("/")}>
      <RouterProvider router={router} />
    </AuthGate>
  </React.StrictMode>,
);
