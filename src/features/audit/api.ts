import type { AuditFilters } from "@/bindings/AuditFilters";
import type { AuditPage } from "@/bindings/AuditPage";
import type { AuditQuery } from "@/bindings/AuditQuery";
import { call } from "@/lib/ipc";

export const listActivity = (query: AuditQuery) => call<AuditPage>("audit_list", { query });
export const activityFilters = () => call<AuditFilters>("audit_filters");
/** Saves the filtered log as CSV. Resolves to the path, or null if the user cancelled. */
export const exportActivity = (query: AuditQuery) =>
  call<string | null>("audit_export_csv", { query });
