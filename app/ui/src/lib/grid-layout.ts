export interface Layout {
  order: string[];
  frozen: string[];
  widths: Record<string, number>;
}

const KEY = 'nd-grid:';
const EMPTY: Layout = { order: [], frozen: [], widths: {} };

export function loadLayout(key: string | null): Layout {
  if (!key) return EMPTY;
  try {
    const raw = JSON.parse(localStorage.getItem(KEY + key) ?? 'null') as Partial<Layout> | null;
    return raw ? { order: raw.order ?? [], frozen: raw.frozen ?? [], widths: raw.widths ?? {} } : EMPTY;
  } catch {
    return EMPTY;
  }
}

export function saveLayout(key: string | null, layout: Layout) {
  if (!key) return;
  try {
    localStorage.setItem(KEY + key, JSON.stringify(layout));
  } catch {
    return;
  }
}

export function arrange(names: readonly string[], saved: Layout): { order: number[]; frozen: number } {
  const at = new Map<string, number>();
  names.forEach((n, i) => at.has(n) || at.set(n, i));
  const frozen = saved.frozen.map((n) => at.get(n)).filter((i): i is number => i !== undefined);
  const known = saved.order.map((n) => at.get(n)).filter((i): i is number => i !== undefined);
  const seen = new Set(frozen);
  const order = [...frozen];
  for (const i of [...known, ...names.keys()]) if (!seen.has(i)) (seen.add(i), order.push(i));
  return { order, frozen: frozen.length };
}

export function moveColumn(order: readonly number[], frozen: number, from: number, to: number): number[] {
  const lo = from < frozen ? 0 : frozen;
  const hi = from < frozen ? frozen : order.length;
  const target = Math.min(hi, Math.max(lo, to));
  const out = [...order];
  const [col] = out.splice(from, 1);
  if (col === undefined) return out;
  out.splice(target > from ? target - 1 : target, 0, col);
  return out;
}

export function toggleFrozen(order: readonly number[], frozen: number, p: number): { order: number[]; frozen: number } {
  const out = [...order];
  const [col] = out.splice(p, 1);
  if (col === undefined) return { order: out, frozen };
  if (p < frozen) {
    out.splice(frozen - 1, 0, col);
    return { order: out, frozen: frozen - 1 };
  }
  out.splice(frozen, 0, col);
  return { order: out, frozen: frozen + 1 };
}

export const layoutKey = (connectionId: string, database: string | null, table: string | null) => (table ? `${connectionId}/${database ?? ''}/${table.split('.').pop()?.replace(/^["`[]|["`\]]$/g, '').toLowerCase()}` : null);
