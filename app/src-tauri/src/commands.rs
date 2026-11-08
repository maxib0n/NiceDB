use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::sync::Arc;

use nd_core::{
    ApplyOutcome, ChangePreview, CommitMode, Core, ExplainRequest, KeyMatch, Page, ParamsFound,
    PlanReport, RunEvent, RunRequest, RunScope, Secrets, SessionEvent, SessionInfo, SummarySource,
    TxState, WriteAccess,
};
use nd_driver::{
    Dependency, Dialect, DocumentWrite, KeyPage, KeyRead, KeyScan, KeyValue, KeyWrite, ObjectRef,
    Outcome, PackedSchema, Report, Schema, SchemaStart, ServerInfo,
};
use nd_sql::{Browse, ChangeSet, GenRequest, Summary};
use nd_store::{
    Appearance, AuditEntry, ConnectionProfile, ErLayout, HistoryEntry, HistoryOutcome, UiScale,
    Visibility,
};
use tauri::ipc::Channel;
use tauri::{State, WebviewWindow};
use tauri_specta::{Builder, collect_commands};

use crate::error::CommandError;

type Ctx<'a> = State<'a, Arc<Core>>;
type Res<T> = Result<T, CommandError>;

#[tauri::command]
#[specta::specta]
fn appearance_get(core: Ctx<'_>) -> Res<Appearance> {
    core.store(nd_store::Store::appearance)
}

#[tauri::command]
#[specta::specta]
fn appearance_set(window: WebviewWindow, core: Ctx<'_>, value: Appearance) -> Res<()> {
    core.store(|s| s.set_appearance(&value))?;
    apply_scale(&window, value.ui_scale);
    Ok(())
}

pub(crate) fn apply_scale(window: &WebviewWindow, scale: UiScale) {
    if !cfg!(desktop) {
        return;
    }
    if let Err(e) = window.set_zoom(scale.zoom()) {
        tracing::warn!(error = %e, "interface size not applied");
    }
}

#[tauri::command]
#[specta::specta]
fn er_layout_get(core: Ctx<'_>, connection_id: String, database: String) -> Res<Option<ErLayout>> {
    core.store(|s| s.layout(&connection_id, &database))
}

#[tauri::command]
#[specta::specta]
fn er_layout_set(
    core: Ctx<'_>,
    connection_id: String,
    database: String,
    layout: ErLayout,
) -> Res<()> {
    core.store(|s| s.set_layout(&connection_id, &database, &layout))
}

#[tauri::command]
#[specta::specta]
fn connections_list(core: Ctx<'_>) -> Res<Vec<ConnectionProfile>> {
    core.store(nd_store::Store::connections)
}

#[tauri::command]
#[specta::specta]
async fn connection_save(
    core: Ctx<'_>,
    profile: ConnectionProfile,
    secrets: Secrets,
    confirm: Option<String>,
) -> Res<()> {
    core.save_connection_confirmed(&profile, secrets, confirm.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
async fn connection_delete(core: Ctx<'_>, id: String) -> Res<()> {
    core.delete_connection(&id).await
}

#[tauri::command]
#[specta::specta]
fn connection_trash(core: Ctx<'_>, id: String) -> Res<()> {
    core.trash_connection(&id)
}

#[tauri::command]
#[specta::specta]
fn connection_restore(
    core: Ctx<'_>,
    profile: ConnectionProfile,
    confirm: Option<String>,
) -> Res<()> {
    core.restore_connection_confirmed(&profile, confirm.as_deref())
}

#[tauri::command]
#[specta::specta]
async fn connection_purge(core: Ctx<'_>, id: String) -> Res<()> {
    core.purge_connection(&id).await
}

#[tauri::command]
#[specta::specta]
async fn connection_test(
    core: Ctx<'_>,
    profile: ConnectionProfile,
    secrets: Secrets,
) -> Res<ServerInfo> {
    core.test_connection(&profile, secrets).await
}

#[tauri::command]
#[specta::specta]
fn vars_preview(
    core: Ctx<'_>,
    connection_id: Option<String>,
    folder: Option<String>,
    texts: Vec<String>,
) -> Res<Vec<nd_core::VarPreview>> {
    core.preview_vars(connection_id.as_deref(), folder.as_deref(), &texts)
}

#[tauri::command]
#[specta::specta]
fn host_key_trust(core: Ctx<'_>, host: String, port: u16, key: String) -> Res<()> {
    core.trust_host_key(&host, port, &key)
}

#[tauri::command]
#[specta::specta]
async fn session_open(
    core: Ctx<'_>,
    connection_id: String,
    database: Option<String>,
) -> Res<SessionInfo> {
    core.open_session_in(&connection_id, database.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
async fn session_switch(core: Ctx<'_>, session_id: String, database: String) -> Res<SessionInfo> {
    core.switch_database(&session_id, &database).await
}

#[tauri::command]
#[specta::specta]
async fn session_reconnect(core: Ctx<'_>, session_id: String) -> Res<SessionInfo> {
    core.reconnect(&session_id).await
}

#[derive(Default)]
pub(crate) struct SessionWatch(std::sync::Mutex<Option<Channel<SessionEvent>>>);

impl SessionWatch {
    pub(crate) fn send(&self, event: SessionEvent) {
        if let Ok(guard) = self.0.lock()
            && let Some(channel) = guard.as_ref()
            && let Err(e) = channel.send(event)
        {
            tracing::warn!(error = %e, "session event dropped");
        }
    }
}

#[tauri::command]
#[specta::specta]
fn session_watch(watch: State<'_, SessionWatch>, events: Channel<SessionEvent>) -> Res<()> {
    *watch.0.lock().map_err(|_| CommandError::Poisoned)? = Some(events);
    Ok(())
}

#[tauri::command]
#[specta::specta]
fn visibility_get(core: Ctx<'_>, connection_id: String) -> Res<Visibility> {
    core.store(|s| s.visibility(&connection_id))
}

#[tauri::command]
#[specta::specta]
fn visibility_set(core: Ctx<'_>, connection_id: String, visibility: Visibility) -> Res<()> {
    core.store(|s| s.set_visibility(&connection_id, &visibility))
}

#[tauri::command]
#[specta::specta]
fn session_close(core: Ctx<'_>, session_id: String) -> Res<()> {
    core.close_session(&session_id)
}

#[tauri::command]
#[specta::specta]
async fn search_path(core: Ctx<'_>, session_id: String) -> Res<Option<Vec<String>>> {
    core.search_path(&session_id).await
}

#[tauri::command]
#[specta::specta]
async fn schema_load(core: Ctx<'_>, session_id: String) -> Res<PackedSchema> {
    core.schema(&session_id).await.map(PackedSchema::from)
}

#[tauri::command]
#[specta::specta]
async fn schema_open(core: Ctx<'_>, session_id: String) -> Res<SchemaStart> {
    let (schema, pending) = core.schema_open(&session_id).await?;
    Ok(SchemaStart {
        schema: schema.into(),
        pending,
    })
}

#[tauri::command]
#[specta::specta]
async fn schema_namespace(
    core: Ctx<'_>,
    session_id: String,
    namespace: String,
) -> Res<PackedSchema> {
    core.schema_namespace(&session_id, &namespace)
        .await
        .map(PackedSchema::from)
}

#[tauri::command]
#[specta::specta]
async fn object_ddl(core: Ctx<'_>, session_id: String, object: ObjectRef) -> Res<String> {
    core.ddl(&session_id, &object).await
}

#[tauri::command]
#[specta::specta]
async fn object_dependencies(
    core: Ctx<'_>,
    session_id: String,
    object: ObjectRef,
) -> Res<Vec<Dependency>> {
    core.dependencies(&session_id, &object).await
}

#[tauri::command]
#[specta::specta]
fn sql_generate(dialect: Dialect, request: GenRequest) -> Option<String> {
    nd_sql::generate(dialect, &request)
}

#[tauri::command]
#[specta::specta]
async fn run(core: Ctx<'_>, request: RunRequest, events: Channel<RunEvent>) -> Res<()> {
    core.run(request, move |e| {
        if let Err(err) = events.send(e) {
            tracing::warn!(error = %err, "run event dropped");
        }
    })
    .await
}

#[tauri::command]
#[specta::specta]
async fn params_found(
    core: Ctx<'_>,
    text: String,
    dialect: Dialect,
    scope: RunScope,
) -> Res<ParamsFound> {
    let core = Arc::clone(&core);
    tokio::task::spawn_blocking(move || core.params_found(&text, dialect, &scope))
        .await
        .map_err(|e| CommandError::Store(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
async fn explain(core: Ctx<'_>, request: ExplainRequest) -> Res<PlanReport> {
    core.explain(request).await
}

#[tauri::command]
#[specta::specta]
async fn write_removals(
    core: Ctx<'_>,
    session_id: String,
    text: String,
    scope: RunScope,
) -> Res<Vec<nd_core::Removing>> {
    core.removals(&session_id, &text, &scope).await
}

#[tauri::command]
#[specta::specta]
async fn write_linked(
    core: Ctx<'_>,
    session_id: String,
    text: String,
    scope: RunScope,
    index: u32,
    links: Vec<nd_core::Referencing>,
) -> Res<Vec<Option<f64>>> {
    let counts = core
        .linked_rows(&session_id, &text, &scope, index, &links)
        .await?;
    #[allow(clippy::cast_precision_loss)]
    Ok(counts.into_iter().map(|n| n.map(|n| n as f64)).collect())
}

#[tauri::command]
#[specta::specta]
fn plan_history(
    core: Ctx<'_>,
    connection_id: String,
    database: String,
    sql: String,
) -> Res<Vec<nd_store::SavedPlan>> {
    core.plan_history(&connection_id, &database, &sql)
}

#[tauri::command]
#[specta::specta]
fn plan_saved(core: Ctx<'_>, id: u32) -> Res<Option<String>> {
    core.plan_saved(i64::from(id))
}

#[tauri::command]
#[specta::specta]
async fn run_cancel(core: Ctx<'_>, run_id: String) -> Res<()> {
    core.cancel(&run_id).await
}

#[tauri::command]
#[specta::specta]
fn history_list(
    core: Ctx<'_>,
    connection_id: Option<String>,
    query: String,
    outcome: HistoryOutcome,
    limit: u32,
) -> Res<Vec<HistoryEntry>> {
    core.store(|s| s.history_where(connection_id.as_deref(), &query, outcome, limit.min(1000)))
}

fn exportable(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        [
            "svg", "png", "csv", "json", "sql", "md", "txt", "xml", "jpg", "jpeg", "gif", "webp",
            "bmp", "ico", "pdf", "zip", "gz", "bin",
        ]
        .iter()
        .any(|x| e.eq_ignore_ascii_case(x))
    })
}

async fn ask_path(window: &WebviewWindow, save_as: Option<&str>) -> Option<PathBuf> {
    ask_path_in(window, save_as, None, None).await
}

async fn ask_path_in(
    window: &WebviewWindow,
    save_as: Option<&str>,
    folder: Option<PathBuf>,
    pick: Option<&[&str]>,
) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt as _;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let done = move |p: Option<tauri_plugin_dialog::FilePath>| {
        let _ = tx.send(p.and_then(|p| p.into_path().ok()));
    };
    let mut dialog = window.dialog().file();
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    if let Some(name) = save_as {
        let mut dialog = dialog.set_parent(window).set_file_name(name);
        if let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) {
            dialog = dialog.add_filter(ext.to_uppercase(), &[ext]);
        }
        dialog.save_file(done);
    } else {
        if let Some(extensions) = pick {
            dialog = dialog.set_parent(window).add_filter("SQL", extensions);
        }
        dialog.pick_file(done);
    }
    rx.await.ok().flatten()
}

#[derive(serde::Deserialize, specta::Type)]
pub struct ColumnSample {
    pub name: String,
    pub values: Vec<String>,
}

#[tauri::command]
#[specta::specta]
fn sensitive_columns(columns: Vec<ColumnSample>) -> Vec<Option<String>> {
    columns
        .iter()
        .map(|c| {
            let values: Vec<&str> = c.values.iter().map(String::as_str).collect();
            nd_core::sensitive_kind(&c.name, &values).map(str::to_owned)
        })
        .collect()
}

#[derive(serde::Serialize, specta::Type)]
pub struct SqlFile {
    pub path: String,
    pub name: String,
    pub text: String,
}

fn sql_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("sql") || e.eq_ignore_ascii_case("txt"))
}

fn sql_file(path: &Path, text: String) -> SqlFile {
    SqlFile {
        path: path.display().to_string(),
        name: path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        text,
    }
}

fn sql_folder(core: &Ctx<'_>) -> Option<PathBuf> {
    core.store(nd_store::Store::sql_folder)
        .ok()
        .flatten()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

fn remember_folder(core: &Ctx<'_>, path: &Path) {
    if let Some(folder) = path.parent().and_then(Path::to_str) {
        let _ = core.store(|s| s.set_sql_folder(folder));
    }
}

#[tauri::command]
#[specta::specta]
async fn sql_file_open(core: Ctx<'_>, window: WebviewWindow) -> Res<Option<SqlFile>> {
    let Some(path) = ask_path_in(&window, None, sql_folder(&core), Some(&["sql", "txt"])).await
    else {
        return Ok(None);
    };
    read_sql(&core, path).await.map(Some)
}

#[tauri::command]
#[specta::specta]
async fn sql_file_read(core: Ctx<'_>, path: String) -> Res<SqlFile> {
    read_sql(&core, PathBuf::from(path)).await
}

async fn read_sql(core: &Ctx<'_>, path: PathBuf) -> Res<SqlFile> {
    const MAX: u64 = 16 * 1024 * 1024;
    if !sql_path(&path) {
        return Err(CommandError::NotFound(path.display().to_string()));
    }
    remember_folder(core, &path);
    tokio::task::spawn_blocking(move || {
        if std::fs::metadata(&path)?.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the file is larger than 16 MB",
            ));
        }
        let text = std::fs::read_to_string(&path)?;
        Ok(sql_file(&path, text))
    })
    .await
    .map_err(|e| CommandError::Store(e.to_string()))?
    .map_err(|e| CommandError::Store(e.to_string()))
}

#[tauri::command]
#[specta::specta]
async fn sql_file_save(
    core: Ctx<'_>,
    window: WebviewWindow,
    path: Option<String>,
    name: String,
    text: String,
) -> Res<Option<SqlFile>> {
    let path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        let name = if sql_path(Path::new(&name)) {
            name
        } else {
            format!("{name}.sql")
        };
        let Some(p) = ask_path_in(&window, Some(&name), sql_folder(&core), None).await else {
            return Ok(None);
        };
        p
    };
    if !sql_path(&path) {
        return Err(CommandError::NotFound(path.display().to_string()));
    }
    remember_folder(&core, &path);
    tokio::task::spawn_blocking(move || {
        std::fs::write(&path, &text)?;
        Ok(Some(sql_file(&path, text)))
    })
    .await
    .map_err(|e| CommandError::Store(e.to_string()))?
    .map_err(|e: std::io::Error| CommandError::Store(e.to_string()))
}

#[derive(serde::Serialize, specta::Type)]
pub struct LoadedFile {
    pub name: String,
    pub data: String,
}

#[tauri::command]
#[specta::specta]
async fn file_load(window: WebviewWindow) -> Res<Option<LoadedFile>> {
    const MAX: u64 = 16 * 1024 * 1024;
    let Some(path) = ask_path(&window, None).await else {
        return Ok(None);
    };
    tokio::task::spawn_blocking(move || {
        if std::fs::metadata(&path)?.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the file is larger than 16 MB",
            ));
        }
        let data = STANDARD.encode(std::fs::read(&path)?);
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        Ok(Some(LoadedFile { name, data }))
    })
    .await
    .map_err(|e| CommandError::Store(e.to_string()))?
    .map_err(|e| CommandError::Store(e.to_string()))
}

#[tauri::command]
#[specta::specta]
async fn export_save(window: WebviewWindow, name: String, data: String) -> Res<bool> {
    let Some(path) = ask_path(&window, Some(&name)).await else {
        return Ok(false);
    };
    write_export(path, data).await?;
    Ok(true)
}

async fn write_export(path: PathBuf, data: String) -> Res<()> {
    if !exportable(&path) {
        return Err(CommandError::NotFound(path.display().to_string()));
    }
    tokio::task::spawn_blocking(move || {
        let bytes = STANDARD
            .decode(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, bytes)
    })
    .await
    .map_err(|e| CommandError::Store(e.to_string()))?
    .map_err(|e| CommandError::Store(e.to_string()))
}

#[tauri::command]
#[specta::specta]
async fn report_pdf(window: WebviewWindow, name: String, report: nd_pdf::PdfReport) -> Res<bool> {
    let Some(path) = ask_path(&window, Some(&name)).await else {
        return Ok(false);
    };
    write_pdf(path, report).await?;
    Ok(true)
}

async fn write_pdf(path: PathBuf, report: nd_pdf::PdfReport) -> Res<()> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
    {
        return Err(CommandError::NotFound(path.display().to_string()));
    }
    tokio::task::spawn_blocking(move || {
        let bytes = nd_pdf::render(&report).map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| CommandError::Store(e.to_string()))?
    .map_err(CommandError::Io)
}

#[tauri::command]
#[specta::specta]
async fn data_browse(core: Ctx<'_>, session_id: String, browse: Browse) -> Res<Page> {
    core.browse(&session_id, &browse).await
}

#[tauri::command]
#[specta::specta]
async fn table_identity(
    core: Ctx<'_>,
    session_id: String,
    namespace: String,
    name: String,
) -> Res<Vec<nd_core::IdentityColumn>> {
    core.identity_columns(&session_id, &namespace, &name).await
}

#[tauri::command]
#[specta::specta]
async fn data_pick(core: Ctx<'_>, session_id: String, browse: Browse, job_id: String) -> Res<Page> {
    core.pick_rows(&session_id, &browse, &job_id).await
}

#[tauri::command]
#[specta::specta]
async fn data_summarize(
    core: Ctx<'_>,
    session_id: String,
    source: SummarySource,
    summary: Summary,
) -> Res<Vec<Vec<Option<String>>>> {
    core.summarize(&session_id, &source, &summary).await
}

#[tauri::command]
#[specta::specta]
async fn database_counts(core: Ctx<'_>, session_id: String) -> Res<Vec<(String, f64)>> {
    let counts = core.database_counts(&session_id).await?;
    #[allow(clippy::cast_precision_loss)]
    Ok(counts.into_iter().map(|(d, n)| (d, n as f64)).collect())
}

#[tauri::command]
#[specta::specta]
async fn data_related(
    core: Ctx<'_>,
    session_id: String,
    matches: Vec<KeyMatch>,
) -> Res<Vec<Option<f64>>> {
    let counts = core.count_matches(&session_id, &matches).await?;
    #[allow(clippy::cast_precision_loss)]
    Ok(counts.into_iter().map(|n| n.map(|n| n as f64)).collect())
}

#[tauri::command]
#[specta::specta]
fn data_preview(core: Ctx<'_>, session_id: String, changes: ChangeSet) -> Res<ChangePreview> {
    core.preview_changes(&session_id, &changes)
}

#[tauri::command]
#[specta::specta]
async fn data_estimate(
    core: Ctx<'_>,
    session_id: String,
    changes: ChangeSet,
) -> Res<Vec<Option<f64>>> {
    let counts = core.estimate_changes(&session_id, &changes).await?;
    #[allow(clippy::cast_precision_loss)]
    Ok(counts.into_iter().map(|n| n.map(|n| n as f64)).collect())
}

#[tauri::command]
#[specta::specta]
async fn documents_write(
    core: Ctx<'_>,
    session_id: String,
    write: DocumentWrite,
    confirm: Option<String>,
) -> Res<Outcome> {
    core.write_documents(&session_id, &write, confirm.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
async fn keys_scan(core: Ctx<'_>, session_id: String, scan: KeyScan) -> Res<KeyPage> {
    core.scan_keys(&session_id, &scan).await
}

#[tauri::command]
#[specta::specta]
async fn key_read(core: Ctx<'_>, session_id: String, read: KeyRead) -> Res<KeyValue> {
    core.read_key(&session_id, &read).await
}

#[tauri::command]
#[specta::specta]
async fn key_write(
    core: Ctx<'_>,
    session_id: String,
    write: KeyWrite,
    confirm: Option<String>,
) -> Res<Outcome> {
    core.write_key(&session_id, &write, confirm.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
async fn server_report(core: Ctx<'_>, session_id: String) -> Res<Report> {
    core.server_report(&session_id).await
}

#[tauri::command]
#[specta::specta]
async fn data_apply(
    core: Ctx<'_>,
    session_id: String,
    changes: ChangeSet,
    confirm: Option<String>,
) -> Res<ApplyOutcome> {
    core.apply_changes(&session_id, &changes, confirm.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
fn write_access(core: Ctx<'_>, session_id: String) -> Res<WriteAccess> {
    core.write_access(&session_id)
}

#[tauri::command]
#[specta::specta]
async fn writes_unlock(core: Ctx<'_>, session_id: String) -> Res<WriteAccess> {
    core.unlock_writes(&session_id).await
}

#[tauri::command]
#[specta::specta]
async fn writes_lock(core: Ctx<'_>, session_id: String) -> Res<WriteAccess> {
    core.lock_writes(&session_id).await
}

#[tauri::command]
#[specta::specta]
fn tx_state(core: Ctx<'_>, session_id: String) -> Res<TxState> {
    core.transaction(&session_id)
}

#[tauri::command]
#[specta::specta]
fn tx_mode_set(core: Ctx<'_>, session_id: String, mode: CommitMode) -> Res<TxState> {
    core.set_commit_mode(&session_id, mode)
}

#[tauri::command]
#[specta::specta]
async fn tx_commit(core: Ctx<'_>, session_id: String, confirm: Option<String>) -> Res<TxState> {
    core.commit(&session_id, confirm.as_deref()).await
}

#[tauri::command]
#[specta::specta]
async fn tx_rollback(core: Ctx<'_>, session_id: String) -> Res<TxState> {
    core.rollback(&session_id).await
}

#[tauri::command]
#[specta::specta]
fn audit_list(core: Ctx<'_>, connection_id: String, limit: u32) -> Res<Vec<AuditEntry>> {
    core.store(|s| s.audit(&connection_id, limit.min(100_000)))
}

pub fn builder() -> Builder<tauri::Wry> {
    commands().typ::<Schema>()
}

#[allow(clippy::too_many_lines)]
fn commands() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        appearance_get,
        appearance_set,
        er_layout_get,
        er_layout_set,
        connections_list,
        connection_save,
        connection_delete,
        connection_trash,
        connection_restore,
        connection_purge,
        connection_test,
        vars_preview,
        host_key_trust,
        session_open,
        session_switch,
        session_reconnect,
        session_watch,
        visibility_get,
        visibility_set,
        session_close,
        schema_load,
        schema_open,
        schema_namespace,
        search_path,
        object_ddl,
        object_dependencies,
        sql_generate,
        run,
        explain,
        params_found,
        run_cancel,
        history_list,
        plan_history,
        plan_saved,
        write_removals,
        write_linked,
        export_save,
        file_load,
        sql_file_open,
        sql_file_read,
        sql_file_save,
        sensitive_columns,
        report_pdf,
        data_browse,
        data_pick,
        table_identity,
        data_related,
        data_summarize,
        database_counts,
        data_preview,
        data_estimate,
        data_apply,
        documents_write,
        keys_scan,
        key_read,
        key_write,
        server_report,
        write_access,
        writes_unlock,
        writes_lock,
        tx_state,
        tx_mode_set,
        tx_commit,
        tx_rollback,
        audit_list,
        crate::library::scripts_list,
        crate::library::script_save,
        crate::library::script_delete,
        crate::library::snippets_list,
        crate::library::snippet_save,
        crate::library::snippet_delete,
        crate::library::notes_list,
        crate::library::note_set,
        crate::library::preferences_get,
        crate::library::preferences_set,
        crate::library::marks_get,
        crate::library::marks_set,
        crate::library::gen_profiles_get,
        crate::library::gen_profile_save,
        crate::library::gen_profile_delete,
        crate::org::folders_list,
        crate::org::folder_save,
        crate::org::folder_delete,
        crate::org::node_move,
        crate::org::workspaces_get,
        crate::org::workspace_save,
        crate::org::workspace_delete,
        crate::org::workspace_session_save,
        crate::org::workspace_activate,
        crate::org::package_export,
        crate::org::package_preview,
        crate::org::package_import,
        crate::tools::data_export,
        crate::tools::data_import_sample,
        crate::tools::data_import,
        crate::tools::document_import_sample,
        crate::tools::document_import,
        crate::tools::job_cancel,
        crate::tools::schema_compare,
        crate::tools::table_design,
        crate::tools::table_design_script,
        crate::tools::row_probe,
        crate::tools::row_compare,
        crate::tools::row_script,
        crate::tools::row_forget,
        crate::tools::backup_tools,
        crate::tools::tool_dir_set,
        crate::tools::backup_run,
        crate::tools::backup_restore,
        crate::tools::server_activity,
        crate::tools::server_sizes,
        crate::tools::server_kill,
        crate::tools::server_monitor,
        crate::tools::server_slow_queries,
        crate::tools::server_maintain,
        crate::tools::principals_list,
        crate::tools::clickhouse_access,
        crate::tools::security_check,
        crate::tools::value_search_plan,
        crate::tools::value_search,
        crate::tools::gen_profile,
        crate::tools::gen_preview,
        crate::tools::gen_run,
        crate::about::app_info,
        crate::about::open_link,
        crate::about::update_check,
        crate::about::seclab_versions_update,
        crate::about::update_download,
        crate::about::update_install,
        crate::about::update_restart,
        crate::about::update_prefs_get,
        crate::about::update_prefs_set,
    ])
}
