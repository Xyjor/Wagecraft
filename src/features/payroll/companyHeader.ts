import type { CompanyHeader } from "@/bindings/CompanyHeader";
import { formatId } from "@/lib/govIds";

/** The lines under the company name: the address and the TIN, when Settings has them. */
export function companyLines(company: CompanyHeader): string[] {
  const lines = [];
  if (company.address) lines.push(company.address);
  if (company.tin) lines.push(`TIN ${formatId("tin", company.tin)}`);
  return lines;
}
