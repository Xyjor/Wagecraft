import type { Employee } from "@/bindings/Employee";
import type { EmployeeInput } from "@/bindings/EmployeeInput";
import type { EmployeePage } from "@/bindings/EmployeePage";
import type { EmployeeQuery } from "@/bindings/EmployeeQuery";
import { call } from "@/lib/ipc";

export const listEmployees = (query: EmployeeQuery) =>
  call<EmployeePage>("employee_list", { query });
export const getEmployee = (id: number) => call<Employee>("employee_get", { id });
export const createEmployee = (input: EmployeeInput) =>
  call<Employee>("employee_create", { input });
export const updateEmployee = (id: number, input: EmployeeInput) =>
  call<Employee>("employee_update", { id, input });
export const archiveEmployee = (id: number, archived: boolean) =>
  call<void>("employee_archive", { id, archived });
