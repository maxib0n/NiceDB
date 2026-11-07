<script lang="ts">
  import { count, ms } from '../lib/plan';
  import { compareWith, type TabPlan } from '../lib/plan.svelte';
  import { tag } from '../lib/format';
  import { m } from '../paraglide/messages';

  let { tabId, plan }: { tabId: string; plan: TabPlan } = $props();

  const when = (at: number) => new Intl.DateTimeFormat(tag(), { dateStyle: 'short', timeStyle: 'short' }).format(new Date(at));
  const analyzed = $derived(plan.current?.plan.analyzed ?? true);
</script>

<aside class="pv-notes ph" aria-label={m.plan_history()}>
  <h4 class="pn-head"><span>{m.plan_history()}</span><small>{m.plan_history_count({ count: String(plan.history.length) })}</small></h4>
  <ol class="ph-list">
    {#each plan.history as h, i (h.id ?? i)}
      {@const other = h.analyzed !== analyzed}
      <li>
        <button
          type="button"
          class:on={plan.against?.id === h.id}
          class:now={i === 0}
          disabled={i === 0 || other}
          aria-pressed={plan.against?.id === h.id}
          data-tip={i === 0 ? m.plan_history_now() : other ? m.plan_history_other() : m.plan_history_pick()}
          onclick={() => void compareWith(tabId, h.id ?? 0)}
        >
          <span class="ph-mark" aria-hidden="true">{h.shape_changed ? '◆' : ''}</span>
          <span class="ph-when">{when(h.at ?? 0)}</span>
          <b class="ph-total">{h.analyzed ? ms(h.total) : count(h.total)}</b>
          <span class="ph-note">{i === 0 ? m.plan_history_now() : h.shape_changed ? m.plan_history_shape() : h.analyzed ? '' : m.plan_estimate_short()}</span>
        </button>
      </li>
    {/each}
  </ol>
  <p class="ph-hint">{m.plan_history_hint()}</p>
</aside>
