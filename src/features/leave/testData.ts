import type { LeaveBalance } from "@/bindings/LeaveBalance";
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
