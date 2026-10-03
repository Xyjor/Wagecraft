import type { Compensation } from "@/bindings/Compensation";
import type { CompensationInput } from "@/bindings/CompensationInput";
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
export const compensationHistory = (id: number) =>
  call<Compensation[]>("employee_compensation_history", { id });
export const addCompensation = (id: number, input: CompensationInput) =>
  call<Compensation>("employee_add_compensation", { id, input });
