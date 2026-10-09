/** How audit entries read on screen. */

/** The part of an action before the dot: `payroll.post` belongs to the `payroll` area. */
export function areaOf(action: string): string {
  const dot = action.indexOf(".");
  return dot === -1 ? action : action.slice(0, dot);
}

/** Actions grouped by area, in the order the areas first appear. */
export function groupByArea(actions: string[]): { area: string; actions: string[] }[] {
  const groups: { area: string; actions: string[] }[] = [];
  for (const action of actions) {
    const area = areaOf(action);
    const group = groups.find((g) => g.area === area);
    if (group) group.actions.push(action);
    else groups.push({ area, actions: [action] });
  }
  return groups;
}

/** Where a record's own screen is, for the record types that have one. */
export function recordLink(entityType: string | null, entityId: number | null): string | null {
  if (entityId === null) return null;
  if (entityType === "employee") return `/employees/${entityId}`;
  if (entityType === "payroll_period") return `/payroll/${entityId}`;
  return null;
}

export function recordLabel(entityType: string | null, entityId: number | null): string {
  if (!entityType) return "";
  return entityId === null ? entityType : `${entityType} #${entityId}`;
}
