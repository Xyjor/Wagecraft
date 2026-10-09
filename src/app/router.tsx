import { createBrowserRouter } from "react-router";
import { AppShell } from "./AppShell";
import { AttendancePage } from "@/features/attendance/AttendancePage";
import { MyAttendancePage } from "@/features/attendance/MyAttendancePage";
import { BackupsPage } from "@/features/backups/BackupsPage";
import { EmployeeFormPage } from "@/features/employees/EmployeeFormPage";
import { EmployeeProfilePage } from "@/features/employees/EmployeeProfilePage";
import { EmployeesPage } from "@/features/employees/EmployeesPage";
import { MyProfilePage } from "@/features/employees/MyProfilePage";
import { HomePage } from "@/features/home/HomePage";
import { LeavePage } from "@/features/leave/LeavePage";
import { LeaveCalendarPage } from "@/features/leave/LeaveCalendarPage";
import { MyLeavePage } from "@/features/leave/MyLeavePage";
import { HolidaysPage } from "@/features/org/HolidaysPage";
import { OrganizationPage } from "@/features/org/OrganizationPage";
import { OvertimePage } from "@/features/overtime/OvertimePage";
import { MyPayslipsPage } from "@/features/payroll/MyPayslipsPage";
import { PayrollPage } from "@/features/payroll/PayrollPage";
import { RegisterPage } from "@/features/payroll/RegisterPage";
import { RulePacksPage } from "@/features/payroll/RulePacksPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { UsersPage } from "@/features/users/UsersPage";
import { ActivityPage } from "@/features/audit/ActivityPage";

export const router = createBrowserRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "me", element: <MyProfilePage /> },
      { path: "my-attendance", element: <MyAttendancePage /> },
      { path: "my-leave", element: <MyLeavePage /> },
      { path: "my-payslips", element: <MyPayslipsPage /> },
      { path: "attendance", element: <AttendancePage /> },
      { path: "overtime", element: <OvertimePage /> },
      { path: "leave", element: <LeavePage /> },
      { path: "leave-calendar", element: <LeaveCalendarPage /> },
      { path: "employees", element: <EmployeesPage /> },
      { path: "employees/new", element: <EmployeeFormPage /> },
      { path: "employees/:id", element: <EmployeeProfilePage /> },
      { path: "employees/:id/edit", element: <EmployeeFormPage /> },
      { path: "payroll", element: <PayrollPage /> },
      { path: "payroll/rules", element: <RulePacksPage /> },
      { path: "payroll/:id", element: <RegisterPage /> },
      { path: "organization", element: <OrganizationPage /> },
      { path: "holidays", element: <HolidaysPage /> },
      { path: "users", element: <UsersPage /> },
      { path: "backups", element: <BackupsPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "activity", element: <ActivityPage /> },
    ],
  },
]);
