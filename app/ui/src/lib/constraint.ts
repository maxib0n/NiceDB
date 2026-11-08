import { m } from '../paraglide/messages';
import type { DriverError_Serialize } from './bindings';

export type ConstraintKind = 'foreign_key' | 'unique' | 'not_null' | 'check';

export interface Violation {
  kind: ConstraintKind;
  name: string | null;
  table: string | null;
  columns: string[];
  other: string | null;
  side: 'missing' | 'used' | null;
}

const PG: Record<string, ConstraintKind> = { '23503': 'foreign_key', '23505': 'unique', '23502': 'not_null', '23514': 'check' };
const MYSQL: Record<string, ConstraintKind> = { '1451': 'foreign_key', '1452': 'foreign_key', '1216': 'foreign_key', '1217': 'foreign_key', '1062': 'unique', '1048': 'not_null', '3819': 'check', '4025': 'check' };
const SQLITE: Record<string, ConstraintKind> = { 'FOREIGN KEY': 'foreign_key', UNIQUE: 'unique', 'NOT NULL': 'not_null', CHECK: 'check' };
const MSSQL: Record<string, ConstraintKind> = { '547': 'foreign_key', '2627': 'unique', '2601': 'unique', '515': 'not_null' };

const quoted = (s: string, q = '"') => [...s.matchAll(new RegExp(`${q}([^${q}]+)${q}`, 'g'))].map((x) => x[1] ?? '');
const list = (s: string | undefined) => (s ? s.split(',').map((c) => c.trim().replace(/^[`"[]|[`"\]]$/g, '')).filter(Boolean) : []);
const last = (s: string | null | undefined) => (s ? (s.split('.').pop() ?? s) : null);
const blank = (kind: ConstraintKind): Violation => ({ kind, name: null, table: null, columns: [], other: null, side: null });

function postgres(kind: ConstraintKind, message: string, detail: string | null): Violation {
  const v = blank(kind);
  const q = quoted(message);
  const cols = detail?.match(/\(([^)]*)\)=/);
  if (kind === 'not_null') {
    v.columns = q[0] ? [q[0]] : [];
    v.table = q[1] ?? null;
    return v;
  }
  v.columns = list(cols?.[1]);
  if (kind === 'unique') {
    v.name = q[0] ?? null;
    return v;
  }
  v.table = q[0] ?? null;
  v.name = q[1] ?? null;
  if (kind === 'foreign_key') {
    const other = q[2] ?? (detail ? (quoted(detail)[0] ?? null) : null);
    v.other = other;
    v.side = q[2] || /referenced|referenziat|引用/i.test(detail ?? '') ? 'used' : 'missing';
  }
  return v;
}

function mysql(kind: ConstraintKind, code: string, message: string): Violation {
  const v = blank(kind);
  if (kind === 'foreign_key') {
    const fk = message.match(/\((?:`[^`]+`\.)?`([^`]+)`, CONSTRAINT `([^`]+)` FOREIGN KEY \(([^)]*)\) REFERENCES `([^`]+)`/);
    v.side = code === '1451' || code === '1217' ? 'used' : 'missing';
    if (fk) {
      v.name = fk[2] ?? null;
      v.columns = list(fk[3]);
      if (v.side === 'used') {
        v.other = `${fk[1] ?? ''} (${v.columns.join(', ')})`;
        v.columns = [];
        v.table = fk[4] ?? null;
      } else {
        v.table = fk[1] ?? null;
        v.other = fk[4] ?? null;
      }
    }
  } else if (kind === 'unique') v.name = last(quoted(message, "'").at(-1));
  else if (kind === 'not_null') v.columns = quoted(message, "'").slice(0, 1);
  else v.name = quoted(message, "'")[0] ?? null;
  return v;
}

function mssql(kind: ConstraintKind, message: string): Violation {
  const check = kind === 'foreign_key' && /CHECK constraint/i.test(message);
  const v = blank(check ? 'check' : kind);
  if (kind === 'not_null') {
    const n = message.match(/column '([^']+)', table '([^']+)'/);
    v.columns = n?.[1] ? [n[1]] : [];
    v.table = last(n?.[2]);
    return v;
  }
  v.name = quoted(message)[0] ?? quoted(message, "'")[0] ?? null;
  const at = message.match(/table "([^"]+)"(?:, column '([^']+)')?/);
  const table = last(at?.[1]);
  const column = at?.[2];
  if (check) {
    v.table = table;
    v.columns = column ? [column] : [];
  } else if (kind === 'foreign_key') {
    v.side = /REFERENCE constraint/i.test(message) ? 'used' : 'missing';
    v.other = v.side === 'used' && table && column ? `${table} (${column})` : table;
  } else v.table = last(message.match(/object '([^']+)'/)?.[1]);
  return v;
}

function sqlite(message: string): Violation | null {
  const s = message.match(/^(FOREIGN KEY|UNIQUE|NOT NULL|CHECK) constraint failed(?:: (.*))?$/);
  if (!s) return null;
  const kind = SQLITE[s[1] ?? ''];
  if (!kind) return null;
  const v = blank(kind);
  const rest = s[2];
  if (!rest) return v;
  if (kind === 'check') v.name = rest;
  else {
    const cols = rest.split(',').map((c) => c.trim());
    v.table = cols[0]?.split('.')[0] ?? null;
    v.columns = cols.map((c) => c.split('.').pop() ?? c);
  }
  return v;
}

export function violation(e: DriverError_Serialize): Violation | null {
  if (e.kind !== 'query') return null;
  const code = e.code ?? '';
  const errno = e.detail?.match(/^error (\d+)$/)?.[1];
  if (errno && MYSQL[errno]) return mysql(MYSQL[errno], errno, e.message);
  if (PG[code]) return postgres(PG[code], e.message, e.detail);
  if (MSSQL[code] && e.detail?.startsWith('Msg ')) return mssql(MSSQL[code], e.message);
  return sqlite(e.message);
}

const KIND: Record<ConstraintKind, () => string> = { foreign_key: m.cons_fk, unique: m.cons_unique, not_null: m.cons_not_null, check: m.cons_check };

export function describeViolation(v: Violation, table: string | null = null): string {
  const t = v.table ?? table;
  const cols = v.columns.join(', ');
  const where = [v.name && m.cons_name({ name: v.name }), t ? (cols ? `${t} (${cols})` : t) : cols].filter(Boolean).join(' · ');
  const side = v.side === 'missing' ? (v.other ? m.cons_fk_missing({ table: v.other }) : m.cons_fk_missing_any()) : v.side === 'used' ? (v.other ? m.cons_fk_used({ table: v.other }) : m.cons_fk_used_any()) : '';
  return `${KIND[v.kind]()}${where ? `: ${where}` : ''}${side ? ` — ${side}` : ''}.`;
}
