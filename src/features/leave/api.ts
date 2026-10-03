import type { BalanceAdjustment } from "@/bindings/BalanceAdjustment";
import type { DateRange } from "@/bindings/DateRange";
import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import type { LeaveRequestInput } from "@/bindings/LeaveRequestInput";
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

export const fileLeave = (input: LeaveRequestInput) =>
  call<LeaveRequest>("leave_request_create", { input });
export const cancelLeave = (id: number) => call<LeaveRequest>("leave_request_cancel", { id });
export const myLeaveRequests = (year: number) =>
  call<LeaveRequest[]>("leave_my_requests", { year });
export const listLeaveRequests = (status: LeaveRequest["status"] | null, range: DateRange) =>
  call<LeaveRequest[]>("leave_request_list", { status, range });
export const pendingLeave = () => call<LeaveRequest[]>("leave_request_pending");
export const decideLeave = (id: number, approve: boolean, note: string | null) =>
  call<LeaveRequest>("leave_request_decide", { id, approve, note });
