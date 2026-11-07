<script lang="ts">
  import { CircleCheck, FileUp, TriangleAlert } from '@lucide/svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import type { DocumentSample, DocumentsImported, OnRowError } from '../lib/bindings';
  import { connections } from '../lib/connections.svelte';
  import { num } from '../lib/format';
  import { duration, fileName } from '../lib/tools';
  import { confirmToolWrite, errorText, Job, type DocumentsAsk } from '../lib/tools.svelte';
  import { refreshSchema, sessionFor } from '../lib/workspace.svelte';
  import { m } from '../paraglide/messages';
  import JobBar from './JobBar.svelte';
  import Segmented from './Segmented.svelte';
  import WriteLock from './WriteLock.svelte';

  let { ask, onclose }: { ask: DocumentsAsk; onclose: () => void } = $props();

  const session = $derived(sessionFor(ask.connectionId, ask.database || null) ?? sessionFor(ask.connectionId));
  const production = $derived(session?.info.environment === 'production');
  const words = $derived(session?.info.caps.words.table ?? 'collection');

  let path = $state<string | null>(null);
  let sample = $state.raw<DocumentSample | null>(null);
  let collection = $state(untrack(() => ask.collection));
  let onError = $state<OnRowError>('skip');
  let failure = $state<string | null>(null);
  let result = $state.raw<DocumentsImported | null>(null);
  let shown = $state(0);
  let reading = $state(false);
  const job = new Job();
  let dialog: HTMLDialogElement;

  $effect(() => dialog.showModal());

  const LAYOUT = { array: m.doc_import_array, lines: m.doc_import_lines, stream: m.doc_import_stream };

  async function choose() {
    const p = await api.pickOpen(['json', 'ndjson', 'jsonl'], m.doc_import_files());
    if (!p) return;
    path = p;
    result = null;
    failure = null;
    sample = null;
    reading = true;
    try {
      sample = await api.documentImportSample(p);
      shown = 0;
    } catch (e) {
      failure = errorText(e);
    } finally {
      reading = false;
    }
  }

  function close() {
    if (job.running) void job.cancel();
    dialog.close();
    onclose();
  }

  const finished = $derived(!!result && !result.stopped && !result.cancelled);

  async function start() {
    if (finished) return close();
    if (!session || !path || !sample || !collection.trim()) return;
    failure = null;
    result = null;
    const name = connections.list.find((c) => c.id === ask.connectionId)?.name ?? '';
    const into = collection.trim();
    try {
      const typed = await confirmToolWrite(session.info, m.tool_import_confirm({ table: into }), [fileName(path), name, m.doc_import_count({ count: num(sample.count ?? 0) })], m.tool_import_go(), m.doc_import_note(), false);
      if (typed === false) return;
      const file = path;
      result = await job.run((job_id, onProgress) =>
        api.documentImport({ job_id, session_id: session.info.session_id, path: file, database: ask.database, collection: into, on_error: onError, confirm: typed }, onProgress),
      );
      if ((result.inserted ?? 0) > 0) void refreshSchema(ask.connectionId, ask.database || null);
    } catch (e) {
      failure = errorText(e);
    }
  }
</script>

<dialog class="sheet-dialog tool-dialog wide" bind:this={dialog} oncancel={(e) => (e.preventDefault(), close())} aria-labelledby="dd-title">
  <form
    method="dialog"
    onsubmit={(e) => {
      e.preventDefault();
      void start();
    }}
  >
    <header class="dialog-head">
      <h2 id="dd-title" class="display">{ask.collection ? m.tool_import_into({ table: ask.collection }) : m.doc_import_title()}</h2>
      <p class="hint">{path ? fileName(path) : m.doc_import_lead()}</p>
    </header>
    <div class="dialog-body">
      {#if result}
        {#if finished}
          <p class="tool-done big" role="status"><CircleCheck size={16} strokeWidth={1.8} aria-hidden="true" /> {m.doc_import_done({ count: num(result.inserted ?? 0), target: collection.trim(), time: duration(result.elapsed_ms) })}</p>
          {#if result.skipped}<p class="tool-warn">{m.doc_import_skipped({ count: num(result.skipped ?? 0) })}</p>{/if}
        {:else if result.cancelled}
          <p class="tool-warn" role="status">{m.doc_import_cancelled({ count: num(result.inserted ?? 0) })}</p>
        {:else}
          <p class="tool-warn big" role="alert"><TriangleAlert size={16} strokeWidth={1.8} aria-hidden="true" /> {m.doc_import_stopped({ line: num(result.faults[0]?.line ?? 0), count: num(result.inserted ?? 0) })}</p>
        {/if}
        {#if result.faults.length}
          <h3 class="caps tool-sub">{m.doc_import_faults()}</h3>
          <ol class="di-faults">
            {#each result.faults as f, i (i)}<li><span class="mono">{m.tool_import_line({ line: num(f.line ?? 0) })}</span> {f.message}</li>{/each}
          </ol>
          {#if result.faults_cut}<p class="hint">{m.tool_import_faults_cut()}</p>{/if}
        {/if}
      {:else if !path}
        <div class="di-start">
          <button class="btn primary" type="button" onclick={() => void choose()}><FileUp size={14} strokeWidth={1.8} aria-hidden="true" /> {m.tool_import_choose()}</button>
          <p class="hint">{m.doc_import_formats()}</p>
        </div>
      {:else}
        <div class="di-file">
          <span class="tool-tag">{sample ? LAYOUT[sample.layout]() : 'JSON'}</span>
          <span class="mono di-path" data-tip={path}>{path}</span>
          <button class="btn ghost small" type="button" disabled={job.running || reading} onclick={() => void choose()}>{m.tool_import_other()}</button>
        </div>
        {#if reading}<p class="dd-count dd-reading" role="status">{m.doc_import_reading()}</p>{/if}
        {#if sample}
          <p class="dd-count">{m.doc_import_count({ count: num(sample.count ?? 0) })}{#if sample.faults.length}{' · '}<span class="dd-bad">{m.doc_import_bad({ count: num(sample.faults.length) })}</span>{/if}</p>
          {#if sample.preview.length}
            <div class="dd-preview">
              <div class="dd-nav">
                <span class="caps">{m.doc_import_preview({ n: shown + 1, of: sample.preview.length })}</span>
                <button class="btn ghost small" type="button" disabled={shown === 0} onclick={() => shown--}>←</button>
                <button class="btn ghost small" type="button" disabled={shown >= sample.preview.length - 1} onclick={() => shown++}>→</button>
              </div>
              <pre class="mono dd-doc">{sample.preview[shown]}</pre>
            </div>
          {/if}
          {#if sample.faults.length}
            <h3 class="caps tool-sub">{m.doc_import_faults_file()}</h3>
            <ol class="di-faults">
              {#each sample.faults.slice(0, 20) as f, i (i)}<li><span class="mono">{m.tool_import_line({ line: num(f.line ?? 0) })}</span> {f.message}</li>{/each}
            </ol>
          {/if}
          <label class="form-field">
            <span class="caps">{words === 'index' ? m.doc_import_index() : words === 'table' ? m.tool_import_table_name() : m.doc_import_collection()}</span>
            <span class="field"><input bind:value={collection} spellcheck="false" autocomplete="off" /></span>
          </label>
          <div class="di-options">
            <Segmented
              label={m.tool_import_on_error()}
              value={onError}
              options={[
                { value: 'skip', label: m.tool_import_skip() },
                { value: 'stop', label: m.tool_import_stop() },
              ] as const}
              onchange={(v) => (onError = v)}
            />
          </div>
          <p class="hint">{onError === 'stop' ? m.doc_import_stop_hint() : m.doc_import_skip_hint()}</p>
        {/if}
        {#if job.running}<JobBar {job} label={m.tool_import_running()} written={(count) => m.doc_import_count({ count })} />{/if}
      {/if}
      {#if failure}<p class="test-result" role="alert">{failure}</p>{/if}
    </div>
    <footer class="dialog-foot">
      {#if production && session}<WriteLock sessionId={session.info.session_id} />{/if}
      <span class="spacer"></span>
      {#if !finished && !job.running}<button class="btn ghost" type="button" onclick={close}>{result ? m.org_close() : m.form_cancel()}</button>{/if}
      {#if finished}
        <button class="btn primary" type="submit">{m.org_close()}</button>
      {:else if path && sample}
        <button class="btn primary" type="submit" disabled={job.running || sample.count === 0 || !collection.trim()}>{result ? m.tool_import_again() : m.doc_import_go({ count: num(sample.count ?? 0) })}</button>
      {/if}
    </footer>
  </form>
</dialog>
