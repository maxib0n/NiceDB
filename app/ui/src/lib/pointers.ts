export type PointerShape = 'soft' | 'hard';
export type P = readonly [number, number];
export interface PointerInk {
  fill: string;
  rim: string;
}

export const POINTERS = [
  'default',
  'pointer',
  'press',
  'text',
  'vertical-text',
  'crosshair',
  'cell',
  'move',
  'all-scroll',
  'grab',
  'grabbing',
  'not-allowed',
  'help',
  'progress',
  'wait',
  'copy',
  'alias',
  'context-menu',
  'zoom-in',
  'zoom-out',
  'ns-resize',
  'ew-resize',
  'nwse-resize',
  'nesw-resize',
  'col-resize',
  'row-resize',
] as const;
export type Pointer = (typeof POINTERS)[number];

const HANDLESS: Partial<Record<Pointer, string>> = { pointer: 'default', press: 'default', grab: 'move', grabbing: 'move' };
export const FALLBACK: Record<Pointer, string> = Object.fromEntries(POINTERS.map((n) => [n, HANDLESS[n] ?? n])) as Record<Pointer, string>;

export const SIZE = 24;
const C: P = [12, 12];
const f = (p: P) => `${p[0].toFixed(2)} ${p[1].toFixed(2)}`;
const tow = (a: P, b: P, d: number): P => {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const l = Math.hypot(dx, dy);
  return [a[0] + (dx / l) * d, a[1] + (dy / l) * d];
};
const dir = (deg: number): P => [Math.cos((deg * Math.PI) / 180), Math.sin((deg * Math.PI) / 180)];
const add = (a: P, b: P, k = 1): P => [a[0] + b[0] * k, a[1] + b[1] * k];
const mid = (a: P, b: P): P => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];

const hex = (n: number) => Math.round(Math.min(255, Math.max(0, n))).toString(16).padStart(2, '0');
function rgb(c: string): [number, number, number] | null {
  const h = /^#([0-9a-f]{6})$/i.exec(c.trim());
  if (h?.[1]) return [0, 2, 4].map((i) => parseInt(h[1]!.slice(i, i + 2), 16)) as [number, number, number];
  const r = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)/i.exec(c.trim());
  if (r) return [Number(r[1]), Number(r[2]), Number(r[3])];
  const s = /^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/i.exec(c.trim());
  if (s) return [Number(s[1]) * 255, Number(s[2]) * 255, Number(s[3]) * 255];
  return null;
}
export function toHex(c: string): string | null {
  const v = rgb(c);
  return v ? `#${v.map(hex).join('')}` : null;
}
const mix = (a: string, b: string, k: number) => {
  const x = rgb(a);
  const y = rgb(b);
  return x && y ? `#${x.map((v, i) => hex(v + (y[i]! - v) * k)).join('')}` : a;
};

const TIP: Record<PointerShape, P> = { soft: [2.6, 1.6], hard: [3, 2] };
const arrowPts = (s: PointerShape) => {
  const t = TIP[s];
  return { t, b: add(t, dir(86), 17.6), r: add(t, dir(52), 17.2) };
};
export const HOTSPOT_TIP: Record<PointerShape, P> = { soft: [2, 2], hard: [3, 2] };

type Map = (p: P) => P;
const id: Map = (p) => p;

function arrowPath(s: PointerShape, m: Map = id) {
  const { t, b, r } = arrowPts(s);
  const g = (p: P) => f(m(p));
  if (s === 'hard') return `M${g(t)} L${g(b)} L${g(tow(mid(b, r), t, 2.2))} L${g(r)} Z`;
  const t1 = tow(t, b, 3.6);
  const t2 = tow(t, r, 3.6);
  const b1 = tow(b, t, 2.6);
  const b2 = tow(b, r, 2.4);
  const r1 = tow(r, b, 2.4);
  const r2 = tow(r, t, 2.6);
  const cc = tow(mid(b2, r1), t, 2.2);
  const k1 = tow(t1, t, 2);
  const k2 = tow(t2, t, 2);
  return `M${g(t1)} L${g(b1)} Q${g(b)} ${g(b2)} Q${g(cc)} ${g(r1)} Q${g(r)} ${g(r2)} L${g(t2)} C${g(k2)} ${g(k1)} ${g(t1)} Z`;
}

export function tilt(t: number, o: P): Map {
  const rad = Math.PI / 180;
  const [rz, ry, rx] = [4 * t * rad, -22 * t * rad, 30 * t * rad];
  const k = 1 - 0.2 * t;
  return (p) => {
    let x = (p[0] - o[0]) * k;
    let y = (p[1] - o[1]) * k;
    let z = 0;
    [x, y] = [x * Math.cos(rz) - y * Math.sin(rz), x * Math.sin(rz) + y * Math.cos(rz)];
    [x, z] = [x * Math.cos(ry) + z * Math.sin(ry), -x * Math.sin(ry) + z * Math.cos(ry)];
    [y, z] = [y * Math.cos(rx) - z * Math.sin(rx), y * Math.sin(rx) + z * Math.cos(rx)];
    const d = 32 / (32 - z);
    return [o[0] + x * d, o[1] + y * d];
  };
}

interface Part {
  d: string;
  w: number;
  fill?: boolean;
}
const line = (a: P, b: P, w: number): Part => ({ d: `M${f(a)} L${f(b)}`, w });
const poly = (pts: P[], w = 1.6): Part => ({ d: `M${pts.map(f).join(' L')} Z`, w, fill: true });
const circle = (c: P, r: number) => `M${f([c[0] - r, c[1]])} A${r} ${r} 0 1 0 ${f([c[0] + r, c[1]])} A${r} ${r} 0 1 0 ${f([c[0] - r, c[1]])}`;
const disk = (s: PointerShape, c: P, r: number, m: Map = id): Part => {
  if (s === 'hard') {
    const q = r * 0.9;
    return poly([m([c[0] - q, c[1] - q]), m([c[0] + q, c[1] - q]), m([c[0] + q, c[1] + q]), m([c[0] - q, c[1] + q])], 0.6);
  }
  const k = r * 0.5523;
  const [x, y] = c;
  const pts: P[] = [[x + r, y], [x + r, y + k], [x + k, y + r], [x, y + r], [x - k, y + r], [x - r, y + k], [x - r, y], [x - r, y - k], [x - k, y - r], [x, y - r], [x + k, y - r], [x + r, y - k]];
  const q = pts.map(m);
  let d = `M${f(q[0]!)}`;
  for (let i = 1; i < 12; i += 3) d += ` C${f(q[i]!)} ${f(q[i + 1]!)} ${f(q[(i + 2) % 12]!)}`;
  return { d: `${d} Z`, w: 0, fill: true };
};
const ring = (c: P, r: number, w: number): Part => ({ d: circle(c, r), w });
const head = (s: PointerShape, tip: P, deg: number, len = 5, half = 3.7): Part => {
  const u = dir(deg);
  const n: P = [-u[1], u[0]];
  const base = add(tip, u, -len);
  if (s === 'hard') return { d: `M${f(tip)} L${f(add(base, n, half))} L${f(add(base, u, 1))} L${f(add(base, n, -half))} Z`, w: 0.9, fill: true };
  return { d: `M${f(tip)} L${f(add(base, n, half))} Q${f(add(base, u, 1.3))} ${f(add(base, n, -half))} Z`, w: 1.3, fill: true };
};

interface Ctx {
  s: PointerShape;
  ink: PointerInk;
}
const RIM_W = 2.3;
const paths = (items: Part[], paint: string, extra: number) =>
  items.map((i) => `<path d="${i.d}" fill="${i.fill ? paint : 'none'}" stroke-width="${(i.w + extra).toFixed(2)}"/>`).join('');
function solid(x: Ctx, items: Part[]) {
  const cap = x.s === 'hard' ? 'square' : 'round';
  const join = x.s === 'hard' ? 'miter' : 'round';
  return (
    `<g filter="url(#s)" stroke-linecap="${cap}" stroke-linejoin="${join}" stroke-miterlimit="6">` +
    `<g stroke="${x.ink.rim}">${paths(items, x.ink.rim, RIM_W)}</g><g stroke="url(#gu)">${paths(items, 'url(#gu)', 0)}</g></g>`
  );
}
function arrow(x: Ctx, m: Map = id, scale = 1) {
  const sc = scale === 1 ? m : (p: P) => m(add(TIP[x.s], [p[0] - TIP[x.s][0], p[1] - TIP[x.s][1]], scale));
  const join = x.s === 'hard' ? 'miter' : 'round';
  return `<path d="${arrowPath(x.s, sc)}" fill="url(#g)" stroke="${x.ink.rim}" stroke-width="1.15" stroke-linejoin="${join}" stroke-miterlimit="8" filter="url(#s)"/>`;
}

const dbl = (x: Ctx, deg: number, l: number) => {
  const u = dir(deg);
  return [line(add(C, u, 3 - l), add(C, u, l - 3), 2.4), head(x.s, add(C, u, l), deg), head(x.s, add(C, u, -l), deg + 180)];
};
const split = (x: Ctx, deg: number) => {
  const u = dir(deg);
  const n: P = [-u[1], u[0]];
  return [
    line(add(C, n, -8), add(C, n, 8), 2.2),
    line(add(C, u, 6.4), add(C, u, 2.4), 2.4),
    head(x.s, add(C, u, 9.4), deg, 4.4, 3.4),
    line(add(C, u, -6.4), add(C, u, -2.4), 2.4),
    head(x.s, add(C, u, -9.4), deg + 180, 4.4, 3.4),
  ];
};
const ibeam = (x: Ctx, deg: number) => {
  const u = dir(deg);
  const n: P = [-u[1], u[0]];
  const m = (p: P) => add(add(C, n, p[0]), u, p[1]);
  const wing = (k: number): Part =>
    x.s === 'hard'
      ? line(m([-2.8, k * 7.4]), m([2.8, k * 7.4]), 1.7)
      : { d: `M${f(m([-3.2, k * 7.8]))} Q${f(m([0, k * 7.6]))} ${f(m([0, k * 6]))} Q${f(m([0, k * 7.6]))} ${f(m([3.2, k * 7.8]))}`, w: 1.9 };
  return [line(m([0, -6.4]), m([0, 6.4]), x.s === 'hard' ? 2 : 2.2), wing(-1), wing(1)];
};
const BADGE: P = [17.2, 17.2];
const badge = (x: Ctx, glyph: string) =>
  solid(x, [disk(x.s, BADGE, 4.4)]) +
  `<g transform="translate(${f(BADGE)})" stroke="${x.ink.rim}" fill="none" stroke-width="1.35" stroke-linecap="${x.s === 'hard' ? 'square' : 'round'}" stroke-linejoin="${x.s === 'hard' ? 'miter' : 'round'}">${glyph}</g>`;
const arcD = (c: P, r: number, a0: number, a1: number) => `M${f(add(c, dir(a0), r))} A${r} ${r} 0 ${a1 - a0 > 180 ? 1 : 0} 1 ${f(add(c, dir(a1), r))}`;
const light = (x: Ctx, c: P, r: number, w: number) => {
  const n = 10;
  const [a0, len] = [-150, 150];
  let s = '';
  for (let i = 0; i < n; i++) s += `<path d="${arcD(c, r, a0 + (len * i) / n, a0 + (len * (i + 1)) / n + 0.6)}" opacity="${((i + 1) / n).toFixed(2)}"/>`;
  return `<g stroke="${x.ink.rim}" stroke-width="${w}" fill="none" stroke-linecap="butt">${s}</g>`;
};
const grip = (x: Ctx, m: Map = id, k = 1) =>
  [-1, 1]
    .flatMap((i) =>
      [-1, 0, 1].map((j) => {
        const p = m([12 + i * 2.1, 12 + j * 3.6]);
        return x.s === 'hard'
          ? `<rect x="${(p[0] - 0.95 * k).toFixed(2)}" y="${(p[1] - 0.95 * k).toFixed(2)}" width="${(1.9 * k).toFixed(2)}" height="${(1.9 * k).toFixed(2)}" fill="${x.ink.rim}"/>`
          : `<circle cx="${p[0].toFixed(2)}" cy="${p[1].toFixed(2)}" r="${(1.05 * k).toFixed(2)}" fill="${x.ink.rim}"/>`;
      }),
    )
    .join('');

const Q_GLYPH = (x: Ctx) =>
  x.s === 'hard'
    ? `<path d="M-1.6 -1.2 V-2.8 H1.6 V-0.4 L0 0.4 V1"/><rect x="-0.75" y="2" width="1.5" height="1.5" fill="${x.ink.rim}" stroke="none"/>`
    : `<path d="M-1.5 -1.3 Q-1.5 -2.9 0 -2.9 Q1.6 -2.9 1.6 -1.4 Q1.6 -0.4 0.4 0.1 Q0 0.3 0 1"/><circle cx="0" cy="2.6" r="0.8" fill="${x.ink.rim}" stroke="none"/>`;

interface Def {
  hot: P | ((s: PointerShape) => P);
  body: (x: Ctx) => string;
}
const LINK_T = 0.35;
const TILT_T = 1;
const DEFS: Record<Pointer, Def> = {
  default: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x) },
  pointer: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x, tilt(LINK_T, TIP[x.s])) },
  press: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x, tilt(TILT_T, TIP[x.s])) },
  text: { hot: C, body: (x) => solid(x, ibeam(x, 90)) },
  'vertical-text': { hot: C, body: (x) => solid(x, ibeam(x, 0)) },
  crosshair: {
    hot: C,
    body: (x) =>
      solid(x, [x.s === 'hard' ? { d: 'M8 8 H16 V16 H8 Z', w: 1.6 } : ring(C, 4.6, 1.6), ...[0, 90, 180, 270].map((d) => line(add(C, dir(d), 7.2), add(C, dir(d), 10), 1.8))]) +
      `<circle cx="12" cy="12" r="0.9" fill="${x.ink.fill}" stroke="${x.ink.rim}" stroke-width="0.8"/>`,
  },
  cell: { hot: C, body: (x) => solid(x, [line([12, 4.6], [12, 19.4], 4.4), line([4.6, 12], [19.4, 12], 4.4)]) },
  move: { hot: C, body: (x) => solid(x, [0, 90, 180, 270].flatMap((d) => [line(C, add(C, dir(d), 6), 2.2), head(x.s, add(C, dir(d), 9.4), d, 4, 2.8)])) },
  'all-scroll': { hot: C, body: (x) => solid(x, [0, 90, 180, 270].map((d) => head(x.s, add(C, dir(d), 9.4), d, 3.6, 2.6)).concat(disk(x.s, C, 2))) },
  grab: { hot: C, body: (x) => solid(x, [disk(x.s, C, 7)]) + grip(x) },
  grabbing: { hot: C, body: (x) => solid(x, [disk(x.s, C, 7, tilt(0.65, C))]) + grip(x, tilt(0.65, C), 0.88) },
  'not-allowed': { hot: C, body: (x) => solid(x, [ring(C, 7.2, 2.8), line(add(C, dir(225), 7.2), add(C, dir(45), 7.2), 2.8)]) },
  help: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x, id, 0.82) + badge(x, Q_GLYPH(x)) },
  progress: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x, id, 0.82) + solid(x, [disk(x.s, BADGE, 4.4)]) + light(x, BADGE, 2.4, 1.6) },
  wait: { hot: C, body: (x) => solid(x, [ring(C, 7.2, 3.4)]) + light(x, C, 7.2, 1.9) },
  copy: { hot: (s) => HOTSPOT_TIP[s], body: (x) => arrow(x, id, 0.82) + badge(x, '<path d="M-2.1 0 H2.1 M0 -2.1 V2.1"/>') },
  alias: {
    hot: (s) => HOTSPOT_TIP[s],
    body: (x) => arrow(x, id, 0.82) + badge(x, `<path d="M-2 2.3 V0.6 Q-2 -1.2 -0.2 -1.2 H0.9"/><path d="M2.6 -1.2 L0.7 -2.7 L0.7 0.3 Z" fill="${x.ink.rim}" stroke-width="0.9"/>`),
  },
  'context-menu': {
    hot: (s) => HOTSPOT_TIP[s],
    body: (x) =>
      arrow(x, id, 0.82) +
      solid(x, [poly([[13, 11.8], [21.6, 11.8], [21.6, 21.6], [13, 21.6]], x.s === 'hard' ? 0.6 : 2)]) +
      `<g stroke="${x.ink.rim}" stroke-width="1.2" stroke-linecap="${x.s === 'hard' ? 'butt' : 'round'}"><path d="M14.9 14.4 H19.7 M14.9 16.7 H19.7 M14.9 19 H18"/></g>`,
  },
  'zoom-in': { hot: [10, 10], body: (x) => zoom(x, 'M7.6 10 H12.4 M10 7.6 V12.4') },
  'zoom-out': { hot: [10, 10], body: (x) => zoom(x, 'M7.6 10 H12.4') },
  'ns-resize': { hot: C, body: (x) => solid(x, dbl(x, 90, 8.8)) },
  'ew-resize': { hot: C, body: (x) => solid(x, dbl(x, 0, 8.8)) },
  'nwse-resize': { hot: C, body: (x) => solid(x, dbl(x, 45, 9.6)) },
  'nesw-resize': { hot: C, body: (x) => solid(x, dbl(x, -45, 9.6)) },
  'col-resize': { hot: C, body: (x) => solid(x, split(x, 0)) },
  'row-resize': { hot: C, body: (x) => solid(x, split(x, 90)) },
};
function zoom(x: Ctx, glyph: string) {
  return (
    `<circle cx="10" cy="10" r="5.4" fill="${x.ink.rim}" opacity="0.92"/>` +
    solid(x, [ring([10, 10], 6.2, 2.6), line([14.7, 14.7], [19.6, 19.6], 3.4)]) +
    `<path d="${glyph}" stroke="url(#gu)" stroke-width="1.8" stroke-linecap="${x.s === 'hard' ? 'butt' : 'round'}"/>`
  );
}

export function hotspot(name: Pointer, s: PointerShape): P {
  const h = DEFS[name].hot;
  return typeof h === 'function' ? h(s) : h;
}

export function pointerSvg(name: Pointer, ink: PointerInk, s: PointerShape, px = SIZE): string {
  const stops =
    s === 'hard'
      ? `<stop offset="0" stop-color="${ink.fill}"/><stop offset="1" stop-color="${mix(ink.fill, '#000000', 0.12)}"/>`
      : `<stop offset="0" stop-color="${mix(ink.fill, '#ffffff', 0.2)}"/><stop offset="0.5" stop-color="${ink.fill}"/><stop offset="1" stop-color="${mix(ink.fill, '#000000', 0.36)}"/>`;
  const shadow = s === 'hard' ? '<feDropShadow dx="0.5" dy="0.7" stdDeviation="0.2" flood-color="#000" flood-opacity="0.45"/>' : '<feDropShadow dx="0" dy="0.6" stdDeviation="0.7" flood-color="#000" flood-opacity="0.5"/>';
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${px}" height="${px}" viewBox="0 0 ${SIZE} ${SIZE}"><defs>` +
    `<linearGradient id="g" x1="0.1" y1="0" x2="0.7" y2="1">${stops}</linearGradient>` +
    `<linearGradient id="gu" gradientUnits="userSpaceOnUse" x1="5" y1="2" x2="15" y2="22">${stops}</linearGradient>` +
    `<filter id="s" filterUnits="userSpaceOnUse" x="-4" y="-4" width="32" height="32">${shadow}</filter>` +
    `</defs>${DEFS[name].body({ s, ink })}</svg>`
  );
}

export type CursorForm = 'image-set' | '-webkit-image-set' | 'url';
const uri = (svg: string) => `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;

export function pointerCss(name: Pointer, ink: PointerInk, s: PointerShape, form: CursorForm, scale = 100): string {
  const px = Math.round((SIZE * scale) / 100);
  const [x, y] = hotspot(name, s).map((v) => Math.round((v * scale) / 100));
  const one = uri(pointerSvg(name, ink, s, px));
  const img = form === 'url' ? one : `${form}(${one} 1x, ${uri(pointerSvg(name, ink, s, px * 2))} 2x)`;
  return `${img} ${x} ${y}, ${FALLBACK[name]}`;
}

export const cursorVar = (name: Pointer) => `--nd-cur-${name}`;

export function pointerVars(ink: PointerInk, s: PointerShape, form: CursorForm, scale = 100): [string, string][] {
  return POINTERS.map((n) => [cursorVar(n), pointerCss(n, ink, s, form, scale)]);
}
