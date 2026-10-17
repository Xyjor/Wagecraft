import { getRegisterReport, savePdf, saveRegisterCsv } from "./api";
import { registerFileName, renderRegisterPdf } from "./registerPdf";
import { SaveButton } from "./SaveButton";

/** The register as a CSV for spreadsheets, or a PDF to print and file (plan §6.8). */
export function RegisterDownloads({ periodId }: { periodId: number }) {
  return (
    <div className="flex flex-wrap items-start gap-2">
      <SaveButton
        label="Download CSV"
        busyLabel="Saving CSV…"
        variant="secondary"
        save={() => saveRegisterCsv(periodId)}
      />
      <SaveButton
        label="Download PDF"
        busyLabel="Making PDF…"
        variant="secondary"
        save={async () => {
          const report = await getRegisterReport(periodId);
          return savePdf(registerFileName(report), await renderRegisterPdf(report));
        }}
      />
    </div>
  );
}
