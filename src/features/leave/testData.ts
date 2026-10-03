import type { LeaveBalance } from "@/bindings/LeaveBalance";
import type { LeaveRequest } from "@/bindings/LeaveRequest";
import type { LeaveType } from "@/bindings/LeaveType";
import type { Me } from "@/bindings/Me";

export const hr: Me = {
  userId: 2,
  username: "maria",
  role: "HR",
  employeeId: 3,
  mustChangePassword: false,
  theme: "SYSTEM",
};

export const balance = (over: Partial<LeaveBalance>): LeaveBalance => ({
  id: 1,
  employeeId: 7,
  leaveTypeId: 2,
  leaveTypeCode: "VL",
  leaveTypeName: "Vacation Leave",
  year: 2026,
  entitledHalfdays: 10,
  usedHalfdays: 3,
  ...over,
});

export const leaveType = (over: Partial<LeaveType>): LeaveType => ({
  id: 1,
  code: "SIL",
  name: "Service Incentive Leave",
  isPaid: true,
  defaultHalfdaysPerYear: 10,
  minServiceMonths: 12,
  isActive: true,
  ...over,
});

/** A leave request for tests, with any field overridden. */
export const leaveRequest = (over: Partial<LeaveRequest>): LeaveRequest => ({
  id: 1,
  employeeId: 7,
  employeeNo: "EMP-0007",
  employeeName: "Dela Cruz, Juan",
  leaveTypeId: 2,
  leaveTypeCode: "VL",
  leaveTypeName: "Vacation Leave",
  isPaid: true,
  startDate: "2026-10-12",
  endDate: "2026-10-16",
  halfDay: false,
  halfdays: 8,
  reason: "Family trip",
  status: "PENDING",
  filedByUsername: "juan",
  decidedByUsername: null,
  decidedAt: null,
  decisionNote: null,
  locked: false,
  ...over,
});
