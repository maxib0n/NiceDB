<script lang="ts">
  import { onScrollbar } from '../lib/scrollbar';
  import { ArrowDownWideNarrow, ArrowUpNarrowWide, ArrowUpRight, ChevronDown, ChevronUp, ClipboardCopy, Eraser, Filter, Link2, Lock, MoveHorizontal, Pencil, Pin, PinOff, RotateCcw, Search, Trash2, X } from '@lucide/svelte';
  import { tick } from 'svelte';
  import { api } from '../lib/api';
  import { presentation } from '../lib/presentation.svelte';
  import { Eye, EyeOff } from '@lucide/svelte';
  import { copyText, keys, openMenu, type MenuEntry } from '../lib/contextmenu.svelte';
  import { insertable, isNumericType, render, tableFromSql, type Cell as Value, type CopyFormat } from '../lib/copy';
  import { clipCell, COLUMN_WINDOW_FROM, columnWindow, type GridData, type GridEdit, type SortKey } from '../lib/grid';
  import * as S from '../lib/selection';
  import { appearance } from '../lib/appearance.svelte';
  import { numericHint, type Stats } from '../lib/aggregate';
  import type { SummaryColumn } from '../lib/summary';
  import { publish, retract } from '../lib/aggregate.svelte';
  import { cellText, nullText, type CellStyle } from '../lib/prefs';
  import { prefs } from '../lib/prefs.svelte';
  import type { SortDir } from '../lib/sort';
  import type { Dialect } from '../lib/bindings';
  import { m } from '../paraglide/messages';
  import { sizeText } from '../lib/value';
  import { isBinaryType } from '../lib/data.svelte';
  import { tag } from '../lib/format';
  import { arrange, loadLayout, moveColumn, saveLayout, toggleFrozen } from '../lib/grid-layout';

  const dense = $derived(appearance.value.style === 'listino');
  const ROW = $derived(dense ? 20 : 24);
  const HEAD = $derived(dense ? 24 : 30);
  const OVERSCAN = 12;
  const MAX_BODY = 8_000_000;
  const GUTTER_MIN = 44;
  const EDGE = 28;
  const HUGE_EDIT = 100_000;

  interface Props {
    result: GridData;
    edit?: GridEdit | undefined;
    sort?: readonly SortKey[];
    onsort?: ((c: number, add: boolean, dir?: SortDir) => void) | undefined;
    unsortable?: string | null;
    onblocked?: (() => void) | undefined;
    onfilter?: ((c: number, value: string | null) => void) | undefined;
    extra?: MenuEntry[];
    links?: ((c: number) => string | null) | undefined;
    onfollow?: ((r: number, c: number, newTab: boolean) => void) | undefined;
    cellmenu?: ((r: number, c: number) => MenuEntry[]) | undefined;
    onedit?: ((e: { r: number; c: number; value: string; input: HTMLInputElement } | null) => void) | undefined;
    editkey?: ((e: KeyboardEvent) => boolean) | undefined;
    gridkey?: ((e: KeyboardEvent) => boolean) | undefined;
    onfocus?: ((cell: S.Cell | null) => void) | undefined;
    dialect?: Dialect | null;
    partial?: boolean;
    summarize?: ((cols: SummaryColumn[]) => Promise<Stats>) | null;
    layout?: string | null;
  }
  let { layout = null, partial = false, summarize = null, result, edit, sort = [], onsort, unsortable = null, onblocked, onfilter, extra = [], links, onfollow, cellmenu, onedit, editkey, gridkey, onfocus, dialect = null }: Props = $props();

  let scroller: HTMLDivElement;
  let scrollTop = $state(0);
  let viewport = $state(400);
  let scrollLeft = $state(0);
  let viewWidth = $state(1200);
  let widths = $state<number[]>([]);
  let order = $state<number[]>([]);
  let frozen = $state(0);
  let moving = $state<{ from: number; to: number } | null>(null);
  let measuredFor: GridData | null = null;
  let measuredColumns: unknown = null;
  let grown: number[] = [];
  let grownFor: unknown = null;
  let editing = $state<{ r: number; c: number; value: string; huge?: number } | null>(null);
  let editInput = $state<HTMLInputElement | null>(null);
  let sel = $state.raw<S.Selection>(S.EMPTY);

  const count = $derived(result.count);
  const columns = $derived(result.columns);
  const dc = (p: number) => order[p] ?? p;
  const cols = $derived(order.length === columns.length ? order.map((i) => columns[i] ?? { name: '', type_name: null }) : columns);
  const bounds = $derived({ rows: count, cols: columns.length });
  const numeric = $derived(columns.map((c) => isNumericType(c.type_name ?? null)));
  const style = $derived<CellStyle>({ nulls: prefs.value.results.null_display, dates: prefs.value.results.dates === 'local', numbers: prefs.value.results.numbers === 'grouped', locale: appearance.value.locale });
  const nullShown = $derived(nullText(style.nulls));
  const show = (v: string, c: number) => cellText(clipCell(v), columns[c]?.type_name ?? null, numeric[c] ?? false, style);
  const total = $derived(count * ROW);
  const bodyHeight = $derived(Math.min(total, MAX_BODY));
  const ratio = $derived(bodyHeight > viewport ? (total - viewport) / (bodyHeight - viewport) : 1);
  const position = $derived(scrollTop * ratio);
  const first = $derived(Math.max(0, Math.floor(position / ROW) - OVERSCAN));
  const last = $derived(Math.min(count, Math.ceil((position + viewport) / ROW) + OVERSCAN));
  const windowTop = $derived(scrollTop - (position - first * ROW));
  const visible = $derived.by(() => {
    void count;
    void result.rev;
    return result.rows.slice(first, last);
  });
  const gutter = $derived(Math.max(GUTTER_MIN, (edit ? Math.max(edit.label(0).length, edit.label(count - 1).length) + 2 : String(count).length) * 8 + 18));
  const shownWidths = $derived(cols.map((_, p) => widths[dc(p)] ?? 100));
  const offsets = $derived.by(() => {
    const out = [0];
    for (const w of shownWidths) out.push((out.at(-1) ?? 0) + w);
    return out;
  });
  const span = $derived.by(() => {
    if (columns.length <= COLUMN_WINDOW_FROM) return { c0: 0, c1: columns.length };
    const w = columnWindow(offsets, scrollLeft - gutter, viewWidth, viewWidth / 2);
    const keep = editing?.c;
    return keep === undefined ? w : { c0: Math.min(w.c0, keep), c1: Math.max(w.c1, keep + 1) };
  });
  const track = $derived.by(() => {
    const lo = Math.max(span.c0, frozen);
    const out: (number | 'skip')[] = [];
    for (let p = 0; p < Math.min(frozen, cols.length); p++) out.push(p);
    if (lo > frozen) out.push('skip');
    for (let p = lo; p < span.c1; p++) out.push(p);
    return out;
  });
  const template = $derived.by(() => {
    const n = shownWidths.length;
    const px = (w: number) => `${w}px`;
    const lo = Math.max(span.c0, frozen);
    const after = (offsets[n] ?? 0) - (offsets[Math.min(span.c1, n)] ?? 0);
    return [px(gutter), ...track.map((t) => px(t === 'skip' ? (offsets[lo] ?? 0) - (offsets[frozen] ?? 0) : (shownWidths[t] ?? 0))), ...(after > 0 ? [px(after)] : [])].join(' ');
  });
  const sortable = $derived(!!onsort && !unsortable);
  const linked = $derived(columns.map((_, c) => links?.(c) ?? null));
  const stickAt = (p: number) => gutter + (offsets[p] ?? 0);
  const colAt = (p: number) => cols[p] ?? { name: '', type_name: null };
  const focus = $derived(sel.focus);
  const gid = $props.id();
  const fullCols = $derived.by(() => {
    const out = new Set<number>();
    for (const x of sel.ranges) if (x.r0 === 0 && x.r1 === count - 1) for (let c = x.c0; c <= x.c1; c++) out.add(c);
    return out;
  });
  const fullRows = $derived.by(() => sel.ranges.filter((x) => x.c0 === 0 && x.c1 === columns.length - 1));

  const MEASURED_PER_COLUMN = 12;
  const ctxs = new Map<string, CanvasRenderingContext2D | null>();
  function textWidth(s: string, font: string) {
    let ctx = ctxs.get(font);
    if (ctx === undefined) {
      ctx = document.createElement('canvas').getContext('2d');
      if (ctx) ctx.font = font;
      ctxs.set(font, ctx);
    }
    return ctx ? ctx.measureText(clipCell(s)).width : s.length * 7;
  }

  function fonts() {
    const mono = getComputedStyle(scroller).fontFamily;
    const body = `400 12.5px ${mono}`;
    return { body, head: `600 12px ${getComputedStyle(document.body).fontFamily}`, type: `400 10.5px ${mono}`, was: `400 11px ${mono}`, digit: textWidth('0', body) };
  }

  function widest(sample: readonly (Value | undefined)[][], c: number): number[] {
    const top: number[] = [];
    const len: number[] = [];
    for (let i = 0; i < sample.length; i++) {
      const n = (sample[i]?.[c] ?? 'NULL').length + (edit?.cell(i, c)?.was?.length ?? 0);
      if (top.length === MEASURED_PER_COLUMN && n <= (len.at(-1) ?? 0)) continue;
      let at = len.length;
      while (at > 0 && (len[at - 1] ?? 0) < n) at--;
      top.splice(at, 0, i);
      len.splice(at, 0, n);
      if (top.length > MEASURED_PER_COLUMN) {
        top.pop();
        len.pop();
      }
    }
    return top;
  }

  function measure(c: number, sample: readonly (Value | undefined)[][], cap: number, f = fonts(), rough = false) {
    const col = columns[c];
    if (!col) return 100;
    if (rough) {
      let chars = 4;
      for (const row of sample) chars = Math.max(chars, row[c]?.length ?? 4);
      const head = col.name.length * 7.4 + (col.type_name ? col.type_name.length * 6 + 6 : 0) + 24 + (links?.(c) ? 17 : 0);
      return Math.round(Math.min(cap, Math.max(56, head, chars * f.digit + 20)));
    }
    let w = textWidth(col.name, f.head) + (col.type_name ? textWidth(col.type_name, f.type) + 6 : 0) + 24 + (links?.(c) ? 17 : 0);
    for (const i of widest(sample, c)) {
      if (w >= cap) break;
      const was = edit?.cell(i, c)?.was;
      const extra = was === undefined ? 0 : textWidth(was ?? 'NULL', f.was) + 36;
      w = Math.max(w, textWidth(sample[i]?.[c] ?? 'NULL', f.body) + 20 + extra);
    }
    return Math.round(Math.min(cap, Math.max(56, w)));
  }

  $effect(() => {
    if (columns.length === 0 || count === 0 || measuredFor === result) return;
    measuredFor = result;
    if (measuredColumns === columns && widths.length === columns.length) return;
    measuredColumns = columns;
    const sample = result.rows.slice(0, 200);
    const f = fonts();
    const saved = loadLayout(layout).widths;
    widths = columns.map((col, i) => saved[col.name] ?? Math.max(grown[i] ?? 0, measure(i, sample, 360, f, i >= COLUMN_WINDOW_FROM)));
  });

  $effect(() => {
    if (widths.length !== columns.length) widths = columns.map((c) => Math.max(64, c.name.length * 8 + 34));
  });

  let arrangedFor: unknown = null;
  $effect(() => {
    if (arrangedFor === columns && order.length === columns.length) return;
    arrangedFor = columns;
    ({ order, frozen } = arrange(columns.map((c) => c.name), loadLayout(layout)));
  });

  function remember(sized: number[] = []) {
    if (!layout) return;
    const was = loadLayout(layout);
    const keep = { ...was.widths };
    for (const i of sized) {
      const col = columns[i];
      if (col) keep[col.name] = widths[i] ?? 100;
    }
    saveLayout(layout, { order: order.map((i) => columns[i]?.name ?? ''), frozen: order.slice(0, frozen).map((i) => columns[i]?.name ?? ''), widths: keep });
  }

  $effect(() => {
    void result;
    editing = null;
    sel = S.EMPTY;
    scrollTop = 0;
    if (scroller) scroller.scrollTop = 0;
  });

  $effect(() => {
    const f = sel.focus;
    if (f) edit?.focus(f.r, dc(f.c));
    onfocus?.(f && { r: f.r, c: dc(f.c) });
  });

  const statsOwner = {};
  $effect(() => {
    const s = sel;
    void result.rev;
    void partial;
    void summarize;
    const t = setTimeout(() => {
      const hints = columns.map((col, c) => numericHint(col.type_name ?? null, numeric[c] ?? false));
      const picked = s.ranges.length ? S.byColumn(s, shown).map((x) => ({ c: dc(x.c), numeric: hints[dc(x.c)] ?? null, values: x.values })) : null;
      const whole = partial && summarize && picked ? S.wholeColumns(s, { rows: result.count, cols: columns.length }) : null;
      const run = summarize;
      const wider =
        whole && run && picked
          ? { loaded: result.count, run: () => run(picked.map((x) => ({ name: columns[x.c]?.name ?? '', numeric: x.numeric ?? x.values.every((v) => v === null || !Number.isNaN(Number(v))) }))) }
          : null;
      publish(statsOwner, picked, wider);
    }, 60);
    return () => clearTimeout(t);
  });
  $effect(() => () => retract(statsOwner));

  $effect(() => {
    const e = editing;
    const input = editInput;
    onedit?.(e && input ? { r: e.r, c: dc(e.c), value: e.value, input } : null);
  });

  export function setEditValue(value: string) {
    if (editing) editing.value = value;
  }

  export function commitEdit(step?: [number, number]) {
    finish(true, step);
  }

  $effect(() => {
    const ro = new ResizeObserver(([e]) => {
      if (!e) return;
      viewport = e.contentRect.height;
      viewWidth = e.contentRect.width;
    });
    ro.observe(scroller);
    return () => ro.disconnect();
  });

  const isSel = (r: number, c: number) => S.selected(sel, r, c);

  const zoom = () => Number.parseFloat(getComputedStyle(document.documentElement).zoom) || 1;

  type Spot = { area: 'head' | 'gutter' | 'cell' | 'corner'; r: number; c: number };

  const contentX = (vx: number) => {
    const x = vx - gutter;
    return x < (offsets[frozen] ?? 0) ? x : x + scroller.scrollLeft;
  };

  function columnAt(x: number): number {
    let lo = 0;
    let hi = widths.length - 1;
    if (x <= 0) return 0;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if ((offsets[mid] ?? 0) <= x) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  }

  function spotAt(clientX: number, clientY: number): Spot {
    const rect = scroller.getBoundingClientRect();
    const z = zoom();
    const vx = (clientX - rect.left) / z;
    const vy = (clientY - rect.top) / z;
    const r = Math.min(count - 1, Math.max(0, Math.floor((vy - HEAD + position) / ROW)));
    const c = Math.min(columns.length - 1, columnAt(contentX(vx)));
    const inHead = vy < HEAD && vy >= 0;
    const inGutter = vx < gutter && vx >= 0;
    return { area: inHead && inGutter ? 'corner' : inHead ? 'head' : inGutter ? 'gutter' : 'cell', r, c };
  }

  const mods = (e: MouseEvent): S.Mods => ({ shift: e.shiftKey, toggle: e.ctrlKey || e.metaKey });

  type Drag = { kind: 'cells' | 'rows' | 'cols'; x: number; y: number; x0: number; start: Spot; moved: boolean; move: boolean; click: MouseEvent | null };
  let drag = $state.raw<Drag | null>(null);
  let frame = 0;

  function follow() {
    const d = drag;
    if (!d) return;
    const s = spotAt(d.x, d.y);
    if (d.kind === 'cells') {
      if (s.r !== sel.focus?.r || s.c !== sel.focus.c) sel = S.extendTo(sel, { r: s.r, c: s.c });
    } else if (d.kind === 'rows') {
      if (s.r !== sel.focus?.r) sel = S.dragRows(sel, s.r, bounds);
    } else if (d.move) {
      if (!d.moved && Math.abs(d.x - d.x0) < 5) return;
      d.moved = true;
      const x = contentX((d.x - scroller.getBoundingClientRect().left) / zoom());
      const mid = ((offsets[s.c] ?? 0) + (offsets[s.c + 1] ?? 0)) / 2;
      moving = { from: d.start.c, to: x > mid ? s.c + 1 : s.c };
    } else if (s.c !== d.start.c || d.moved) {
      if (!d.moved) {
        d.moved = true;
        sel = S.clickColumn(sel, d.start.c, bounds, { toggle: !!d.click && (d.click.ctrlKey || d.click.metaKey) });
      }
      if (s.c !== sel.focus?.c) sel = S.dragColumns(sel, s.c, bounds);
    }
  }

  function autoscroll() {
    const d = drag;
    if (!d) return;
    const rect = scroller.getBoundingClientRect();
    const z = zoom();
    const speed = (over: number) => Math.min(48, Math.max(2, over * 0.6));
    let dy = 0;
    let dx = 0;
    if (d.kind !== 'cols') {
      const top = rect.top + HEAD * z;
      if (d.y < top + EDGE) dy = -speed(top + EDGE - d.y);
      else if (d.y > rect.bottom - EDGE) dy = speed(d.y - rect.bottom + EDGE);
    }
    if (d.kind !== 'rows') {
      const left = rect.left + gutter * z;
      if (d.x < left + EDGE) dx = -speed(left + EDGE - d.x);
      else if (d.x > rect.right - EDGE) dx = speed(d.x - rect.right + EDGE);
    }
    if (dy) scroller.scrollTop += dy / ratio;
    if (dx) scroller.scrollLeft += dx;
    if (dx || dy) follow();
    frame = requestAnimationFrame(autoscroll);
  }

  function startDrag(e: PointerEvent, kind: Drag['kind'], start: Spot) {
    drag = { kind, x: e.clientX, y: e.clientY, x0: e.clientX, start, moved: false, move: kind === 'cols' && !e.shiftKey && !e.ctrlKey && !e.metaKey, click: kind === 'cols' ? e : null };
    const move = (ev: PointerEvent) => {
      if (!drag) return;
      drag.x = ev.clientX;
      drag.y = ev.clientY;
      follow();
    };
    const up = (ev: PointerEvent) => {
      const d = drag;
      drag = null;
      cancelAnimationFrame(frame);
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', up);
      const to = moving?.to;
      moving = null;
      if (d?.move && d.moved && to !== undefined && ev.type === 'pointerup') {
        order = moveColumn(order, frozen, d.start.c, to);
        sel = S.EMPTY;
        remember();
      } else if (d?.kind === 'cols' && !d.moved && d.click && ev.type === 'pointerup') headClick(d.start.c, d.click);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    window.addEventListener('pointercancel', up);
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(autoscroll);
  }

  function onpointerdown(e: PointerEvent) {
    if (e.target instanceof Element && e.target.closest('.th-resize, .cell-input, .grid-find')) return;
    if (onScrollbar(scroller, e.clientX, e.clientY)) return;
    const s = spotAt(e.clientX, e.clientY);
    if (e.target instanceof Element && e.target.closest('.fk-go') && onfollow && (e.button === 0 || e.button === 1)) {
      e.preventDefault();
      if (editing) finish(true);
      sel = S.clickCell(S.EMPTY, s);
      scroller.focus({ preventScroll: true });
      onfollow(s.r, dc(s.c), e.button === 1 || e.ctrlKey || e.metaKey);
      return;
    }
    if (s.area === 'corner') {
      if (e.button === 0) sel = S.all(bounds);
      e.preventDefault();
      scroller.focus({ preventScroll: true });
      return;
    }
    if (e.button === 2) {
      if (s.area === 'cell' && !isSel(s.r, s.c)) sel = S.clickCell(sel, s);
      if (s.area === 'gutter' && !fullRows.some((x) => s.r >= x.r0 && s.r <= x.r1)) sel = S.clickRow(sel, s.r, bounds);
      return;
    }
    if (e.button !== 0 || count === 0 && s.area !== 'head') return;
    e.preventDefault();
    if (editing) finish(true);
    scroller.focus({ preventScroll: true });
    if (s.area === 'head') {
      if (e.ctrlKey || e.metaKey) {
        sel = S.clickColumn(sel, s.c, bounds, { toggle: true });
        startDrag(e, 'cols', s);
        if (drag) drag.moved = true;
        return;
      }
      startDrag(e, 'cols', s);
      return;
    }
    if (s.area === 'gutter') {
      sel = S.clickRow(sel, s.r, bounds, mods(e));
      startDrag(e, 'rows', s);
      return;
    }
    sel = S.clickCell(sel, s, mods(e));
    startDrag(e, 'cells', s);
  }

  function headClick(c: number, e: MouseEvent) {
    if (sortable && onsort) return onsort(dc(c), e.shiftKey);
    sel = S.clickColumn(sel, c, bounds, { shift: e.shiftKey });
    if (unsortable) onblocked?.();
  }

  function ondblclick(e: MouseEvent) {
    if (e.target instanceof Element && e.target.closest('.cell-input, .th-resize')) return;
    const s = spotAt(e.clientX, e.clientY);
    if (s.area === 'cell') begin(s.r, s.c);
  }

  export function select(r: number, column: number) {
    if (!count || !columns.length) return;
    const at = order.indexOf(column);
    const c = at < 0 ? column : at;
    if (editing) finish(true);
    sel = S.clickCell(S.EMPTY, { r: Math.min(count - 1, Math.max(0, r)), c: Math.min(columns.length - 1, Math.max(0, c)) });
    if (sel.focus) {
      reveal(sel.focus.r);
      revealColumn(sel.focus.c);
    }
  }

  export function takeFocus() {
    scroller.focus({ preventScroll: true });
  }

  export function value(r: number, c: number): string | null {
    const v = edit?.cell(r, c);
    return v ? v.value : (result.rows[r]?.[c] ?? null);
  }

  const shown = (r: number, p: number) => value(r, dc(p));

  let masked = $state.raw<(string | null)[]>([]);
  let revealed = $state.raw(new Set<number>());
  const hasRows = $derived(count > 0);
  const hidden = (c: number) => !!masked[c] && !revealed.has(c);
  const KIND: Record<string, () => string> = { cf: m.sec_d_cf, iban: m.sec_d_iban, email: m.sec_d_email, phone: m.sec_d_phone };

  $effect(() => {
    const on = prefs.value.results.mask_sensitive || presentation.on;
    const cs = columns;
    void hasRows;
    revealed = new Set();
    if (!on || !cs.length) {
      masked = [];
      return;
    }
    let live = true;
    const sample = result.rows.slice(0, 50);
    void api
      .sensitiveColumns(cs.map((col, i) => ({ name: col.name, values: sample.flatMap((row) => row[i] ?? []).slice(0, 20) })))
      .then((kinds) => live && (masked = kinds))
      .catch(() => {});
    return () => {
      live = false;
    };
  });

  function toggleMask(c: number) {
    const next = new Set(revealed);
    if (next.has(c)) next.delete(c);
    else next.add(c);
    revealed = next;
  }

  const MAX_HITS = 100_000;
  let finding = $state(false);
  let needle = $state('');
  let findInput = $state<HTMLInputElement | null>(null);
  let hits = $state.raw<S.Cell[]>([]);
  let hitKeys = $state.raw(new Set<number>());
  let hitAt = $state(-1);

  $effect(() => {
    const q = needle.trim().toLowerCase();
    void result.rev;
    void count;
    void order;
    if (!finding || !q) {
      hits = [];
      hitKeys = new Set();
      hitAt = -1;
      return;
    }
    const t = setTimeout(() => {
      const found: S.Cell[] = [];
      const keys = new Set<number>();
      const n = columns.length;
      scan: for (let r = 0; r < count; r++)
        for (let p = 0; p < n; p++) {
          if (hidden(dc(p))) continue;
          const v = value(r, dc(p));
          if (v === null || !v.toLowerCase().includes(q)) continue;
          found.push({ r, c: p });
          keys.add(r * n + dc(p));
          if (found.length >= MAX_HITS) break scan;
        }
      hits = found;
      hitKeys = keys;
      hitAt = -1;
      if (found.length) goHit(0, false);
    }, 120);
    return () => clearTimeout(t);
  });

  function goHit(i: number, focusGrid = true) {
    if (!hits.length) return;
    hitAt = (i + hits.length) % hits.length;
    const h = hits[hitAt];
    if (!h) return;
    if (editing) finish(true);
    sel = S.clickCell(S.EMPTY, h);
    reveal(h.r);
    revealColumn(h.c);
    if (focusGrid) findInput?.focus();
  }

  async function openFind() {
    finding = true;
    await tick();
    findInput?.focus();
    findInput?.select();
  }

  function closeFind() {
    finding = false;
    scroller.focus({ preventScroll: true });
  }

  function findKey(e: KeyboardEvent) {
    e.stopPropagation();
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === 'Enter' || e.key === 'F3') {
      e.preventDefault();
      goHit(hitAt + (e.shiftKey ? -1 : 1));
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeFind();
    } else if (mod && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      findInput?.select();
    }
  }

  function begin(r: number, c: number, initial?: string) {
    if (editing?.r === r && editing.c === c) return;
    if (!edit || !edit.start(r, dc(c))) return;
    sel = S.clickCell(S.EMPTY, { r, c });
    const now = shown(r, c) ?? '';
    const huge = initial === undefined && now.length > HUGE_EDIT && !isBinaryType(cols[c]?.type_name ?? '');
    editing = huge ? { r, c, value: '', huge: now.length } : { r, c, value: initial ?? now };
  }

  function finish(save: boolean, step?: [number, number], blur = false) {
    const e = editing;
    if (!e) return;
    editing = null;
    if (save && !(e.huge && e.value === '')) {
      if (edit?.commit(e.r, dc(e.c), e.value) === false && !blur) {
        editing = e;
        return;
      }
      fit(e.r, e.c);
    }
    if (step) {
      sel = S.move(S.clickCell(S.EMPTY, e), step[0], step[1], bounds);
      if (sel.focus) reveal(sel.focus.r);
    }
    scroller.focus({ preventScroll: true });
  }

  function fit(r: number, p: number) {
    const c = dc(p);
    const cv = edit?.cell(r, c);
    if (!cv) return;
    const font = getComputedStyle(scroller).fontFamily;
    const was = cv.was === undefined ? 0 : textWidth(cv.was ?? 'NULL', `400 11px ${font}`) + 36;
    const need = Math.round(Math.min(480, textWidth(cv.value ?? 'NULL', `600 12.5px ${font}`) + was + 26));
    if (grownFor !== columns) {
      grown = [];
      grownFor = columns;
    }
    grown[c] = Math.max(grown[c] ?? 0, need);
    if (need > (widths[c] ?? 0)) widths[c] = need;
  }

  function editorKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (editkey?.(e)) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      finish(false);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      finish(true, [e.shiftKey ? -1 : 1, 0]);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      finish(true, [0, e.shiftKey ? -1 : 1]);
    }
  }

  function autofocus(node: HTMLInputElement) {
    node.focus();
    const end = node.value.length;
    node.setSelectionRange(end, end);
  }

  const sortOf = (p: number) => sort.findIndex((k) => k.column === dc(p));

  function reveal(r: number) {
    const top = r * ROW;
    const pos = scroller.scrollTop * ratio;
    if (top < pos) scroller.scrollTop = top / ratio;
    else if (top + 2 * ROW > pos + viewport) scroller.scrollTop = (top + 2 * ROW - viewport) / ratio;
  }

  function revealColumn(c: number) {
    if (c < frozen) return;
    const pinned = offsets[frozen] ?? 0;
    const x0 = (offsets[c] ?? 0) - pinned;
    const x1 = (offsets[c + 1] ?? 0) - pinned;
    const room = scroller.clientWidth - gutter - pinned;
    if (x0 < scroller.scrollLeft) scroller.scrollLeft = x0;
    else if (x1 > scroller.scrollLeft + room) scroller.scrollLeft = x1 - room;
  }

  export function selection(): { columns: typeof columns; rows: Value[][] } {
    if (!sel.ranges.length) return { columns, rows: result.rows };
    const p = S.project(sel, shown);
    return { columns: p.cols.flatMap((c) => cols[c] ?? []), rows: p.cells.map((row) => row.map((v) => v ?? null)) };
  }

  export async function copy(format: CopyFormat) {
    const s = selection();
    await copyText(render(format, s.columns, s.rows, { table: tableFromSql(result.sql), dialect, header: format !== 'tsv' || s.rows.length === result.rows.length }));
  }

  async function copyColumn(c: number) {
    const lines: string[] = [];
    for (let r = 0; r < count; r++) lines.push(shown(r, c) ?? '');
    await copyText(lines.join('\n'));
  }

  function autofit(c: number) {
    widths[c] = measure(c, result.rows.slice(0, 2000), 720);
    remember([c]);
  }

  function autofitAll() {
    const sample = result.rows.slice(0, 2000);
    const f = fonts();
    widths = columns.map((_, i) => measure(i, sample, 720, f, i >= COLUMN_WINDOW_FROM));
    remember(columns.map((_, i) => i));
  }

  function freeze(p: number) {
    ({ order, frozen } = toggleFrozen(order, frozen, p));
    sel = S.EMPTY;
    remember();
  }

  function resetLayout() {
    saveLayout(layout, { order: [], frozen: [], widths: {} });
    order = columns.map((_, i) => i);
    frozen = 0;
    grown = [];
    const sample = result.rows.slice(0, 200);
    const f = fonts();
    widths = columns.map((_, i) => measure(i, sample, 360, f, i >= COLUMN_WINDOW_FROM));
    sel = S.EMPTY;
  }

  function selectedCells(): S.Cell[] {
    const out: S.Cell[] = [];
    for (const x of sel.ranges)
      for (let r = x.r0; r <= x.r1; r++)
        for (let c = x.c0; c <= x.c1; c++) {
          if (out.length >= 10_000) return out;
          out.push({ r, c });
        }
    return out;
  }

  function setNull() {
    if (!edit) return;
    for (const { r, c } of selectedCells()) edit.setNull(r, dc(c));
  }

  function toggleDelete() {
    if (!edit) return;
    const spans = S.rowSpans(sel);
    if (!spans.length && focus) spans.push([focus.r, focus.r]);
    for (const [a, b] of spans) edit.toggleDelete(a, b);
  }

  function editKey(e: KeyboardEvent): boolean {
    if (!edit || edit.readonly || !focus) return false;
    const mod = e.ctrlKey || e.metaKey;
    const { r, c } = focus;
    if (mod && (e.key === 'Delete' || e.key === 'Backspace')) toggleDelete();
    else if (e.key === 'Delete') setNull();
    else if (e.key === 'Enter' || e.key === 'F2') begin(r, c);
    else if (e.key === 'Backspace') begin(r, c, '');
    else if (e.key.length === 1 && !mod && !e.altKey && e.key !== ' ') begin(r, c, e.key);
    else return false;
    e.preventDefault();
    return true;
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      void openFind();
      return;
    }
    if (e.key === 'F3' && hits.length) {
      e.preventDefault();
      goHit(hitAt + (e.shiftKey ? -1 : 1), false);
      return;
    }
    if (e.key === 'F4' && focus && onfollow) {
      e.preventDefault();
      onfollow(focus.r, focus.c, e.shiftKey);
      return;
    }
    if (gridkey?.(e) || editKey(e)) return;
    const mod = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (mod && key === 'a') {
      e.preventDefault();
      sel = S.all(bounds);
      return;
    }
    if (mod && key === 'c') {
      e.preventDefault();
      void copy('tsv');
      return;
    }
    if (e.key === ' ' && (e.shiftKey || mod) && focus) {
      e.preventDefault();
      sel = e.shiftKey ? S.clickRow(sel, focus.r, bounds) : S.clickColumn(sel, focus.c, bounds);
      return;
    }
    if (e.key === 'Escape' && focus && sel.ranges.length) {
      sel = S.clickCell(S.EMPTY, focus);
      return;
    }
    const page = Math.max(1, Math.floor(viewport / ROW) - 1);
    const far = 1e9;
    const steps: Record<string, [number, number]> = {
      ArrowUp: [mod ? -far : -1, 0],
      ArrowDown: [mod ? far : 1, 0],
      ArrowLeft: [0, mod ? -far : -1],
      ArrowRight: [0, mod ? far : 1],
      PageUp: [-page, 0],
      PageDown: [page, 0],
      Home: mod ? [-far, -far] : [0, -far],
      End: mod ? [far, far] : [0, far],
    };
    const step = steps[e.key];
    if (!step || !count || e.altKey) return;
    e.preventDefault();
    sel = S.move(sel, step[0], step[1], bounds, e.shiftKey);
    if (sel.focus) {
      reveal(sel.focus.r);
      revealColumn(sel.focus.c);
    }
  }

  function resize(e: PointerEvent, p: number) {
    e.preventDefault();
    e.stopPropagation();
    const c = dc(p);
    const start = e.clientX;
    const w0 = widths[c] ?? 100;
    const z = zoom();
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    const onmove = (ev: PointerEvent) => {
      widths[c] = Math.max(48, Math.round(w0 + (ev.clientX - start) / z));
    };
    const onup = () => {
      target.removeEventListener('pointermove', onmove);
      target.removeEventListener('pointerup', onup);
      remember([c]);
    };
    target.addEventListener('pointermove', onmove);
    target.addEventListener('pointerup', onup);
  }

  const copyAs = (): MenuEntry => ({
    label: m.grid_copy_as(),
    icon: ClipboardCopy,
    items: [
      { label: m.copy_tsv_header(), run: () => copyWithHeader() },
      { label: m.copy_csv(), run: () => copy('csv') },
      { label: m.copy_json(), run: () => copy('json') },
      insertable(dialect) && { label: m.copy_insert(), run: () => copy('insert') },
    ],
  });

  async function copyWithHeader() {
    const s = selection();
    await copyText(render('tsv', s.columns, s.rows, { header: true }));
  }

  function sortItems(c: number): MenuEntry[] {
    if (!onsort) return [];
    const k = sort[sortOf(c)];
    const blocked = unsortable ?? undefined;
    return [
      'separator',
      { label: m.grid_sort_asc(), icon: ArrowUpNarrowWide, disabled: !!blocked, hint: blocked, run: () => onsort(dc(c), false, 'asc') },
      { label: m.grid_sort_desc(), icon: ArrowDownWideNarrow, disabled: !!blocked, run: () => onsort(dc(c), false, 'desc') },
      !!k && { label: m.grid_sort_off(), icon: X, run: () => onsort(dc(c), true, 'none') },
    ];
  }

  function deleteItem(r: number): MenuEntry {
    if (!edit || edit.readonly) return null;
    const state = edit.row(r)?.state;
    const spans = S.rowSpans(sel);
    const locked = !!edit.lock?.();
    const many = spans.length > 1 || spans.some(([a, b]) => b > a);
    return {
      label: state === 'deleted' ? m.grid_restore_row() : state === 'new' ? m.grid_drop_row() : many ? m.grid_delete_rows() : m.grid_delete_row(),
      icon: Trash2,
      shortcut: keys(`Mod+${m.key_delete()}`),
      danger: state !== 'deleted' && !locked,
      disabled: locked,
      run: toggleDelete,
    };
  }

  function lockNote(): MenuEntry {
    const lock = edit && !edit.readonly ? edit.lock?.() : null;
    return !!lock && { label: lock, icon: Lock, disabled: true };
  }

  function editItems(r: number, c: number): MenuEntry[] {
    if (!edit || edit.readonly) return [];
    const lock = !!edit.lock?.();
    return [
      'separator',
      lockNote(),
      { label: m.grid_edit_cell(), icon: Pencil, shortcut: 'F2', disabled: lock, run: () => begin(r, c) },
      { label: m.grid_set_null(), icon: Eraser, shortcut: m.key_delete(), disabled: lock, run: setNull },
      deleteItem(r),
    ];
  }

  function oncontextmenu(e: MouseEvent) {
    if (e.target instanceof Element && e.target.closest('.cell-input')) return;
    const keyboard = e.clientX === 0 && e.clientY === 0;
    let s: Spot;
    if (keyboard) {
      const f = focus ?? { r: 0, c: 0 };
      const rect = scroller.getBoundingClientRect();
      const z = zoom();
      s = { area: 'cell', ...f };
      e.preventDefault();
      const x = rect.left + (gutter + (offsets[f.c] ?? 0) - scroller.scrollLeft + 12) * z;
      const y = rect.top + (HEAD + f.r * ROW - position + ROW) * z;
      return open({ x, y }, s);
    }
    s = spotAt(e.clientX, e.clientY);
    open(e, s);
  }

  function linkItems(r: number, c: number): MenuEntry[] {
    const items = cellmenu?.(r, dc(c)) ?? [];
    return items.length ? ['separator', ...items] : [];
  }

  function open(at: MouseEvent | { x: number; y: number }, s: Spot) {
    const col = cols[s.c];
    if (!col) return openMenu(at, []);
    const n = S.count(sel);
    if (s.area === 'head' || s.area === 'corner') {
      return openMenu(at, [
        { label: m.grid_select_column(), run: () => (sel = S.clickColumn(sel, s.c, bounds)) },
        { label: m.grid_copy_name(), hint: col.name, run: () => copyText(col.name) },
        { label: m.grid_copy_column(), run: () => copyColumn(s.c) },
        ...sortItems(s.c),
        'separator',
        !!masked[dc(s.c)] && { label: hidden(dc(s.c)) ? m.grid_mask_show() : m.grid_mask_hide(), icon: hidden(dc(s.c)) ? Eye : EyeOff, run: () => toggleMask(dc(s.c)) },
        { label: s.c < frozen ? m.grid_unfreeze() : m.grid_freeze(), icon: s.c < frozen ? PinOff : Pin, run: () => freeze(s.c) },
        { label: m.grid_autofit(), icon: MoveHorizontal, run: () => autofit(dc(s.c)) },
        { label: m.grid_autofit_all(), icon: MoveHorizontal, run: autofitAll },
        !!layout && { label: m.grid_layout_reset(), icon: RotateCcw, run: resetLayout },
      ]);
    }
    if (s.area === 'gutter') {
      return openMenu(at, [
        { label: m.grid_copy(), icon: ClipboardCopy, shortcut: keys('Mod+C'), run: () => copy('tsv') },
        copyAs(),
        'separator',
        { label: m.grid_select_all(), shortcut: keys('Mod+A'), run: () => (sel = S.all(bounds)) },
        !!edit && !edit.readonly && 'separator',
        lockNote(),
        deleteItem(s.r),
        extra.length > 0 && 'separator',
        ...extra,
      ]);
    }
    const value = shown(s.r, s.c);
    openMenu(at, [
      { label: n > 1 ? m.grid_copy_cells({ count: n }) : m.grid_copy(), icon: ClipboardCopy, shortcut: keys('Mod+C'), run: () => copy('tsv') },
      copyAs(),
      { label: m.grid_copy_column(), hint: col.name, run: () => copyColumn(s.c) },
      ...sortItems(s.c),
      !!onfilter && { label: m.grid_filter_value(), icon: Filter, hint: value === null ? 'NULL' : value.length > 24 ? `${value.slice(0, 24)}…` : value, run: () => onfilter(dc(s.c), value) },
      ...linkItems(s.r, s.c),
      ...editItems(s.r, s.c),
      'separator',
      { label: m.grid_select_all(), shortcut: keys('Mod+A'), run: () => (sel = S.all(bounds)) },
      extra.length > 0 && 'separator',
      ...extra,
    ]);
  }
</script>

<div class="grid-wrap">
<div
  class="grid"
  class:dragging={!!drag}
  style:--g-row="{ROW}px"
  style:--g-head="{HEAD}px"
  bind:this={scroller}
  onscroll={() => {
    scrollTop = scroller.scrollTop;
    scrollLeft = scroller.scrollLeft;
  }}
  {onkeydown}
  {onpointerdown}
  {ondblclick}
  {oncontextmenu}
  tabindex="0"
  role="grid"
  aria-multiselectable="true"
  aria-rowcount={count + 1}
  aria-colcount={columns.length + 1}
  aria-activedescendant={focus ? `${gid}-cell` : undefined}
  aria-label={m.results_grid()}
>
  <div class="head" style:grid-template-columns={template} role="row">
    <div class="gutter corner" role="columnheader" aria-label={m.grid_select_all()} data-tip={m.grid_select_all()}></div>
    {#each track as t (t)}
      {#if t === 'skip'}<div class="skip" aria-hidden="true"></div>{:else}
      {@const c = t}
      {@const col = colAt(c)}
      {@const sk = sortOf(c)}
      <div
        class="th"
        class:frozen={c < frozen}
        class:edge={c === frozen - 1}
        class:moving={moving?.from === c}
        style:left={c < frozen ? `${stickAt(c)}px` : undefined}
        class:num={numeric[dc(c)]}
        class:sortable
        class:unsortable={!!onsort && !!unsortable}
        class:picked={fullCols.has(c)}
        role="columnheader"
        aria-sort={sk < 0 ? undefined : sort[sk]?.descending ? 'descending' : 'ascending'}
        data-tip={onsort && unsortable ? `${col.type_name ?? ''}${col.type_name ? ' · ' : ''}${unsortable}` : (col.type_name ?? '')}
      >
        <span class="th-name">{col.name}</span>
        {#if linked[dc(c)]}<span class="th-fk" data-tip={linked[dc(c)]}><Link2 size={11} strokeWidth={2} aria-label={linked[dc(c)]} /></span>{/if}
        {#if sk >= 0}<span class="th-sort" aria-hidden="true">{sort[sk]?.descending ? '↓' : '↑'}{sort.length > 1 ? sk + 1 : ''}</span>{/if}
        {#if col.type_name}<span class="th-type">{col.type_name}</span>{/if}
        {#if masked[dc(c)]}<button class="th-mask" type="button" aria-pressed={!hidden(dc(c))} aria-label={hidden(dc(c)) ? m.grid_mask_show() : m.grid_mask_hide()} data-tip={`${KIND[masked[dc(c)] ?? '']?.() ?? ''} · ${hidden(dc(c)) ? m.grid_mask_show() : m.grid_mask_hide()}`} onpointerdown={(e) => e.stopPropagation()} onclick={() => toggleMask(dc(c))}>{#if hidden(dc(c))}<EyeOff size={12} strokeWidth={2} />{:else}<Eye size={12} strokeWidth={2} />{/if}</button>{/if}
        <span class="th-resize" aria-hidden="true" onpointerdown={(e) => resize(e, c)} ondblclick={(e) => (e.stopPropagation(), autofit(dc(c)))}></span>
      </div>
      {/if}
    {/each}
    {#if moving}<div class="th-drop" style:left="{gutter + (offsets[moving.to] ?? 0)}px" aria-hidden="true"></div>{/if}
  </div>
  <div class="body" style:height="{bodyHeight}px">
    <div class="window" style:transform="translateY({windowTop}px)">
      {#each visible as row, k (first + k)}
        {@const r = first + k}
        {@const rv = edit?.row(r) ?? null}
        <div
          class="tr"
          class:odd={r % 2 === 1}
          class:edited={rv?.state === 'edited'}
          class:new={rv?.state === 'new'}
          class:deleted={rv?.state === 'deleted'}
          class:conflict={rv?.mark === 'conflict'}
          class:failed={rv?.mark === 'error'}
          class:picked={fullRows.some((x) => r >= x.r0 && r <= x.r1)}
          style:grid-template-columns={template}
          role="row"
          aria-rowindex={r + 2}
          data-tip={rv?.note ?? undefined}
        >
          <div class="gutter" role="rowheader">
            {#if rv && !edit?.readonly}<span class="mark" data-n={rv.n ?? ''}><Pencil size={11} strokeWidth={2} aria-hidden="true" /></span>{/if}
            <span class="gutter-n">{edit ? edit.label(r) : r + 1}</span>
            {#if rv?.note}<span class="sr-only">{rv.note}</span>{/if}
          </div>
          {#each track as t (t)}
            {#if t === 'skip'}<div class="skip" aria-hidden="true"></div>{:else}
            {@const c = t}
            {@const cell = row[dc(c)] ?? null}
            {@const cv = edit?.cell(r, dc(c)) ?? null}
            {@const value = cv ? cv.value : cell}
            {@const on = isSel(r, c)}
            <div
              class="td"
              class:frozen={c < frozen}
              class:edge={c === frozen - 1}
              style:left={c < frozen ? `${stickAt(c)}px` : undefined}
              class:num={numeric[dc(c)]}
              class:sel={on}
              class:focus={focus?.r === r && focus?.c === c}
              class:changed={cv?.was !== undefined}
              class:applied={!!edit?.applied(r, c)}
              class:editing={editing?.r === r && editing.c === c}
              class:fk={!!linked[dc(c)] && value !== null}
              class:hit={hitKeys.has(r * columns.length + dc(c))}
              class:hit-on={hits[hitAt]?.r === r && hits[hitAt]?.c === c}
              role="gridcell"
              id={focus?.r === r && focus?.c === c ? `${gid}-cell` : undefined}
              aria-colindex={c + 2}
              aria-selected={on}
            >
              {#if editing?.r === r && editing.c === c}
                <input class="cell-input" bind:this={editInput} bind:value={editing.value} placeholder={editing.huge ? m.grid_edit_huge({ size: sizeText(editing.huge, tag()) }) : undefined} data-tip={editing.huge ? m.grid_edit_huge_tip() : undefined} use:autofocus onkeydown={editorKey} onblur={() => finish(true, undefined, true)} aria-label={colAt(c).name} />
              {:else}{#if cv && cv.was !== undefined}<s class="was">{cv.was === null ? nullShown : show(cv.was ?? '', dc(c))}</s>{#if rv?.n}<i class="rev" aria-hidden="true">{rv.n}</i>{/if}{/if}<span class="val">{#if value === null}<span class="null">{cv?.hint ?? nullShown}</span>{:else if hidden(dc(c))}<span class="masked" aria-label={m.grid_masked()}>••••••</span>{:else}{show(value ?? '', dc(c))}{/if}</span>{#if linked[dc(c)] && value !== null && onfollow}<span class="fk-go" aria-hidden="true"><ArrowUpRight size={12} strokeWidth={2} /></span>{/if}{/if}
            </div>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
  </div>
</div>
{#if finding}
  <div class="grid-find" role="search">
    <Search size={13} strokeWidth={1.8} aria-hidden="true" />
    <input bind:this={findInput} bind:value={needle} placeholder={m.grid_find()} aria-label={m.grid_find()} onkeydown={findKey} />
    <span class="grid-find-n" aria-live="polite">{needle.trim() ? (hits.length ? `${hitAt + 1}/${hits.length}${hits.length >= MAX_HITS ? '+' : ''}` : m.grid_find_none()) : ''}</span>
    <button class="icon-btn" type="button" aria-label={m.grid_find_prev()} data-tip={m.grid_find_prev()} disabled={!hits.length} onclick={() => goHit(hitAt - 1)}><ChevronUp size={14} strokeWidth={1.8} /></button>
    <button class="icon-btn" type="button" aria-label={m.grid_find_next()} data-tip={m.grid_find_next()} disabled={!hits.length} onclick={() => goHit(hitAt + 1)}><ChevronDown size={14} strokeWidth={1.8} /></button>
    <button class="icon-btn" type="button" aria-label={m.grid_find_close()} onclick={closeFind}><X size={14} strokeWidth={1.8} /></button>
  </div>
{/if}
</div>

<style>
  .grid-wrap {
    position: relative;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }
  .grid-find {
    position: absolute;
    top: 4px;
    right: 18px;
    z-index: 4;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 4px 3px 9px;
    border: 1px solid var(--nd-rule);
    border-radius: var(--nd-radius);
    background: var(--nd-raised);
    color: var(--nd-ink-2);
    box-shadow: 0 4px 14px rgb(var(--nd-shade) / 0.18);
  }
  .grid-find input {
    width: 180px;
    border: 0;
    outline: none;
    background: none;
    font: 400 12.5px var(--nd-font-ui);
    color: var(--nd-ink);
  }
  .th-mask {
    display: inline-grid;
    place-items: center;
    align-self: center;
    flex: none;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--nd-ink-3);
    cursor: var(--nd-cur-pointer, pointer);
  }
  .th-mask:hover,
  .th-mask[aria-pressed='true'] {
    color: var(--nd-accent-text);
  }
  .masked {
    color: var(--nd-ink-3);
    letter-spacing: 0.1em;
  }
  .grid-find-n {
    min-width: 56px;
    font: 400 11px var(--nd-font-mono);
    font-variant-numeric: tabular-nums;
    text-align: right;
    color: var(--nd-ink-3);
  }
  .td.hit {
    background-image: linear-gradient(color-mix(in srgb, var(--nd-warn) 24%, transparent), color-mix(in srgb, var(--nd-warn) 24%, transparent));
  }
  .td.hit-on {
    background-image: linear-gradient(color-mix(in srgb, var(--nd-warn) 48%, transparent), color-mix(in srgb, var(--nd-warn) 48%, transparent));
  }
  .grid {
    --sel-tint: color-mix(in srgb, var(--nd-focus) 10%, transparent);
    position: relative;
    height: 100%;
    overflow: auto;
    outline: none;
    font: 400 12.5px/var(--g-row) var(--nd-font-mono);
    color: var(--nd-ink);
    contain: strict;
    user-select: none;
    -webkit-user-select: none;
  }
  .grid:focus-visible {
    outline: 2px solid var(--nd-focus);
    outline-offset: -2px;
  }
  .head {
    position: sticky;
    top: 0;
    z-index: 2;
    display: grid;
    width: max-content;
    min-width: 100%;
    height: var(--g-head);
    background: var(--nd-surface-2);
    box-shadow: inset 0 -1px 0 var(--nd-rule), 0 -2px 0 var(--nd-surface-2);
  }
  .th {
    position: relative;
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 0 10px;
    overflow: hidden;
    font: 600 12px/var(--g-head) var(--nd-font-ui);
    color: var(--nd-ink);
    white-space: nowrap;
    cursor: var(--nd-cur-default, default);
    box-shadow: inset -1px 0 0 var(--nd-rule);
  }
  .th.num {
    flex-direction: row-reverse;
    justify-content: flex-start;
  }
  .th-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .th-type {
    font: 400 10.5px var(--nd-font-mono);
    color: var(--nd-ink-3);
  }
  .th.frozen,
  .td.frozen {
    position: sticky;
    z-index: 1;
  }
  .th.frozen {
    background: var(--nd-surface-2);
  }
  .td.frozen {
    background-color: var(--nd-surface);
  }
  .tr.odd .td.frozen:not(.sel) {
    background-color: color-mix(in srgb, var(--nd-ink) 2.2%, var(--nd-surface));
  }
  .th.edge,
  .td.edge {
    box-shadow: inset -2px 0 0 var(--nd-rule);
  }
  .th.moving {
    opacity: 0.55;
  }
  .th-drop {
    position: absolute;
    top: 3px;
    bottom: 3px;
    width: 2px;
    margin-left: -1px;
    background: var(--nd-focus);
    z-index: 3;
    pointer-events: none;
  }
  .th-resize {
    position: absolute;
    top: 0;
    right: -3px;
    width: 7px;
    height: 100%;
    cursor: var(--nd-cur-col-resize, col-resize);
    z-index: 1;
  }
  .body {
    position: relative;
    width: max-content;
    min-width: 100%;
  }
  .window {
    position: absolute;
    inset: 0 auto auto 0;
    min-width: 100%;
    will-change: transform;
  }
  .tr {
    display: grid;
    height: var(--g-row);
  }
  .tr.odd {
    background: color-mix(in srgb, var(--nd-ink) 2.2%, transparent);
  }
  .td {
    position: relative;
    padding: 0 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
    box-shadow: inset -1px 0 0 color-mix(in srgb, var(--nd-rule) 55%, transparent);
    cursor: var(--nd-cur-cell, cell);
  }
  .grid.dragging .td {
    cursor: var(--nd-cur-cell, cell);
  }
  .td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .td.sel,
  .th.picked {
    background-color: var(--nd-sel);
    background-image: linear-gradient(var(--sel-tint), var(--sel-tint));
  }
  .td.focus {
    box-shadow: inset 0 0 0 2px var(--nd-focus);
  }
  .th-fk {
    display: inline-flex;
    align-self: center;
    flex: none;
    color: var(--nd-ink-3);
  }
  .td.fk .val {
    text-decoration: underline dotted color-mix(in srgb, var(--nd-ink-3) 70%, transparent);
    text-underline-offset: 3px;
  }
  .fk-go {
    --fk: min(17px, calc(var(--g-row) - 8px));
    position: absolute;
    top: 50%;
    right: 5px;
    display: grid;
    place-items: center;
    width: var(--fk);
    height: var(--fk);
    margin-top: calc(var(--fk) / -2);
    border-radius: 4px;
    background: var(--nd-raised);
    color: var(--nd-ink-2);
    box-shadow: 0 0 0 1px var(--nd-rule);
    cursor: var(--nd-cur-pointer, pointer);
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .td.num .fk-go {
    right: auto;
    left: 5px;
  }
  .td.fk:hover .fk-go,
  .td.fk.focus .fk-go {
    opacity: 1;
  }
  .fk-go:hover {
    color: var(--nd-accent-text);
    box-shadow: 0 0 0 1px currentColor;
  }
  @media (prefers-reduced-motion: reduce) {
    .fk-go {
      transition: none;
    }
  }
  .null {
    color: var(--nd-ink-3);
    font-style: italic;
    font-size: 11.5px;
  }
  .gutter {
    position: sticky;
    left: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    padding: 0 8px;
    text-align: right;
    color: var(--nd-ink-3);
    font-size: 11px;
    background: var(--nd-surface);
    box-shadow: inset -1px 0 0 var(--nd-rule);
    cursor: var(--nd-cur-default, default);
  }
  .tr.odd .gutter {
    background: color-mix(in srgb, var(--nd-ink) 2.2%, var(--nd-surface));
  }
  .tr.picked .gutter {
    background: linear-gradient(var(--sel-tint), var(--sel-tint)), linear-gradient(var(--nd-sel), var(--nd-sel)) var(--nd-surface);
    color: var(--nd-ink);
  }
  .corner {
    background: var(--nd-surface-2);
    cursor: var(--nd-cur-cell, cell);
  }
  .th.sortable {
    cursor: var(--nd-cur-pointer, pointer);
  }
  .th-sort {
    font: 600 11px var(--nd-font-mono);
    color: var(--nd-accent-text);
  }
  .was,
  .rev {
    display: none;
  }
  .td.changed .was {
    display: inline;
    margin-right: 6px;
    font-size: 11px;
    color: var(--nd-ink-3);
  }
  .td.changed .val,
  .tr.new .val {
    color: var(--nd-pending);
  }
  .td.applied .val {
    animation: nd-ink 1.6s ease-out;
  }
  @keyframes nd-ink {
    from {
      color: var(--nd-pending);
    }
  }
  .tr.deleted .td {
    color: var(--nd-ink-3);
    text-decoration: line-through;
    text-decoration-color: var(--nd-error);
  }
  .tr.conflict .gutter,
  .tr.failed .gutter {
    box-shadow: inset 3px 0 0 var(--nd-error), inset -1px 0 0 var(--nd-rule);
  }
  .td.editing {
    overflow: visible;
    z-index: 3;
  }
  .cell-input {
    position: absolute;
    inset: -1px;
    min-width: 100%;
    padding: 0 9px;
    border: 0;
    outline: none;
    font: inherit;
    color: var(--nd-ink);
    background: var(--nd-raised);
    box-shadow: inset 0 0 0 2px var(--nd-focus), 0 4px 14px rgb(var(--nd-shade) / 0.18);
    user-select: text;
    -webkit-user-select: text;
  }
  .mark {
    display: inline-flex;
    margin-right: auto;
    color: var(--nd-pending);
  }
  .tr.conflict .mark,
  .tr.failed .mark,
  .tr.deleted .mark {
    color: var(--nd-error);
  }
  @media (prefers-reduced-motion: reduce) {
    .td.applied .val {
      animation: none;
    }
  }
</style>
