import { useEffect, useId, useState, type ReactNode } from "react";
import { Link } from "react-router";
import type { PremiumRow } from "@/bindings/PremiumRow";
import type { RulePackDetail } from "@/bindings/RulePackDetail";
import type { RulePackSummary } from "@/bindings/RulePackSummary";
import type { TaxBracketRow } from "@/bindings/TaxBracketRow";
import { FormAlert, SelectField } from "@/components/form";
import { quietButton } from "@/components/ui";
import { formatDate } from "@/lib/dates";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import { getRulePack, listRulePacks } from "./api";
import { percentText } from "./periods";

const DAY_TYPES: Record<string, string> = {
  ORDINARY: "Ordinary day",
  REST_DAY: "Rest day",
  SPECIAL: "Special non-working day",
  SPECIAL_REST_DAY: "Special day on a rest day",
  REGULAR: "Regular holiday",
  REGULAR_REST_DAY: "Regular holiday on a rest day",
  DOUBLE_REGULAR: "Double regular holiday",
  DOUBLE_REGULAR_REST_DAY: "Double holiday on a rest day",
};

/**
 * The rates payroll is computed with (`/payroll/rules`), read-only (plan §6.5). New
 * government rates arrive as a new pack in an app update, never as an edit here.
 */
export function RulePacksPage() {
  const [packs, setPacks] = useState<RulePackSummary[] | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [pack, setPack] = useState<RulePackDetail | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    let live = true;
    listRulePacks()
      .then((p) => {
        if (!live) return;
        setPacks(p);
        setSelected(p[0]?.id ?? null);
      })
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    if (selected === null) return;
    let live = true;
    getRulePack(selected)
      .then((p) => live && setPack(p))
      .catch((e: AppError) => live && setAlert(e.message));
    return () => {
      live = false;
    };
  }, [selected]);

  return (
    <div className="max-w-4xl space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Rule packs</h1>
        <Link to="/payroll" className={quietButton}>
          Back to payroll
        </Link>
      </div>
      <p className="max-w-prose text-sm text-zinc-600 dark:text-zinc-400">
        The government rates each payroll is computed with. They can&apos;t be edited here: new
        rates come as a new pack in an app update, so payslips already posted keep the rates they
        were made with.
      </p>
      <FormAlert message={alert} />
      {packs?.length === 0 && <p className="text-sm text-zinc-500">No rule packs are active.</p>}
      {packs && packs.length > 1 && (
        <div className="max-w-xs">
          <SelectField
            name="rulePack"
            label="Rule pack"
            options={packs.map((p) => ({ value: String(p.id), label: p.name }))}
            defaultValue={String(packs[0].id)}
            onChange={(v) => setSelected(Number(v))}
          />
        </div>
      )}
      {pack && <PackView pack={pack} />}
    </div>
  );
}

function PackView({ pack }: { pack: RulePackDetail }) {
  const { summary, settings } = pack;
  const { philhealth, pagibig } = settings;
  const effective = summary.effectiveTo
    ? `In effect from ${formatDate(summary.effectiveFrom)} to ${formatDate(summary.effectiveTo)}`
    : `In effect from ${formatDate(summary.effectiveFrom)}`;

  return (
    <section className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold">
          {summary.name} ({summary.code})
        </h2>
        <p className="text-sm text-zinc-500">{effective}</p>
      </div>

      <ul className="list-disc space-y-1 pl-5 text-sm">
        <li>
          Daily rate from a monthly rate: {settings.factorFiveDay} paid days a year for a 5-day
          week, {settings.factorSixDay} for a 6-day week
        </li>
        <li>
          SSS: employee {percentText(settings.sssEmployeeBp)}, employer{" "}
          {percentText(settings.sssEmployerBp)} of the Monthly Salary Credit
        </li>
        <li>
          PhilHealth: {percentText(philhealth.rateBp)} of basic pay, on{" "}
          {formatPesos(philhealth.floorCents)} to {formatPesos(philhealth.ceilingCents)} a month
        </li>
        <li>
          Pag-IBIG: employee {percentText(pagibig.employeeBp)} ({percentText(pagibig.lowRateBp)} on
          pay up to {formatPesos(pagibig.lowPayLimitCents)}), employer{" "}
          {percentText(pagibig.employerBp)}, on pay up to {formatPesos(pagibig.maxBaseCents)} a
          month
        </li>
      </ul>

      <Table
        caption="Premium pay"
        note="Share of the hourly rate."
        headers={["Day", "Work", "Overtime", "Night shift extra"]}
      >
        {pack.premiums.map((p: PremiumRow) => (
          <tr key={p.dayType}>
            <Th>{DAY_TYPES[p.dayType] ?? p.dayType}</Th>
            <Td>{percentText(p.workBp)}</Td>
            <Td>{percentText(p.otBp)}</Td>
            <Td>{percentText(p.nightDiffBp)}</Td>
          </tr>
        ))}
      </Table>

      <TaxTable frequency="SEMI_MONTHLY" caption="Withholding tax, semi-monthly" pack={pack} />
      <TaxTable frequency="MONTHLY" caption="Withholding tax, monthly" pack={pack} />

      <Table
        caption="SSS contributions"
        note="Monthly amounts. EC is the employer's Employees' Compensation share."
        headers={["Monthly pay", "Salary credit", "Employee", "Employer", "EC"]}
      >
        {pack.sssBrackets.map((b) => (
          <tr key={b.fromCents}>
            <Th>
              {b.toCents === null
                ? `${formatPesos(b.fromCents)} and up`
                : `${formatPesos(b.fromCents)} – ${formatPesos(b.toCents)}`}
            </Th>
            <Td>{formatPesos(b.mscCents)}</Td>
            <Td>{formatPesos(b.eeCents)}</Td>
            <Td>{formatPesos(b.erCents)}</Td>
            <Td>{formatPesos(b.ecCents)}</Td>
          </tr>
        ))}
      </Table>
    </section>
  );
}

function TaxTable({
  frequency,
  caption,
  pack,
}: {
  frequency: TaxBracketRow["frequency"];
  caption: string;
  pack: RulePackDetail;
}) {
  const rows = pack.taxBrackets.filter((t) => t.frequency === frequency);
  if (rows.length === 0) return null;
  return (
    <Table caption={caption} headers={["Taxable pay", "Tax"]}>
      {rows.map((t) => (
        <tr key={t.overCents}>
          <Th>
            {t.notOverCents === null
              ? `Over ${formatPesos(t.overCents)}`
              : `${formatPesos(t.overCents)} – ${formatPesos(t.notOverCents)}`}
          </Th>
          <Td>
            {t.rateBp === 0 && t.baseTaxCents === 0
              ? "None"
              : `${formatPesos(t.baseTaxCents)} + ${percentText(t.rateBp)} of the excess over ${formatPesos(t.overCents)}`}
          </Td>
        </tr>
      ))}
    </Table>
  );
}

function Table({
  caption,
  note,
  headers,
  children,
}: {
  caption: string;
  note?: string;
  headers: string[];
  children: ReactNode;
}) {
  const noteId = useId();
  return (
    <table className="w-full text-left text-sm" aria-describedby={note ? noteId : undefined}>
      <caption className="pb-2 text-left font-medium">
        {caption}
        {note && (
          <span id={noteId} className="block text-xs font-normal text-zinc-500" aria-hidden>
            {note}
          </span>
        )}
      </caption>
      <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
        <tr>
          {headers.map((h, i) => (
            <th key={h} className={`py-2 font-medium ${i > 0 ? "text-right" : ""}`}>
              {h}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>{children}</tbody>
    </table>
  );
}

function Th({ children }: { children: ReactNode }) {
  return (
    <th scope="row" className="py-1 font-normal">
      {children}
    </th>
  );
}

function Td({ children }: { children: ReactNode }) {
  return <td className="py-1 text-right tabular-nums">{children}</td>;
}
