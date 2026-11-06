import { cursorVar, POINTERS, pointerSvg, pointerVars, toHex, type CursorForm, type Pointer, type PointerShape } from './pointers';

const PLAIN = 'forced-colors: active), (prefers-contrast: more';
const PROBE = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E")`;

let form: CursorForm | null | undefined;
function cursorForm(): CursorForm | null {
  if (form !== undefined) return form;
  const ok = (v: string) => typeof CSS !== 'undefined' && CSS.supports('cursor', v);
  form = ok(`image-set(${PROBE} 1x) 1 1, auto`) ? 'image-set' : ok(`-webkit-image-set(${PROBE} 1x) 1 1, auto`) ? '-webkit-image-set' : ok(`${PROBE} 1 1, auto`) ? 'url' : null;
  return form;
}

const plain = typeof matchMedia === 'function' ? matchMedia(`(${PLAIN})`) : null;
let wanted = false;
let size = 100;
let last = '';

function clear(root: HTMLElement) {
  for (const n of POINTERS) root.style.removeProperty(cursorVar(n));
  delete root.dataset.pointers;
  last = '';
}

function resolve(root: HTMLElement, prop: string) {
  const probe = document.createElement('i');
  probe.style.cssText = `position:absolute;display:none;color:var(${prop})`;
  root.appendChild(probe);
  const c = getComputedStyle(probe).color;
  probe.remove();
  return toHex(c);
}

function themeInk(root: HTMLElement) {
  const fill = resolve(root, '--nd-pointer');
  const rim = resolve(root, '--nd-pointer-rim');
  const shape: PointerShape = getComputedStyle(root).getPropertyValue('--nd-pointer-shape').trim() === 'hard' ? 'hard' : 'soft';
  return fill && rim ? { ink: { fill, rim }, shape } : null;
}

export function pointerPreview(names: readonly Pointer[], root: HTMLElement = document.documentElement): string[] {
  const t = themeInk(root);
  return t ? names.map((n) => `data:image/svg+xml,${encodeURIComponent(pointerSvg(n, t.ink, t.shape, 48))}`) : [];
}

export function applyPointers(on: boolean, scale = 100, root: HTMLElement = document.documentElement) {
  wanted = on;
  size = scale;
  const f = cursorForm();
  if (!on || !f || plain?.matches) return clear(root);
  const t = themeInk(root);
  if (!t) return clear(root);
  const key = `${t.ink.fill}${t.ink.rim}${t.shape}${f}${scale}`;
  if (key === last) return;
  for (const [name, value] of pointerVars(t.ink, t.shape, f, scale)) root.style.setProperty(name, value);
  root.dataset.pointers = t.shape;
  last = key;
}

plain?.addEventListener('change', () => applyPointers(wanted, size));
