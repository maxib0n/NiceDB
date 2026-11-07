<script lang="ts">
  import { ClipboardCopy, Search, SquarePlus, TextCursorInput, X } from '@lucide/svelte';
  import { copyText, openMenu } from '../lib/contextmenu.svelte';
  import { api, type HistoryEntry } from '../lib/api';
  import type { HistoryOutcome } from '../lib/bindings';
  import { connections } from '../lib/connections.svelte';
  import { workspace } from '../lib/workspace.svelte';
  import { num, when } from '../lib/format';
  import { m } from '../paraglide/messages';
  import Resizer from './Resizer.svelte';
  import Segmented from './Segmented.svelte';

  let { connectionId, oninsert, onopen, onclose }: { connectionId: string; oninsert: (sql: string) => void; onopen: (sql: string, connectionId: string) => void; onclose: () => void } = $props();

  function entryMenu(e: MouseEvent, entry: HistoryEntry) {
    openMenu(e, [
      { label: m.history_insert(), icon: TextCursorInput, run: () => oninsert(entry.sql) },
      { label: m.history_open_tab(), icon: SquarePlus, run: () => onopen(entry.sql, entry.connection_id) },
      'separator',
      { label: m.menu_copy(), icon: ClipboardCopy, run: () => copyText(entry.sql) },
    ]);
  }
  let query = $state('');
  let scope = $state<'here' | 'all'>('here');
  let outcome = $state<HistoryOutcome>('any');
  let entries = $state<HistoryEntry[]>([]);
  const SCOPES = [
    { value: 'here', label: m.history_scope_here() },
    { value: 'all', label: m.history_scope_all() },
  ] as const;
  const OUTCOMES = [
    { value: 'any', label: m.history_outcome_any() },
    { value: 'ok', label: m.history_outcome_ok() },
    { value: 'failed', label: m.history_outcome_failed() },
  ] as const;
  const nameOf = (id: string) => connections.list.find((c) => c.id === id)?.name ?? id;

  $effect(() => {
    const q = query;
    const id = scope === 'all' ? null : connectionId;
    const o = outcome;
    void workspace.runs;
    const t = setTimeout(() => {
      void api.historyList(id, q, o, 200).then((r) => (entries = r));
    }, 120);
    return () => clearTimeout(t);
  });
</script>

<aside class="history" aria-label={m.run_history()}>
  <Resizer pane="history" side="start" label={m.resize_history()} />
  <div class="history-head">
    <h2 class="caps">{m.run_history()}</h2>
    <span class="spacer"></span>
    <button class="icon-btn" type="button" aria-label={m.history_close()} onclick={onclose}><X size={15} strokeWidth={1.8} /></button>
  </div>
  <label class="field">
    <Search size={13} strokeWidth={1.8} aria-hidden="true" />
    <input type="search" placeholder={m.history_search()} aria-label={m.history_search()} bind:value={query} />
  </label>
  <div class="history-filters">
    <Segmented label={m.history_scope()} value={scope} options={SCOPES} onchange={(v) => (scope = v)} />
    <Segmented label={m.history_outcome()} value={outcome} options={OUTCOMES} onchange={(v) => (outcome = v)} />
  </div>
  {#if entries.length === 0}
    <p class="hint history-empty">{m.history_empty()}</p>
  {:else}
    <ul>
      {#each entries as e (e.id)}
        <li>
          <button type="button" class:err={!!e.error} data-tip={m.history_insert()} onclick={() => oninsert(e.sql)} oncontextmenu={(ev) => entryMenu(ev, e)}>
            <code>{e.sql}</code>
            <span class="history-meta">
              {[scope === 'all' && nameOf(e.connection_id), when(e.started_at ?? 0), `${num(e.elapsed_ms ?? 0)} ms`, e.rows !== null && m.results_rows({ count: num(e.rows) })].filter(Boolean).join(' · ')}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>
