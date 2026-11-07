import type { ExplainRequest, PlanReport } from './api';
import type { SavedPlan_Serialize as SavedPlan } from './bindings';
import data from './mock-plan.json';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function estimated(r: PlanReport): PlanReport {
  return {
    ...r,
    execution: { kind: 'estimated', reason: 'requested' },
    plan: {
      ...r.plan,
      analyzed: false,
      total_ms: null,
      slowest: null,
      worst_estimate: null,
      nodes: r.plan.nodes.map((n) => ({ ...n, rows: null, loops: null, rows_removed: null, time_ms: null, self_ms: null, buffers: null })),
    },
  };
}

export function planMock(running: Map<string, () => void>, own: (sid: string) => PlanReport | null = () => null, retell: (r: PlanReport) => PlanReport = (r) => r) {
  const seen = new Map<string, number>();
  const kept: { id: number; connection: string; key: string; shape: string; report: PlanReport }[] = [];
  const keyOf = (sql: string) => sql.toLowerCase().replace(/'[^']*'|\b\d+(\.\d+)?\b/g, '?').replace(/\s+/g, ' ').trim().replace(/;$/, '');
  const shapeOf = (r: PlanReport) => r.plan.nodes.map((n) => `${n.depth} ${n.op} ${n.relation ?? ''} ${n.index ?? ''}`).join('\n');
  const keep = (sid: string, r: PlanReport) => kept.push({ id: kept.length + 1, connection: sid.replace(/^mock-/, '').replace(/@.*$/, ''), key: keyOf(r.sql), shape: shapeOf(r), report: r });
  return {
    planHistory: async (connectionId: string, _database: string, sql: string): Promise<SavedPlan[]> => {
      const mine = kept.filter((k) => k.connection === connectionId && k.key === keyOf(sql)).reverse();
      return mine.map((k, i) => ({
        id: k.id,
        at: k.report.at,
        analyzed: k.report.plan.analyzed,
        total: k.report.plan.analyzed ? k.report.plan.total_ms : (k.report.plan.nodes[0]?.total_cost ?? null),
        shape_changed: !!mine[i + 1] && mine[i + 1]?.shape !== k.shape,
      }));
    },
    planSaved: async (id: number): Promise<PlanReport | null> => structuredClone(kept.find((x) => x.id === id)?.report ?? null),
    explain: async (req: ExplainRequest): Promise<PlanReport> => {
      let cancelled = false;
      running.set(req.run_id, () => (cancelled = true));
      await sleep(260);
      running.delete(req.run_id);
      if (cancelled) throw new Error('cancelled');
      const n = seen.get(req.session_id) ?? 0;
      seen.set(req.session_id, n + 1);
      const base = own(req.session_id) ?? retell(structuredClone((n === 0 ? data.before : data.after) as PlanReport));
      const out = req.mode === 'estimate' ? estimated(base) : base;
      keep(req.session_id, { ...out, at: (out.at ?? 0) + n * 86_400_000 });
      return out;
    },
  };
}
