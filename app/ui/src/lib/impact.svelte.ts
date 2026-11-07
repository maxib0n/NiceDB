import type { RunScope } from './bindings';
import { api } from './api';
import { linksTo, namespaceOf, type Effect } from './impact';
import type { SessionState } from './workspace.svelte';

export async function effectsOf(s: SessionState, text: string, scope: RunScope): Promise<Effect[]> {
  if (!s.info.dialect) return [];
  try {
    const found = await api.writeRemovals(s.info.session_id, text, scope);
    const out: Effect[] = [];
    for (const r of found) {
      const ns = namespaceOf(s.schema, r.namespace, r.name, s.searchPath);
      const links = linksTo(s.schema, ns, r.name);
      const counts = links.length ? await api.writeLinked(s.info.session_id, text, scope, r.index, links) : [];
      out.push({
        index: r.index,
        kind: r.kind,
        table: r.namespace ? `${r.namespace}.${r.name}` : r.name,
        rows: r.rows ?? null,
        linked: links.map((l, i) => ({ table: `${l.namespace}.${l.table}`, via: l.via, action: l.action, rows: counts[i] ?? null })),
      });
    }
    return out;
  } catch {
    return [];
  }
}
