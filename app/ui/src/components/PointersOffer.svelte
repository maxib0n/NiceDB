<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { appearance, updateAppearance } from '../lib/appearance.svelte';
  import { pointerPreview } from '../lib/pointer-theme';
  import { wizardShown } from '../lib/wizard.svelte';
  import { m } from '../paraglide/messages';

  let dialog = $state<HTMLDialogElement>();
  let yes = $state<HTMLButtonElement>();
  let calm = $state(false);
  let error = $state('');
  let shots = $state<string[]>([]);

  const a = $derived(appearance.value);
  const due = $derived(a.setup_done && !a.pointers_asked && !wizardShown());

  onMount(() => {
    const free = () => !document.querySelector('.splash');
    if (free()) return void (calm = true);
    const watch = new MutationObserver(() => free() && (watch.disconnect(), (calm = true)));
    watch.observe(document.body, { childList: true });
    return () => watch.disconnect();
  });

  $effect(() => {
    if (due && calm && dialog && !dialog.open) {
      shots = pointerPreview(['default', 'pointer', 'text', 'col-resize']);
      dialog.showModal();
      void tick().then(() => yes?.focus());
    }
  });

  function answer(pointers: boolean) {
    error = '';
    updateAppearance({ pointers, pointers_asked: true }, { replaySplash: false }).catch((e: unknown) => {
      error = m.save_failed({ detail: e instanceof Error ? e.message : String(e) });
    });
  }
</script>

{#if due && calm}
  <dialog class="sheet-dialog ptr-offer" bind:this={dialog} oncancel={(e) => (e.preventDefault(), answer(false))} aria-labelledby="ptr-offer-title" aria-describedby="ptr-offer-body">
    <header class="dialog-head">
      <h2 class="display" id="ptr-offer-title">{m.pointers_offer_title()}</h2>
    </header>
    <div class="dialog-body">
      {#if shots.length}
        <div class="ptr-shots" aria-hidden="true">
          {#each shots as src, i (i)}<img {src} alt="" width="48" height="48" />{/each}
        </div>
      {/if}
      <p id="ptr-offer-body">{m.pointers_offer_body()}</p>
      <p class="hint">{m.pointers_offer_later()}</p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
    </div>
    <footer class="dialog-foot">
      <span class="spacer"></span>
      <button class="btn" type="button" onclick={() => answer(false)}>{m.pointers_offer_no()}</button>
      <button class="btn primary" type="button" bind:this={yes} onclick={() => answer(true)}>{m.pointers_offer_yes()}</button>
    </footer>
  </dialog>
{/if}
