import type { MyPayslip } from "@/bindings/MyPayslip";
import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { PayrollPeriodInput } from "@/bindings/PayrollPeriodInput";
import type { PayrollRegister } from "@/bindings/PayrollRegister";
import type { PayslipDetail } from "@/bindings/PayslipDetail";
import type { PeriodChecks } from "@/bindings/PeriodChecks";
import type { RegisterReport } from "@/bindings/RegisterReport";
import type { RulePackDetail } from "@/bindings/RulePackDetail";
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
export const getRulePack = (id: number) => call<RulePackDetail>("rule_pack_get", { id });
/** Asks where to save, then writes the file. Null when the person cancels. */
export const savePdf = (fileName: string, bytes: Uint8Array) =>
  call<string | null>("export_save_pdf", { fileName, bytes: Array.from(bytes) });
export const getRegisterReport = (periodId: number) =>
  call<RegisterReport>("report_payroll_register", { periodId });
/** Asks where to save the register CSV, then writes it. Null when the person cancels. */
export const saveRegisterCsv = (periodId: number) =>
  call<string | null>("report_payroll_register_csv", { periodId });
