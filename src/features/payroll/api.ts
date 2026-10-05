import type { PayrollPeriod } from "@/bindings/PayrollPeriod";
import type { PayrollPeriodInput } from "@/bindings/PayrollPeriodInput";
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
