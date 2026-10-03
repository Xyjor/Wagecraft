import type { DateRange } from "@/bindings/DateRange";
import type { OvertimeInput } from "@/bindings/OvertimeInput";
import type { OvertimeRequest } from "@/bindings/OvertimeRequest";
import { call } from "@/lib/ipc";

export const fileOvertime = (input: OvertimeInput) =>
  call<OvertimeRequest>("overtime_request_create", { input });
export const cancelOvertime = (id: number) =>
  call<OvertimeRequest>("overtime_request_cancel", { id });
export const myOvertime = (range: DateRange) => call<OvertimeRequest[]>("overtime_mine", { range });
export const listOvertime = (status: OvertimeRequest["status"] | null, range: DateRange) =>
  call<OvertimeRequest[]>("overtime_list", { status, range });
export const pendingOvertime = () => call<OvertimeRequest[]>("overtime_pending");
export const decideOvertime = (id: number, approve: boolean, note: string | null) =>
  call<OvertimeRequest>("overtime_decide", { id, approve, note });
