import { m } from '../paraglide/messages';
import { api } from './api';
import { showToast } from './toast.svelte';
import { newTab, workspace, type Tab } from './workspace.svelte';

const failed = (e: unknown) => showToast(m.file_failed({ error: e instanceof Error ? e.message : String(e) }), null, 'error');

export async function openSqlFile(connectionId: string) {
  try {
    const f = await api.sqlFileOpen();
    if (!f) return;
    const open = workspace.tabs.find((t) => t.file?.path === f.path);
    if (open) {
      workspace.active = open.id;
      return;
    }
    const tab = newTab(connectionId, f.text);
    tab.title = f.name;
    tab.file = { path: f.path, name: f.name };
    tab.savedText = f.text;
  } catch (e) {
    failed(e);
  }
}

export async function openSqlPaths(connectionId: string, paths: readonly string[]) {
  for (const path of paths) {
    const open = workspace.tabs.find((t) => t.file?.path === path);
    if (open) {
      workspace.active = open.id;
      continue;
    }
    try {
      const f = await api.sqlFileRead(path);
      const tab = newTab(connectionId, f.text);
      tab.title = f.name;
      tab.file = { path: f.path, name: f.name };
      tab.savedText = f.text;
    } catch (e) {
      failed(e);
    }
  }
}

export async function saveSqlFile(tab: Tab, as = false) {
  try {
    const text = tab.text;
    const f = await api.sqlFileSave(as ? null : (tab.file?.path ?? null), tab.file?.name ?? tab.title, text);
    if (!f) return;
    if (!tab.file || tab.title === tab.file.name) tab.title = f.name;
    tab.file = { path: f.path, name: f.name };
    tab.savedText = text;
    showToast(m.file_saved({ name: f.name }));
  } catch (e) {
    failed(e);
  }
}
