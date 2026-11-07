import { sqlOf } from './family';
import { api, type PlanReport, type SessionInfo } from './api';
import type { SavedPlan_Serialize as SavedPlan } from './bindings';
import type { ExplainMode, RunScope } from './bindings';
import { paramsForRun } from './params.svelte';
import { sessionOf, type Tab } from './workspace.svelte';
import { m } from '../paraglide/messages';

export type PlanView = 'drawing' | 'slip' | 'text';

export interface TabPlan {
  current: PlanReport | null;
  previous: PlanReport | null;
  error: string | null;
  running: boolean;
  view: PlanView | null;
  compare: boolean;
  seq: number;
  scope: RunScope | null;
  history: SavedPlan[];
  historyOpen: boolean;
  against: { id: number; report: PlanReport } | null;
}

const plans = $state<Record<string, TabPlan>>({});

export const planFor = (tabId: string): TabPlan | null => plans[tabId] ?? null;

export function defaultView(style: string): PlanView {
  return style === 'registro' ? 'slip' : 'drawing';
}

function slot(tabId: string): TabPlan {
  plans[tabId] ??= { current: null, previous: null, error: null, running: false, view: null, compare: true, seq: 0, scope: null, history: [], historyOpen: false, against: null };
  return plans[tabId];
}

export async function explain(tab: Tab, scope: RunScope, mode: ExplainMode = 'analyze') {
  const s = sessionOf(tab);
  if (!s || tab.running) return;
  const params = await paramsForRun(tab.id, tab.text, sqlOf(s.info), scope).catch(() => null);
  if (params === false || tab.running) return;
  const p = slot(tab.id);
  const runId = crypto.randomUUID();
  tab.runId = runId;
  p.running = true;
  p.error = null;
  p.scope = scope;
  p.seq++;
  try {
    const report = await api.explain({ run_id: runId, session_id: s.info.session_id, text: tab.text, scope, mode, params });
    if (p.current) p.previous = p.current;
    p.current = report;
    p.against = null;
    p.seq++;
    void loadHistory(p, s.info, report);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    p.error = message === 'cancelled' ? null : message.includes('SQL0219N') ? `${message} ${m.plan_db2_explain_tables()}` : message;
  } finally {
    p.running = false;
    if (tab.runId === runId) tab.runId = null;
  }
}

export function again(tab: Tab, mode: ExplainMode) {
  const p = planFor(tab.id);
  const scope = p?.scope ?? (p?.current ? { kind: 'statement' as const, cursor: p.current.from } : null);
  if (scope) void explain(tab, scope, mode);
}

async function loadHistory(p: TabPlan, info: SessionInfo, report: PlanReport) {
  const seq = p.seq;
  const history = await api.planHistory(info.connection_id, info.server.database, report.sql).catch(() => []);
  if (p.seq === seq) p.history = history;
}

export async function compareWith(tabId: string, id: number) {
  const p = planFor(tabId);
  if (!p) return;
  if (p.against?.id === id) {
    p.against = null;
    return;
  }
  const report = await api.planSaved(id).catch(() => null);
  if (!report) return;
  p.against = { id, report };
  p.compare = true;
}
