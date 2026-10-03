import { createBrowserRouter } from "react-router";
import { AppShell } from "./AppShell";
import { EmployeeFormPage } from "@/features/employees/EmployeeFormPage";
import { EmployeeProfilePage } from "@/features/employees/EmployeeProfilePage";
import { EmployeesPage } from "@/features/employees/EmployeesPage";
import { HomePage } from "@/features/home/HomePage";
import { OrganizationPage } from "@/features/org/OrganizationPage";
import { UsersPage } from "@/features/users/UsersPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "employees", element: <EmployeesPage /> },
      { path: "employees/new", element: <EmployeeFormPage /> },
      { path: "employees/:id", element: <EmployeeProfilePage /> },
      { path: "employees/:id/edit", element: <EmployeeFormPage /> },
      { path: "organization", element: <OrganizationPage /> },
      { path: "users", element: <UsersPage /> },
    ],
  },
]);
