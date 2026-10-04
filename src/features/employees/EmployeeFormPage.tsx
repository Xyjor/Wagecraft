import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { Link, useNavigate, useParams } from "react-router";
import type { Department } from "@/bindings/Department";
import type { Employee } from "@/bindings/Employee";
import type { Position } from "@/bindings/Position";
import type { WorkSchedule } from "@/bindings/WorkSchedule";
import { Field, FormAlert, SelectField } from "@/components/form";
import { primaryButton, quietButton } from "@/components/ui";
import { listDepartments, listPositions, listSchedules } from "@/features/org/api";
import { scheduleHours, workDaysLabel } from "@/features/org/format";
import { formValues, serverErrors } from "@/lib/formData";
import { formatId } from "@/lib/govIds";
import type { AppError } from "@/lib/ipc";
import { createEmployee, getEmployee, updateEmployee } from "./api";
import { CIVIL_STATUS_LABELS, fullName, SEX_LABELS, STATUS_LABELS } from "./format";
import { checkEmployee } from "./validation";

type Loaded = {
  departments: Department[];
  positions: Position[];
  schedules: WorkSchedule[];
  employee: Employee | null;
};

const options = (labels: Record<string, string>, blank?: string) => [
  ...(blank === undefined ? [] : [{ value: "", label: blank }]),
  ...Object.entries(labels).map(([value, label]) => ({ value, label })),
];

/** Add employee (`/employees/new`) or edit one (`/employees/:id/edit`). */
export function EmployeeFormPage() {
  const { id } = useParams();
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [alert, setAlert] = useState<string>();

  useEffect(() => {
    Promise.all([
      listDepartments(),
      listPositions(),
      listSchedules(),
      id ? getEmployee(Number(id)) : null,
    ])
      .then(([departments, positions, schedules, employee]) =>
        setLoaded({ departments, positions, schedules, employee }),
      )
      .catch((e: AppError) => setAlert(e.message));
  }, [id]);

  if (!loaded) return <FormAlert message={alert} />;
  return <EmployeeForm {...loaded} />;
}

function EmployeeForm({ departments, positions, schedules, employee }: Loaded) {
  const navigate = useNavigate();
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [departmentId, setDepartmentId] = useState(String(employee?.departmentId ?? ""));

  // New assignments must be active; an employee keeps an inactive one they already have.
  const departmentOptions = departments.filter(
    (d) => d.isActive || d.id === employee?.departmentId,
  );
  const positionOptions = positions.filter(
    (p) => String(p.departmentId) === departmentId && (p.isActive || p.id === employee?.positionId),
  );
  const scheduleOptions = schedules.filter((s) => s.isActive);
  const keepsPosition = departmentId === String(employee?.departmentId ?? "");

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const checked = checkEmployee(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      setAlert("Some fields need attention.");
      return;
    }
    setBusy(true);
    try {
      const saved = employee
        ? await updateEmployee(employee.id, checked.value)
        : await createEmployee(checked.value);
      navigate(`/employees/${saved.id}`);
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert ?? "Some fields need attention.");
      setBusy(false);
    }
  }

  const e = employee;
  const title = e ? `Edit ${fullName(e)}` : "Add employee";

  return (
    <form onSubmit={submit} noValidate className="max-w-3xl space-y-8">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <FormAlert message={alert} />

      <Section title="Name and number">
        <Field
          name="employeeNo"
          label="Employee no."
          autoFocus
          defaultValue={e?.employeeNo}
          error={errors.employeeNo}
        />
        <div />
        <Field
          name="firstName"
          label="First name"
          defaultValue={e?.firstName}
          error={errors.firstName}
        />
        <Field
          name="middleName"
          label="Middle name"
          defaultValue={e?.middleName ?? undefined}
          error={errors.middleName}
        />
        <Field
          name="lastName"
          label="Last name"
          defaultValue={e?.lastName}
          error={errors.lastName}
        />
        <Field
          name="suffix"
          label="Suffix"
          defaultValue={e?.suffix ?? undefined}
          error={errors.suffix}
        />
      </Section>

      <Section title="Personal">
        <Field
          name="birthDate"
          label="Birth date"
          type="date"
          defaultValue={e?.birthDate ?? undefined}
          error={errors.birthDate}
        />
        <SelectField
          name="sex"
          label="Sex"
          options={options(SEX_LABELS, "Not set")}
          defaultValue={e?.sex ?? ""}
          error={errors.sex}
        />
        <SelectField
          name="civilStatus"
          label="Civil status"
          options={options(CIVIL_STATUS_LABELS, "Not set")}
          defaultValue={e?.civilStatus ?? ""}
          error={errors.civilStatus}
        />
        <Field
          name="email"
          label="Email"
          type="email"
          defaultValue={e?.email ?? undefined}
          error={errors.email}
        />
        <Field
          name="mobile"
          label="Mobile"
          type="tel"
          defaultValue={e?.mobile ?? undefined}
          error={errors.mobile}
        />
        <Field
          name="address"
          label="Address"
          defaultValue={e?.address ?? undefined}
          error={errors.address}
        />
      </Section>

      <Section title="Employment">
        <SelectField
          name="employmentStatus"
          label="Status"
          options={options(STATUS_LABELS)}
          defaultValue={e?.employmentStatus ?? "PROBATIONARY"}
          error={errors.employmentStatus}
        />
        <Field
          name="hireDate"
          label="Hire date"
          type="date"
          defaultValue={e?.hireDate}
          error={errors.hireDate}
        />
        <Field
          name="regularizationDate"
          label="Regularization date"
          type="date"
          defaultValue={e?.regularizationDate ?? undefined}
          error={errors.regularizationDate}
        />
        <Field
          name="separationDate"
          label="Separation date"
          type="date"
          defaultValue={e?.separationDate ?? undefined}
          error={errors.separationDate}
        />
        <SelectField
          name="departmentId"
          label="Department"
          options={[
            { value: "", label: "No department" },
            ...departmentOptions.map((d) => ({ value: String(d.id), label: d.name })),
          ]}
          defaultValue={departmentId}
          onChange={setDepartmentId}
          error={errors.departmentId}
        />
        <SelectField
          key={departmentId}
          name="positionId"
          label="Position"
          options={[
            { value: "", label: "No position" },
            ...positionOptions.map((p) => ({ value: String(p.id), label: p.title })),
          ]}
          defaultValue={keepsPosition ? String(e?.positionId ?? "") : ""}
          error={errors.positionId}
        />
        {e ? (
          <CurrentSchedule
            id={e.scheduleId}
            schedule={schedules.find((s) => s.id === e.scheduleId)}
          />
        ) : (
          <SelectField
            name="scheduleId"
            label="Work schedule"
            options={[
              { value: "", label: "No schedule" },
              ...scheduleOptions.map((s) => ({
                value: String(s.id),
                label: `${s.name} · ${scheduleHours(s)}, ${workDaysLabel(s.workDays)}`,
              })),
            ]}
            defaultValue=""
            error={errors.scheduleId}
          />
        )}
      </Section>

      <Section title="Government IDs and bank">
        <Field
          name="tin"
          label="TIN"
          defaultValue={formatId("tin", e?.tin) || undefined}
          error={errors.tin}
        />
        <Field
          name="sssNo"
          label="SSS no."
          defaultValue={formatId("sss", e?.sssNo) || undefined}
          error={errors.sssNo}
        />
        <Field
          name="philhealthNo"
          label="PhilHealth no."
          defaultValue={formatId("philhealth", e?.philhealthNo) || undefined}
          error={errors.philhealthNo}
        />
        <Field
          name="pagibigNo"
          label="Pag-IBIG MID"
          defaultValue={formatId("pagibig", e?.pagibigNo) || undefined}
          error={errors.pagibigNo}
        />
        <Field
          name="bankName"
          label="Bank"
          defaultValue={e?.bankName ?? undefined}
          error={errors.bankName}
        />
        <Field
          name="bankAccountNo"
          label="Account no."
          defaultValue={e?.bankAccountNo ?? undefined}
          error={errors.bankAccountNo}
        />
      </Section>

      <div className="flex gap-2">
        <button type="submit" disabled={busy} className={primaryButton}>
          Save employee
        </button>
        <Link to={e ? `/employees/${e.id}` : "/employees"} className={quietButton}>
          Cancel
        </Link>
      </div>
    </form>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="space-y-3">
      <legend className="text-lg font-semibold">{title}</legend>
      <div className="grid gap-4 sm:grid-cols-2">{children}</div>
    </fieldset>
  );
}

/** The schedule is only shown here; a move needs a start date, so it's on the Schedule tab. */
function CurrentSchedule({ id, schedule }: { id: number | null; schedule?: WorkSchedule }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium">Work schedule</span>
      <span className="text-sm">
        {schedule
          ? `${schedule.name} · ${scheduleHours(schedule)}, ${workDaysLabel(schedule.workDays)}`
          : "No schedule"}
      </span>
      <span className="text-xs text-zinc-500">
        To change it from a date, use the Schedule tab on the profile.
      </span>
      <input type="hidden" name="scheduleId" value={id === null ? "" : String(id)} />
    </div>
  );
}
