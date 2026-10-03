import type { AttendanceInput } from "@/bindings/AttendanceInput";
import type { AttendanceRecord } from "@/bindings/AttendanceRecord";
import type { DateRange } from "@/bindings/DateRange";
import type { DayRow } from "@/bindings/DayRow";
import type { KioskPunch } from "@/bindings/KioskPunch";
import type { ReviewItem } from "@/bindings/ReviewItem";
import { call } from "@/lib/ipc";

export const clockIn = (employeeNo: string, pin: string) =>
  call<KioskPunch>("kiosk_clock_in", { employeeNo, pin });
export const clockOut = (employeeNo: string, pin: string) =>
  call<KioskPunch>("kiosk_clock_out", { employeeNo, pin });
export const myAttendance = (range: DateRange) =>
  call<AttendanceRecord[]>("attendance_mine", { range });
export const attendanceDay = (date: string) => call<DayRow[]>("attendance_day", { date });
export const reviewQueue = () => call<ReviewItem[]>("attendance_review_queue");
export const attendanceForEmployee = (id: number, range: DateRange) =>
  call<AttendanceRecord[]>("attendance_for_employee", { id, range });
export const saveAttendance = (input: AttendanceInput) =>
  call<AttendanceRecord>("attendance_save", { input });
export const markReviewed = (id: number) =>
  call<AttendanceRecord>("attendance_mark_reviewed", { id });
