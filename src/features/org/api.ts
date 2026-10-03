import type { Department } from "@/bindings/Department";
import type { DepartmentInput } from "@/bindings/DepartmentInput";
import type { Position } from "@/bindings/Position";
import type { PositionInput } from "@/bindings/PositionInput";
import { call } from "@/lib/ipc";

export const listDepartments = () => call<Department[]>("department_list");
export const createDepartment = (input: DepartmentInput) =>
  call<Department>("department_create", { input });
export const updateDepartment = (id: number, input: DepartmentInput) =>
  call<Department>("department_update", { id, input });
export const setDepartmentActive = (id: number, active: boolean) =>
  call<void>("department_set_active", { id, active });

export const listPositions = () => call<Position[]>("position_list");
export const createPosition = (input: PositionInput) =>
  call<Position>("position_create", { input });
export const updatePosition = (id: number, input: PositionInput) =>
  call<Position>("position_update", { id, input });
export const setPositionActive = (id: number, active: boolean) =>
  call<void>("position_set_active", { id, active });
