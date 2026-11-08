<script lang="ts">
  import { Check, Undo2 } from '@lucide/svelte';
  import { tick } from 'svelte';
  import { leave } from '../lib/leave.svelte';
  import { m } from '../paraglide/messages';

  const ask = $derived(leave.ask);
  let dialog = $state<HTMLDialogElement>();
  let stay = $state<HTMLButtonElement>();

  $effect(() => {
    if (ask && dialog && !dialog.open) {
      dialog.showModal();
      void tick().then(() => stay?.focus());
    }
    if (!ask && dialog?.open) dialog.close();
  });
</script>

<dialog class="sheet-dialog tx-close leave-dialog" bind:this={dialog} oncancel={(e) => (e.preventDefault(), ask?.resolve(false))} aria-labelledby="leave-title">
  {#if ask}
    <header class="dialog-head">
      <h2 class="display" id="leave-title">{ask.count === 1 ? m.leave_title_one() : m.leave_title_other({ count: ask.count })}</h2>
    </header>
    <div class="dialog-body txc-body">
      <p class="txc-note">{m.leave_body()}</p>
    </div>
    <footer class="dialog-foot">
      <button class="btn" type="button" bind:this={stay} onclick={() => ask?.resolve(false)}>{m.leave_stay()}</button>
      <span class="spacer"></span>
      <button class="btn" type="button" onclick={() => ask?.resolve('discard')}><Undo2 size={13} strokeWidth={1.8} aria-hidden="true" /> {m.leave_discard()}</button>
      <button class="btn primary" type="button" onclick={() => ask?.resolve('apply')}><Check size={13} strokeWidth={2} aria-hidden="true" /> {m.leave_apply()}</button>
    </footer>
  {/if}
</dialog>
