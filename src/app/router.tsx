import { createBrowserRouter } from "react-router";
import { AppShell } from "./AppShell";
import { HomePage } from "@/features/home/HomePage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [{ index: true, element: <HomePage /> }],
  },
]);
