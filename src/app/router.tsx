import { createBrowserRouter } from "react-router";
import { AppShell } from "./AppShell";
import { HomePage } from "@/features/home/HomePage";
import { OrganizationPage } from "@/features/org/OrganizationPage";
import { UsersPage } from "@/features/users/UsersPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "organization", element: <OrganizationPage /> },
      { path: "users", element: <UsersPage /> },
    ],
  },
]);
