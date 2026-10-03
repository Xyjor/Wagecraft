import { useCallback, useEffect, useState, type FormEvent } from "react";
import type { Department } from "@/bindings/Department";
import type { Position } from "@/bindings/Position";
import { Field, FormAlert, SelectField } from "@/components/form";
import { ActiveBadge, primaryButton, quietButton } from "@/components/ui";
import { useSession } from "@/features/auth/session";
import { formValues, serverErrors } from "@/lib/formData";
import type { AppError } from "@/lib/ipc";
import { formatPesos } from "@/lib/money";
import {
  createDepartment,
  createPosition,
  listDepartments,
  listPositions,
  setDepartmentActive,
  setPositionActive,
  updateDepartment,
  updatePosition,
} from "./api";
import { checkDepartment, checkPosition, POSITION_FIELD } from "./validation";

/** Which form is open: none, a new record, or editing one. */
type Editing<T> = null | "new" | T;

export function OrganizationPage() {
  const { me } = useSession();
  if (me.role === "STAFF") {
    return (
      <p className="text-zinc-600 dark:text-zinc-400">
        Only Admin and HR can manage the organization.
      </p>
    );
  }
  return <Organization />;
}

function Organization() {
  const [departments, setDepartments] = useState<Department[] | null>(null);
  const [positions, setPositions] = useState<Position[] | null>(null);
  const [alert, setAlert] = useState<string>();
  const [deptForm, setDeptForm] = useState<Editing<Department>>(null);
  const [posForm, setPosForm] = useState<Editing<Position>>(null);

  const reload = useCallback(async () => {
    const [d, p] = await Promise.all([listDepartments(), listPositions()]);
    setDepartments(d);
    setPositions(p);
  }, []);

  useEffect(() => {
    Promise.all([listDepartments(), listPositions()])
      .then(([d, p]) => {
        setDepartments(d);
        setPositions(p);
      })
      .catch((e: AppError) => setAlert(e.message));
  }, []);

  async function toggle(change: Promise<void>) {
    setAlert(undefined);
    try {
      await change;
      await reload();
    } catch (e) {
      setAlert((e as AppError).message);
    }
  }

  const activeDepartments = departments?.filter((d) => d.isActive) ?? [];

  return (
    <div className="space-y-10">
      <h1 className="text-2xl font-semibold tracking-tight">Organization</h1>
      <FormAlert message={alert} />

      <section className="space-y-4" aria-labelledby="departments-heading">
        <div className="flex items-center justify-between">
          <h2 id="departments-heading" className="text-lg font-semibold">
            Departments
          </h2>
          {deptForm === null && (
            <button type="button" className={primaryButton} onClick={() => setDeptForm("new")}>
              Add department
            </button>
          )}
        </div>
        {deptForm !== null && (
          <DepartmentForm
            key={deptForm === "new" ? "new" : deptForm.id}
            editing={deptForm === "new" ? null : deptForm}
            onCancel={() => setDeptForm(null)}
            onSaved={async () => {
              setDeptForm(null);
              await reload();
            }}
          />
        )}
        {departments && (
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <th className="py-2 font-medium">Code</th>
                <th className="py-2 font-medium">Name</th>
                <th className="py-2 font-medium">Description</th>
                <th className="py-2 font-medium">Status</th>
                <th className="py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {departments.length === 0 && <EmptyRow>No departments yet.</EmptyRow>}
              {departments.map((d) => (
                <tr key={d.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2 font-medium tracking-wide">{d.code}</td>
                  <td className="py-2">{d.name}</td>
                  <td className="py-2 text-zinc-600 dark:text-zinc-400">{d.description}</td>
                  <td className="py-2">
                    <ActiveBadge active={d.isActive} />
                  </td>
                  <td className="py-2">
                    <RowActions
                      name={d.name}
                      active={d.isActive}
                      onEdit={() => setDeptForm(d)}
                      onToggle={() => toggle(setDepartmentActive(d.id, !d.isActive))}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="space-y-4" aria-labelledby="positions-heading">
        <div className="flex items-center justify-between">
          <h2 id="positions-heading" className="text-lg font-semibold">
            Positions
          </h2>
          {posForm === null && (
            <button type="button" className={primaryButton} onClick={() => setPosForm("new")}>
              Add position
            </button>
          )}
        </div>
        {posForm !== null && (
          <PositionForm
            key={posForm === "new" ? "new" : posForm.id}
            editing={posForm === "new" ? null : posForm}
            departments={activeDepartments}
            onCancel={() => setPosForm(null)}
            onSaved={async () => {
              setPosForm(null);
              await reload();
            }}
          />
        )}
        {positions && (
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-zinc-800">
              <tr>
                <th className="py-2 font-medium">Title</th>
                <th className="py-2 font-medium">Department</th>
                <th className="py-2 font-medium">Monthly salary range</th>
                <th className="py-2 font-medium">Status</th>
                <th className="py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {positions.length === 0 && <EmptyRow>No positions yet.</EmptyRow>}
              {positions.map((p) => (
                <tr key={p.id} className="border-b border-zinc-100 dark:border-zinc-900">
                  <td className="py-2 font-medium">{p.title}</td>
                  <td className="py-2">{p.departmentName}</td>
                  <td className="py-2 tabular-nums">
                    {salaryRange(p.minRateCents, p.maxRateCents)}
                  </td>
                  <td className="py-2">
                    <ActiveBadge active={p.isActive} />
                  </td>
                  <td className="py-2">
                    <RowActions
                      name={p.title}
                      active={p.isActive}
                      onEdit={() => setPosForm(p)}
                      onToggle={() => toggle(setPositionActive(p.id, !p.isActive))}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

function RowActions(props: {
  name: string;
  active: boolean;
  onEdit: () => void;
  onToggle: () => void;
}) {
  const verb = props.active ? "Deactivate" : "Activate";
  return (
    <div className="flex justify-end gap-1">
      <button
        type="button"
        className={quietButton}
        aria-label={`Edit ${props.name}`}
        onClick={props.onEdit}
      >
        Edit
      </button>
      <button
        type="button"
        className={quietButton}
        aria-label={`${verb} ${props.name}`}
        onClick={props.onToggle}
      >
        {verb}
      </button>
    </div>
  );
}

function DepartmentForm({
  editing,
  onSaved,
  onCancel,
}: {
  editing: Department | null;
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const checked = checkDepartment(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      if (editing) await updateDepartment(editing.id, checked.value);
      else await createDepartment(checked.value);
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(s.fields);
      setAlert(s.alert);
      setBusy(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      noValidate
      aria-label={editing ? `Edit ${editing.name}` : "Add department"}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <Field
        name="code"
        label="Code"
        autoComplete="off"
        autoFocus
        defaultValue={editing?.code}
        error={errors.code}
      />
      <Field name="name" label="Name" defaultValue={editing?.name} error={errors.name} />
      <Field
        name="description"
        label="Description"
        defaultValue={editing?.description ?? undefined}
        error={errors.description}
      />
      <FormButtons busy={busy} save="Save department" onCancel={onCancel} />
    </form>
  );
}

function PositionForm({
  editing,
  departments,
  onSaved,
  onCancel,
}: {
  editing: Position | null;
  departments: Department[];
  onSaved: () => Promise<void>;
  onCancel: () => void;
}) {
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [alert, setAlert] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const checked = checkPosition(formValues(e.currentTarget));
    if (!checked.ok) {
      setErrors(checked.errors);
      return;
    }
    setBusy(true);
    try {
      if (editing) await updatePosition(editing.id, checked.value);
      else await createPosition(checked.value);
      await onSaved();
    } catch (err) {
      const s = serverErrors(err as AppError);
      setErrors(
        Object.fromEntries(Object.entries(s.fields).map(([k, v]) => [POSITION_FIELD[k] ?? k, v])),
      );
      setAlert(s.alert);
      setBusy(false);
    }
  }

  const options = [
    { value: "", label: "Pick a department" },
    ...departments.map((d) => ({ value: String(d.id), label: d.name })),
  ];
  const plain = (cents: number | null | undefined) =>
    cents == null ? undefined : formatPesos(cents).replace("₱", "");

  return (
    <form
      onSubmit={submit}
      noValidate
      aria-label={editing ? `Edit ${editing.title}` : "Add position"}
      className="grid max-w-md gap-4 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800"
    >
      <FormAlert message={alert} />
      <SelectField
        name="departmentId"
        label="Department"
        options={options}
        defaultValue={editing ? String(editing.departmentId) : ""}
        error={errors.departmentId}
      />
      <Field name="title" label="Title" defaultValue={editing?.title} error={errors.title} />
      <Field
        name="minRate"
        label="Minimum monthly rate"
        defaultValue={plain(editing?.minRateCents)}
        error={errors.minRate}
      />
      <Field
        name="maxRate"
        label="Maximum monthly rate"
        defaultValue={plain(editing?.maxRateCents)}
        error={errors.maxRate}
      />
      <p className="text-sm text-zinc-600 dark:text-zinc-400">
        Optional. A pay rate outside this range shows a warning on the employee.
      </p>
      <FormButtons busy={busy} save="Save position" onCancel={onCancel} />
    </form>
  );
}

function FormButtons({
  busy,
  save,
  onCancel,
}: {
  busy: boolean;
  save: string;
  onCancel: () => void;
}) {
  return (
    <div className="flex gap-2">
      <button type="submit" disabled={busy} className={primaryButton}>
        {save}
      </button>
      <button type="button" onClick={onCancel} className={quietButton}>
        Cancel
      </button>
    </div>
  );
}

function EmptyRow({ children }: { children: string }) {
  return (
    <tr>
      <td colSpan={5} className="py-3 text-zinc-500">
        {children}
      </td>
    </tr>
  );
}

function salaryRange(min: number | null, max: number | null): string {
  if (min == null && max == null) return "No range";
  if (max == null) return `From ${formatPesos(min!)}`;
  if (min == null) return `Up to ${formatPesos(max)}`;
  return `${formatPesos(min)} – ${formatPesos(max)}`;
}
