import { createBrowserRouter } from "react-router";
import { AppShell } from "./AppShell";
import { AttendancePage } from "@/features/attendance/AttendancePage";
import { MyAttendancePage } from "@/features/attendance/MyAttendancePage";
import { EmployeeFormPage } from "@/features/employees/EmployeeFormPage";
import { EmployeeProfilePage } from "@/features/employees/EmployeeProfilePage";
import { EmployeesPage } from "@/features/employees/EmployeesPage";
import { MyProfilePage } from "@/features/employees/MyProfilePage";
import { HomePage } from "@/features/home/HomePage";
import { HolidaysPage } from "@/features/org/HolidaysPage";
import { OrganizationPage } from "@/features/org/OrganizationPage";
import { UsersPage } from "@/features/users/UsersPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "me", element: <MyProfilePage /> },
      { path: "my-attendance", element: <MyAttendancePage /> },
      { path: "attendance", element: <AttendancePage /> },
      { path: "employees", element: <EmployeesPage /> },
      { path: "employees/new", element: <EmployeeFormPage /> },
      { path: "employees/:id", element: <EmployeeProfilePage /> },
      { path: "employees/:id/edit", element: <EmployeeFormPage /> },
      { path: "organization", element: <OrganizationPage /> },
      { path: "holidays", element: <HolidaysPage /> },
      { path: "users", element: <UsersPage /> },
    ],
  },
]);
