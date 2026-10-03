import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import type { DateRange } from "@/bindings/DateRange";
import type { KioskPunch } from "@/bindings/KioskPunch";
import { call } from "@/lib/ipc";

export const clockIn = (employeeNo: string, pin: string) =>
  call<KioskPunch>("kiosk_clock_in", { employeeNo, pin });
export const clockOut = (employeeNo: string, pin: string) =>
  call<KioskPunch>("kiosk_clock_out", { employeeNo, pin });
export const myAttendance = (range: DateRange) =>
  call<AttendanceRecord[]>("attendance_mine", { range });
