import type { BalanceAdjustment } from "@/bindings/BalanceAdjustment";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { LeaveType } from "@/bindings/LeaveType";
import type { LeaveTypeInput } from "@/bindings/LeaveTypeInput";
import { call } from "@/lib/ipc";

export const listLeaveTypes = () => call<LeaveType[]>("leave_types_list");
export const createLeaveType = (input: LeaveTypeInput) =>
  call<LeaveType>("leave_type_create", { input });
export const updateLeaveType = (id: number, input: LeaveTypeInput) =>
  call<LeaveType>("leave_type_update", { id, input });
export const setLeaveTypeActive = (id: number, active: boolean) =>
  call<void>("leave_type_set_active", { id, active });

export const myBalances = (year: number) => call<LeaveBalance[]>("leave_my_balances", { year });
export const balancesFor = (employeeId: number, year: number) =>
  call<LeaveBalance[]>("leave_balances_for", { employeeId, year });
export const grantLeave = (year: number) => call<number>("leave_balance_grant", { year });
export const adjustBalance = (id: number, input: BalanceAdjustment) =>
  call<LeaveBalance>("leave_balance_adjust", { id, input });
