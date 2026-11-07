import { m } from '../paraglide/messages';
import type { FkAction, RemovalKind, Schema_Serialize as Schema } from './bindings';
import { incoming } from './fk';
import { num } from './format';

export interface Effect {
  index: number;
  kind: RemovalKind;
  table: string;
  rows: number | null;
  linked: { table: string; via: string; action: FkAction; rows: number | null }[];
}

export function namespaceOf(schema: Schema | null, written: string, name: string, path: readonly string[] | null): string {
  if (written) return written;
  const has = (ns: string) => schema?.namespaces.some((n) => n.name === ns && n.tables.some((t) => t.name === name));
  return [...(path ?? [])].find(has) ?? schema?.namespaces.find((n) => n.tables.some((t) => t.name === name))?.name ?? '';
}

export const linksTo = (schema: Schema | null, namespace: string, name: string) =>
  incoming(schema, { namespace, name }).map((i) => ({ namespace: i.namespace, table: i.table, via: i.fk.name, action: i.fk.on_delete, columns: i.fk.columns, ref_columns: i.fk.ref_columns }));

const rowsText = (n: number | null) => (n === null ? m.impact_rows_unknown() : n === 1 ? m.impact_row_one() : m.impact_rows({ count: num(n) }));

export function effectLines(effects: readonly Effect[], inline = false): string[] {
  return effects.flatMap((e) => {
    const head = { delete: m.impact_delete, truncate: m.impact_truncate, drop: m.impact_drop }[e.kind]({ table: e.table, rows: rowsText(e.rows) });
    const links = e.linked
      .filter((l) => l.rows !== 0)
      .map((l) => {
        const rows = rowsText(l.rows);
        if (e.kind === 'drop') return m.impact_link_drop({ table: l.table, rows });
        if (l.action === 'cascade') return m.impact_link_cascade({ table: l.table, rows });
        if (l.action === 'set_null' || l.action === 'set_default') return m.impact_link_set_null({ table: l.table, rows });
        return m.impact_link_block({ table: l.table, rows });
      });
    return inline ? [[head, ...links].join(' · ')] : [head, ...links.map((l) => `↳ ${l}`)];
  });
}
