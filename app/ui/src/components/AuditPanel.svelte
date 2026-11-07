<script lang="ts">
  import { Download, Undo2, X } from '@lucide/svelte';
  import { api, driverMessage, type AuditEntry } from '../lib/api';
  import { askConfirm, writesOpen } from '../lib/guard.svelte';
  import { ensureSession, sessionFor, workspace } from '../lib/workspace.svelte';
  import { play } from '../lib/sound';
  import { auditCsv, auditFile, auditJson } from '../lib/audit';
  import { connections } from '../lib/connections.svelte';
  import { num, when } from '../lib/format';
  import { m } from '../paraglide/messages';
  import Menu from './Menu.svelte';
  import Resizer from './Resizer.svelte';

  let { connectionId, rev, onclose, onundone }: { connectionId: string; rev: number; onclose: () => void; onundone?: () => void } = $props();
  let entries = $state<AuditEntry[]>([]);
  let failure = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let undoing = $state<number | null>(null);
  let bump = $state(0);

  const lost = (e: AuditEntry) => !e.undo && !e.error && (e.rows ?? 0) > 0 && !e.sql.startsWith('--');

  async function sessionOfEntry(database: string) {
    const known = () => {
      const main = workspace.sessions[connectionId];
      if (main && (!database || main.info.server.database === database || main.database === database)) return main;
      return Object.values(workspace.extra[connectionId] ?? {}).find((x) => x.info.server.database === database || x.database === database) ?? null;
    };
    if (known()) return known();
    if (!(await ensureSession(connectionId, null))) return null;
    if (known()) return known();
    return (await ensureSession(connectionId, database)) ? sessionFor(connectionId, database) : null;
  }

  async function undo(e: AuditEntry) {
    const set = e.undo;
    if (!set || undoing !== null) return;
    failure = null;
    notice = null;
    undoing = e.id;
    try {
      const s = await sessionOfEntry(e.database);
      if (!s) throw new Error(m.audit_undo_no_session());
      const production = s.info.environment === 'production';
      if (production && !writesOpen(s.info.session_id)) throw new Error(m.data_ro_locked());
      const preview = await api.dataPreview(s.info.session_id, set);
      const name = connections.list.find((c) => c.id === connectionId)?.name ?? '';
      const typed = await askConfirm({
        title: m.audit_undo_title({ count: set.changes.length }),
        connection: name,
        production,
        lines: preview.statements.slice(0, 8).map((x) => (x.length > 160 ? `${x.slice(0, 160)}…` : x)),
        risks: [],
        note: m.audit_undo_note(),
        action: m.audit_undo_btn(),
      });
      if (typed === false) return;
      const out = await api.dataApply(s.info.session_id, set, typed);
      if (out.kind === 'applied') {
        play('applied');
        notice = m.audit_undone();
        onundone?.();
      } else {
        play('error');
        failure = out.kind === 'conflict' ? m.audit_undo_conflict({ n: out.index + 1 }) : m.data_failed({ n: out.index + 1, message: driverMessage(out.error) });
      }
      bump++;
    } catch (err) {
      failure = err instanceof Error ? err.message : String(err);
    } finally {
      undoing = null;
    }
  }

  async function exportAs(kind: 'csv' | 'json') {
    failure = null;
    try {
      const all = await api.auditList(connectionId, 100_000);
      const text = kind === 'csv' ? auditCsv(all) : auditJson(all);
      const name = connections.list.find((c) => c.id === connectionId)?.name ?? connectionId;
      await api.exportSave(auditFile(name, kind), new TextEncoder().encode(text));
    } catch (e) {
      failure = m.audit_export_failed({ message: e instanceof Error ? e.message : String(e) });
    }
  }

  $effect(() => {
    void rev;
    void bump;
    void api
      .auditList(connectionId, 200)
      .then((e) => (entries = e))
      .catch(() => (entries = []));
  });
</script>

<aside class="history audit" aria-label={m.audit_title()}>
  <Resizer pane="audit" side="start" label={m.resize_audit()} />
  <header class="history-head">
    <h2 class="caps">{m.audit_title()}</h2>
    <span class="spacer"></span>
    <Menu
      label={m.audit_export()}
      disabled={entries.length === 0}
      items={[
        { value: 'csv', label: m.audit_export_csv() },
        { value: 'json', label: m.audit_export_json() },
      ] as const}
      onpick={(v) => void exportAs(v)}
    >
      <Download size={13} strokeWidth={1.8} aria-hidden="true" />
      {m.audit_export()}
    </Menu>
    <button class="icon-btn" type="button" aria-label={m.audit_close()} onclick={onclose}><X size={14} strokeWidth={1.8} /></button>
  </header>
  {#if failure}<p class="audit-failure" role="alert">{failure}</p>{/if}
  {#if notice}<p class="audit-notice" role="status">{notice}</p>{/if}
  {#if entries.length === 0}
    <p class="history-empty">{m.audit_empty()}</p>
  {:else}
    <ul class="audit-list">
      {#each entries as e (e.id)}
        <li class:err={!!e.error}>
          <span class="audit-meta">{when(e.at ?? 0)} · {e.who || '—'}{#if e.database} · {e.database}{/if}{#if e.rows !== null} · {e.rows === 1 ? m.audit_row_one() : m.audit_rows({ count: num(e.rows) })}{/if}</span>
          <code>{e.sql.length > 400 ? `${e.sql.slice(0, 400)}…` : e.sql}</code>
          {#if e.error}<span class="audit-err">{e.error}</span>{/if}
          {#if e.undo}
            <button class="btn ghost small audit-undo" type="button" disabled={undoing !== null} onclick={() => void undo(e)}><Undo2 size={12} strokeWidth={1.8} aria-hidden="true" />{m.audit_undo()}</button>
          {:else if lost(e)}
            <span class="audit-lost" data-tip={m.audit_undo_lost_tip()}>{m.audit_undo_lost()}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</aside>
