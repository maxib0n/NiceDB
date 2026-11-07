<script lang="ts">
  import CommandPalette from './components/CommandPalette.svelte';
  import ConnectionDialog from './components/ConnectionDialog.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import OrgDialogs from './components/OrgDialogs.svelte';
  import PaletteButton from './components/PaletteButton.svelte';
  import SpaceSwitcher from './components/SpaceSwitcher.svelte';
  import Toast from './components/Toast.svelte';
  import Tooltip from './components/Tooltip.svelte';
  import ConfirmWrite from './components/ConfirmWrite.svelte';
  import Shell from './components/Shell.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import ViewSwitch from './components/ViewSwitch.svelte';
  import Workspace from './components/Workspace.svelte';
  import Wizard from './components/Wizard.svelte';
  import type { ConnectionProfile } from './lib/api';
  import { connections, loadConnections, type EditMode } from './lib/connections.svelte';
  import { guardNativeMenu } from './lib/contextmenu.svelte';
  import { flush, loadFolders, spaceKeys, startSpaces, watchSession } from './lib/org.svelte';
  import { undoKeys } from './lib/undo.svelte';
  import { activeTab, databaseOf, reopenClosed, sessionOf, waitingAll, watchSessions, workspace } from './lib/workspace.svelte';
  import { currentBindings } from './lib/prefs.svelte';
  import { matches } from './lib/shortcuts';
  import { togglePresentation } from './lib/presentation.svelte';
  import { api } from './lib/api';
  import { splitDropped } from './lib/file-drop';
  import { openSqlPaths } from './lib/sqlfile.svelte';
  import { showToast } from './lib/toast.svelte';
  import { guardWindowClose, settleBeforeClosing } from './lib/tx.svelte';
  import { widthVars } from './lib/resize';
  import { wizardShown } from './lib/wizard.svelte';
  import { closePalette, palette, paletteHost, paletteKeys, registerCommands, warmPalette } from './lib/palette.svelte';
  import { appCommands } from './lib/palette-commands';
  import { m } from './paraglide/messages';
  import { profileCommands } from './lib/profile-commands';
  import Connections from './screens/Connections.svelte';
  import Settings, { showSettingsSection } from './screens/Settings.svelte';
  import UpdateBanner from './components/UpdateBanner.svelte';
  import PointersOffer from './components/PointersOffer.svelte';
  import { update } from './lib/update.svelte';
  import Manual from './screens/Manual.svelte';
  import { currentContext, help, helpKeys, openHelp, closeHelp } from './lib/manual.svelte';
  import { CircleHelp, FolderOpen } from '@lucide/svelte';

  let settings = $state(new URLSearchParams(location.search).get('screen') === 'settings');
  let editing = $state<{ profile: ConnectionProfile | null; mode: EditMode; folder?: string | null } | null>(null);
  let dropping = $state(false);

  $effect(() => {
    let stop: (() => void) | null = null;
    let gone = false;
    void api
      .listenFileDrop((e) => {
        if (e.type === 'enter') dropping = splitDropped(e.paths).open.length > 0;
        else if (e.type === 'leave') dropping = false;
        else if (e.type === 'drop') {
          dropping = false;
          const { open, skipped } = splitDropped(e.paths);
          const tab = activeTab();
          if (!open.length) showToast(m.file_drop_none());
          else if (!tab) showToast(m.file_drop_no_connection());
          else {
            if (skipped.length) showToast(m.file_drop_skipped({ names: skipped.join(', ') }));
            void openSqlPaths(tab.connectionId, open);
          }
        }
      })
      .then((f) => (gone ? f() : (stop = f)))
      .catch(() => {});
    return () => {
      gone = true;
      stop?.();
    };
  });

  void Promise.all([loadConnections(), loadFolders()]).then(startSpaces);
  void update.autoCheck();
  update.beforeRestart = async () => (await settleBeforeClosing(waitingAll(), 'app')) && (await flush().catch(() => {}), true);
  $effect(() => guardNativeMenu());
  $effect(() => spaceKeys());
  $effect(() => watchSession());
  $effect(() => undoKeys());
  $effect(() => {
    const onkey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || wizardShown() || document.querySelector('dialog[open]')) return;
      const b = currentBindings();
      if (matches(e, b.presentation)) {
        e.preventDefault();
        togglePresentation();
        return;
      }
      if (settings || !matches(e, b.reopen_tab)) return;
      e.preventDefault();
      void reopenClosed();
    };
    window.addEventListener('keydown', onkey);
    return () => window.removeEventListener('keydown', onkey);
  });
  $effect(() => paletteKeys(() => !wizardShown()));
  $effect(() => warmPalette());
  const helpHere = () => {
    const slug = currentContext(settings);
    if (palette.open) closePalette();
    openHelp(slug);
  };
  $effect(() => helpKeys(helpHere, () => !wizardShown() && !document.querySelector('dialog[open]:not(.cmd-palette)')));
  $effect(() => registerCommands([{ id: 'app.help', label: m.pal_cmd_help, group: m.pal_group_app, keys: () => 'F1', icon: CircleHelp, run: helpHere }]));
  $effect(() => registerCommands(appCommands({ settings: () => (settings = true), newConnection: () => (editing = { profile: null, mode: 'edit' }) })));
  paletteHost.editConnection = (p) => (editing = { profile: p, mode: 'edit' });
  paletteHost.connectionDialog = (p, mode) => (editing = { profile: p, mode });
  $effect(() => registerCommands(profileCommands((p) => (editing = { profile: p, mode: 'follow' }))));
  watchSessions();
  guardWindowClose(waitingAll, flush);
  $effect(() => {
    const root = document.documentElement.style;
    for (const [name, value] of widthVars(workspace.widths)) {
      if (value === null) root.removeProperty(name);
      else root.setProperty(name, value);
    }
  });

  const tab = $derived(activeTab());
  const session = $derived(tab ? sessionOf(tab) : null);
  const profile = $derived(tab ? connections.list.find((c) => c.id === tab.connectionId) : undefined);
  const title = $derived(help.open ? m.help_title() : settings ? m.settings_title() : (profile?.name ?? m.nav_connections()));
  const HEALTH = { lost: m.status_lost, closed: m.status_idle_closed, reconnecting: m.status_reconnecting };
  const status = $derived(
    session
      ? m.status_session({ product: session.info.server.product, version: session.info.server.version, database: (tab && databaseOf(tab)) || session.info.server.database }) +
          (session.health === 'ok' ? '' : ` · ${HEALTH[session.health]()}`)
      : m.status_ready(),
  );
  const statusTone = $derived(session && session.health !== 'ok' ? session.health : null);

</script>

<Shell
  {title}
  settingsOpen={settings && !help.open}
  helpOpen={help.open}
  ontogglehelp={() => (help.open ? closeHelp() : helpHere())}
  {status}
  {statusTone}
  statusLive={!!session && session.health === 'ok'}
  statusRight={m.status_connections({ count: connections.list.length })}
  production={!settings && !help.open && profile?.environment === 'production'}
  inert={wizardShown()}
  ontogglesettings={() => {
    settings = help.open || !settings;
    closeHelp();
  }}
>
  {#snippet sidebar()}
    <Sidebar onedit={(p, mode = 'edit', folder = null) => (editing = { profile: p, mode, folder })} />
  {/snippet}
  {#snippet switcher()}
    <SpaceSwitcher />
  {/snippet}
  {#snippet actions()}
    {#if !settings && workspace.tabs.length > 0}<ViewSwitch />{/if}
    <PaletteButton />
  {/snippet}
  {#if help.open && (settings || workspace.tabs.length === 0)}
    <Manual />
  {:else if settings}
    <Settings />
  {:else if workspace.tabs.length > 0}
    <div class="beside" class:with-manual={help.open}>
      <Workspace />
      {#if help.open}<Manual side />{/if}
    </div>
  {:else}
    <Connections onnew={() => (editing = { profile: null, mode: 'edit' })} hasConnections={connections.list.length > 0} />
  {/if}
</Shell>
{#if dropping}
  <div class="file-drop" role="status" aria-live="polite"><FolderOpen size={28} strokeWidth={1.5} aria-hidden="true" /><span>{m.file_drop()}</span></div>
{/if}

{#if editing}
  <ConnectionDialog profile={editing.profile} copy={editing.mode === 'copy'} askDelete={editing.mode === 'delete'} follow={editing.mode === 'follow'} folder={editing.folder} onclose={() => (editing = null)} />
{/if}

{#if wizardShown()}
  <Wizard onnewconnection={() => (editing = { profile: null, mode: 'edit' })} />
{/if}

{#if palette.open}
  <CommandPalette />
{/if}

{#if !settings && !help.open && !wizardShown()}
  <UpdateBanner
    ondetails={() => {
      showSettingsSection('updates');
      closeHelp();
      settings = true;
    }}
  />
{/if}

<PointersOffer />
<OrgDialogs />
<ConfirmWrite />
<ContextMenu />
<Toast />
<Tooltip />
