import type { MyPayslip } from "@/bindings/MyPayslip";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { PayrollPeriodInput } from "@/bindings/PayrollPeriodInput";
import type { PayrollRegister } from "@/bindings/PayrollRegister";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PeriodChecks } from "@/bindings/PeriodChecks";
import type { RulePackSummary } from "@/bindings/RulePackSummary";
import { call } from "@/lib/ipc";

export const listPeriods = () => call<PayrollPeriod[]>("payroll_period_list");
export const listRulePacks = () => call<RulePackSummary[]>("rule_pack_list");
export const periodChecks = (periodStart: string) =>
  call<PeriodChecks>("payroll_period_checks", { periodStart });
export const createPeriod = (input: PayrollPeriodInput) =>
  call<PayrollPeriod>("payroll_period_create", { input });
export const deletePeriod = (id: number) => call<void>("payroll_period_delete", { id });
export const getRegister = (periodId: number) =>
  call<PayrollRegister>("payroll_register", { periodId });
export const computePayroll = (periodId: number) =>
  call<PayrollRegister>("payroll_compute", { periodId });
export const approvePayroll = (periodId: number) =>
  call<PayrollRegister>("payroll_approve", { periodId });
export const sendBackPayroll = (periodId: number, reason: string) =>
  call<PayrollRegister>("payroll_send_back", { periodId, reason });
export const postPayroll = (periodId: number) =>
  call<PayrollRegister>("payroll_post", { periodId });
export const getPayslip = (id: number) => call<PayslipDetail>("payslip_get", { id });
export const myPayslips = () => call<MyPayslip[]>("payslip_my_list");
export const getMyPayslip = (id: number) => call<PayslipDetail>("payslip_my_get", { id });
