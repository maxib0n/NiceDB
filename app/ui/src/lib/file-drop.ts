export type FileDrag = { type: 'enter'; paths: string[] } | { type: 'over' } | { type: 'drop'; paths: string[] } | { type: 'leave' };

export const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function splitDropped(paths: readonly string[]): { open: string[]; skipped: string[] } {
  const open = paths.filter((p) => /\.sql$/i.test(p));
  return { open, skipped: paths.filter((p) => !open.includes(p)).map(baseName) };
}
