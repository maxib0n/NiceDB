<script lang="ts">
  import { History, RotateCcw } from '@lucide/svelte';
  import { appearance } from '../lib/appearance.svelte';
  import { clip, compare, planNews } from '../lib/plan';
  import { again, defaultView, planFor, type PlanView } from '../lib/plan.svelte';
  import type { Tab } from '../lib/workspace.svelte';
  import { getLocale } from '../paraglide/runtime';
  import { m } from '../paraglide/messages';
  import PlanFigures from './PlanFigures.svelte';
  import PlanGraph from './PlanGraph.svelte';
  import PlanHistory from './PlanHistory.svelte';
  import { tag } from '../lib/format';
  import PlanNotes from './PlanNotes.svelte';
  import PlanSection from './PlanSection.svelte';
  import PlanTree from './PlanTree.svelte';
  import Segmented from './Segmented.svelte';

  let { tab }: { tab: Tab } = $props();
  let selected = $state<number | null>(null);

  const p = $derived(planFor(tab.id));
  const r = $derived(p?.current ?? null);
  const view = $derived<PlanView>(p?.view ?? defaultView(appearance.value.style));
  const before = $derived(p?.against?.report ?? p?.previous ?? null);
  const previous = $derived(r && before && before.plan.analyzed === r.plan.analyzed ? before : null);
  const news = $derived(planNews(p?.history ?? []));
  const day = (at: number | null) => (at === null ? '' : new Intl.DateTimeFormat(tag(), { day: 'numeric', month: 'short' }).format(new Date(at)));
  const comparison = $derived(r && previous && p?.compare ? compare(previous.plan, r.plan) : null);
  const time = $derived(r ? new Intl.DateTimeFormat(getLocale() === 'it' ? 'it-IT' : 'en-GB', { timeStyle: 'short' }).format(new Date(r.at ?? 0)) : '');

  const mode = $derived.by(() => {
    if (!r) return '';
    if (r.plan.engine === 'cassandra') return m.plan_traced({ time });
    if (r.plan.engine === 'elasticsearch') return m.plan_profiled({ time });
    if (r.execution.kind === 'measured') return r.plan.engine === 'mongodb' ? m.plan_measured_mongo({ time }) : r.plan.engine === 'oracle' ? m.plan_measured_oracle({ time }) : m.plan_measured({ time });
    if (r.execution.kind === 'measured_rolled_back') return r.plan.engine === 'oracle' ? m.plan_measured_rolled_back_oracle({ time }) : m.plan_measured_rolled_back({ time });
    return m.plan_estimated({ time });
  });

  $effect(() => {
    void p?.seq;
    selected = null;
  });

  function setView(v: PlanView) {
    if (p) p.view = v;
  }
</script>

<div class="pv" data-view={view} class:estimated={r ? !r.plan.analyzed : false}>
  {#if !r}
    <p class="results-empty">{p?.running ? m.plan_running() : (p?.error ?? m.plan_empty())}</p>
  {:else}
    <header class="pv-head">
      <div class="pv-title">
        <p class="pv-kicker"><b>{m.plan_heading()}</b><span>{mode}</span></p>
        <p class="pv-sql" data-tip={r.sql}>{clip(r.sql.replace(/\s+/g, ' '), 140)}</p>
        {#if news.count > 1}
          <p class="pv-story">
            {#if news.warn === 'shape'}<b class="pv-warn">{m.plan_warn_shape()}</b>{:else if news.warn === 'slower'}<b class="pv-warn">{m.plan_warn_slower({ times: (news.ratio ?? 1).toFixed(1) })}</b>{/if}
            <span>{[m.plan_story({ count: String(news.count), since: day(news.since) }), news.shapeAt !== null && m.plan_story_shape({ day: day(news.shapeAt) })].filter(Boolean).join(' · ')}</span>
          </p>
        {/if}
      </div>
      {#if p?.running}<span class="pv-busy">{m.plan_running()}</span>{/if}
      {#if p?.error}<span class="pv-error">{p.error}</span>{/if}
      <Segmented
        label={m.plan_view()}
        value={view}
        options={[
          { value: 'drawing', label: m.plan_view_drawing() },
          { value: 'slip', label: m.plan_view_slip() },
          { value: 'text', label: m.plan_view_text() },
        ] as const}
        onchange={setView}
      />
      {#if (p?.history.length ?? 0) > 1}
        <button class="btn ghost" type="button" aria-pressed={p?.historyOpen ?? false} onclick={() => p && (p.historyOpen = !p.historyOpen)}><History size={13} strokeWidth={1.8} aria-hidden="true" />{m.plan_history()}</button>
      {/if}
      {#if previous}
        <button class="btn ghost" type="button" aria-pressed={p?.compare ?? false} onclick={() => p && (p.compare = !p.compare)}>{m.plan_compare()}</button>
      {/if}
      {#if r.plan.engine !== 'cassandra' && r.plan.engine !== 'elasticsearch'}<button class="btn ghost" type="button" disabled={tab.running} onclick={() => again(tab, 'estimate')}>{m.plan_estimate_only()}</button>{/if}
      <button class="btn" type="button" disabled={tab.running} onclick={() => again(tab, 'analyze')}>
        <RotateCcw size={13} strokeWidth={1.8} aria-hidden="true" />
        {m.plan_again()}
      </button>
    </header>

    <PlanFigures report={r} {comparison} seq={p?.seq ?? 0} />

    <div class="pv-body">
      <div class="pv-main">
        {#if r.unreadable || view === 'text'}
          {#if r.unreadable}<p class="pv-unreadable">{m.plan_unreadable()}</p>{/if}
          <pre class="pv-raw">{r.plan.raw}</pre>
        {:else if view === 'drawing'}
          <PlanGraph plan={r.plan} {selected} onselect={(id) => (selected = id)} />
          <PlanSection plan={r.plan} />
          <PlanTree plan={r.plan} {comparison} {selected} onselect={(id) => (selected = id)} compact />
        {:else}
          <PlanTree plan={r.plan} {comparison} {selected} onselect={(id) => (selected = id)} />
        {/if}
      </div>
      {#if p?.historyOpen && p.history.length > 1}
        <PlanHistory tabId={tab.id} plan={p} />
      {:else}
        <PlanNotes report={r} previous={p?.compare ? previous : null} {comparison} {selected} onclose={() => (selected = null)} />
      {/if}
    </div>
  {/if}
</div>
