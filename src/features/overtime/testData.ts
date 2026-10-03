import type { OvertimeRequest } from "@/bindings/OvertimeRequest";

/** An overtime request for tests, with any field overridden. */
export const request = (over: Partial<OvertimeRequest>): OvertimeRequest => ({
  id: 1,
  employeeId: 7,
  employeeNo: "EMP-0007",
  employeeName: "Dela Cruz, Juan",
  workDate: "2026-10-07",
  startAt: "2026-10-07T17:00:00",
  endAt: "2026-10-07T20:00:00",
  minutes: 180,
  reason: "Month-end stock count",
  status: "PENDING",
  filedByUsername: "juan",
  decidedByUsername: null,
  decidedAt: null,
  decisionNote: null,
  locked: false,
  ...over,
});
