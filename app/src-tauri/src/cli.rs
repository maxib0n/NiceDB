use std::io::Write;
use std::sync::{Arc, Mutex};

use nd_core::{Core, CoreError, RunConfirm, RunEvent, RunRequest, RunScope};
use nd_net::KnownHosts;
use nd_store::Store;

const USAGE: &str = "usage: nicedb run --conn <connection> [--db <database>] [--csv | --tsv] [--confirm <connection name>] [--risky] <file.sql | ->

Runs the statements of the file (or standard input with -) on a connection saved in NiceDB,
by name or id, with its saved password from the keychain. Results go to standard output as CSV
(or TSV), one block per statement that returns rows; messages go to standard error.

  --conn      the connection, as named in NiceDB
  --db        another database of the same server
  --csv       comma-separated values (the default)
  --tsv       tab-separated values
  --confirm   on a production connection, opens writes and confirms them with the connection's name
  --risky     runs statements flagged as risky (DROP, TRUNCATE, DELETE without WHERE…)

Exit status: 0 when every statement ran, 1 when one failed, 2 when the command was wrong.";

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    conn: String,
    database: Option<String>,
    tsv: bool,
    confirm: Option<String>,
    risky: bool,
    file: String,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match a.as_str() {
            "--conn" | "-c" => out.conn = value("--conn")?,
            "--db" => out.database = Some(value("--db")?),
            "--csv" => out.tsv = false,
            "--tsv" => out.tsv = true,
            "--confirm" => out.confirm = Some(value("--confirm")?),
            "--risky" => out.risky = true,
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s if out.file.is_empty() => s.clone_into(&mut out.file),
            s => return Err(format!("one file only, {s} is extra")),
        }
    }
    if out.conn.is_empty() || out.file.is_empty() {
        return Err("--conn and a file are needed".into());
    }
    Ok(out)
}

pub fn usage(args: &[String]) -> Option<(i32, String)> {
    match parse(args) {
        Ok(_) => None,
        Err(why) if why.is_empty() => Some((0, USAGE.to_owned())),
        Err(why) => Some((2, format!("nicedb run: {why}\n\n{USAGE}"))),
    }
}

pub fn data_dir() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("app.nicedb.desktop"))
}

pub fn open_core() -> Result<Core, String> {
    let dir = data_dir().ok_or("no data folder for this user")?;
    std::fs::create_dir_all(&dir)
        .and_then(|()| crate::keep_private(&dir))
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    let store = Store::open(&dir.join("nicedb.sqlite")).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    if let Some(home) = dirs::home_dir() {
        files.push(home.join(".ssh/known_hosts"));
    }
    files.push(dir.join("known_hosts"));
    Ok(Core::new(
        store,
        Arc::new(nd_secrets::Keychain),
        KnownHosts { files },
    ))
}

fn connection_id(core: &Core, wanted: &str) -> Result<String, String> {
    let all = core.store(Store::connections).map_err(|e| e.to_string())?;
    if let Some(c) = all.iter().find(|c| c.id == wanted || c.name == wanted) {
        return Ok(c.id.clone());
    }
    let like: Vec<_> = all
        .iter()
        .filter(|c| c.name.to_lowercase() == wanted.to_lowercase())
        .collect();
    match like.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => Err(format!(
            "no connection named \"{wanted}\"; saved: {}",
            if all.is_empty() {
                "none".to_owned()
            } else {
                all.iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        )),
        _ => Err(format!(
            "more than one connection is named \"{wanted}\": use its id"
        )),
    }
}

fn explain(e: &CoreError) -> String {
    match e {
        CoreError::ReadOnly => {
            "production connection: add --confirm \"<connection name>\" to write".into()
        }
        CoreError::NeedsConfirmation | CoreError::NameMismatch => {
            "production connection: --confirm must be the connection's name".into()
        }
        CoreError::Risky(r) => format!(
            "{} risky statement(s), starting at line text \"{}\": add --risky to run them",
            r.len(),
            r.first().map(|x| x.sql.as_str()).unwrap_or_default()
        ),
        e => e.to_string(),
    }
}

pub async fn run(
    core: &Core,
    args: &[String],
    input: &str,
    out: &mut (dyn Write + Send),
    err: &mut (dyn Write + Send),
) -> i32 {
    let a = match parse(args) {
        Ok(a) => a,
        Err(why) => {
            if !why.is_empty() {
                let _ = writeln!(err, "nicedb run: {why}\n");
            }
            let _ = writeln!(err, "{USAGE}");
            return 2;
        }
    };
    let text = if a.file == "-" {
        input.to_owned()
    } else {
        match std::fs::read_to_string(&a.file) {
            Ok(t) => t,
            Err(e) => {
                let _ = writeln!(err, "nicedb run: {}: {e}", a.file);
                return 2;
            }
        }
    };
    let id = match connection_id(core, &a.conn) {
        Ok(id) => id,
        Err(why) => {
            let _ = writeln!(err, "nicedb run: {why}");
            return 2;
        }
    };
    let mut session = match core.open_session(&id).await {
        Ok(s) => s.session_id,
        Err(e) => {
            let _ = writeln!(err, "nicedb run: {}", explain(&e));
            return 1;
        }
    };
    if let Some(db) = &a.database {
        match core.switch_database(&session, db).await {
            Ok(s) => {
                let _ = core.close_session(&session);
                session = s.session_id;
            }
            Err(e) => {
                let _ = writeln!(err, "nicedb run: {}", explain(&e));
                let _ = core.close_session(&session);
                return 1;
            }
        }
    }
    if a.confirm.is_some()
        && core.write_access(&session).is_ok_and(|w| w.guarded)
        && let Err(e) = core.unlock_writes(&session).await
    {
        let _ = writeln!(err, "nicedb run: {}", explain(&e));
    }
    let status = run_text(core, &session, &text, &a, out, err).await;
    let _ = core.close_session(&session);
    status
}

async fn run_text(
    core: &Core,
    session: &str,
    text: &str,
    a: &Args,
    out: &mut (dyn Write + Send),
    err: &mut (dyn Write + Send),
) -> i32 {
    let sink = Mutex::new(Sink {
        out: Vec::new(),
        csv: None,
        delimiter: if a.tsv { b'\t' } else { b',' },
        messages: Vec::new(),
        failed: false,
    });
    let request = RunRequest {
        run_id: format!("cli-{}", std::process::id()),
        session_id: session.to_owned(),
        text: text.to_owned(),
        scope: RunScope::Script,
        confirm: RunConfirm {
            risks: a.risky,
            name: a.confirm.clone(),
        },
        sort: Vec::new(),
        all_rows: true,
        dry_run: false,
        continue_on_error: false,
        params: None,
    };
    let ran = core
        .run(request, |e| {
            if let Ok(mut s) = sink.lock() {
                s.take(e);
            }
        })
        .await;
    let Ok(mut s) = sink.into_inner() else {
        return 1;
    };
    s.end_block();
    let _ = out.write_all(&s.out);
    let _ = out.flush();
    for m in s.messages.drain(..) {
        let _ = writeln!(err, "{m}");
    }
    match ran {
        Err(e) => {
            let _ = writeln!(err, "nicedb run: {}", explain(&e));
            1
        }
        Ok(()) if s.failed => 1,
        Ok(()) => 0,
    }
}

struct Sink {
    out: Vec<u8>,
    csv: Option<csv::Writer<Vec<u8>>>,
    delimiter: u8,
    messages: Vec<String>,
    failed: bool,
}

impl Sink {
    fn end_block(&mut self) {
        if let Some(bytes) = self.csv.take().and_then(|w| w.into_inner().ok()) {
            if !self.out.is_empty() {
                self.out.push(b'\n');
            }
            self.out.extend(bytes);
        }
    }

    fn block(&mut self) -> &mut csv::Writer<Vec<u8>> {
        let delimiter = self.delimiter;
        self.csv.get_or_insert_with(|| {
            csv::WriterBuilder::new()
                .delimiter(delimiter)
                .flexible(true)
                .from_writer(Vec::new())
        })
    }

    fn take(&mut self, e: RunEvent) {
        match e {
            RunEvent::Columns { columns, .. } => {
                self.end_block();
                let _ = self
                    .block()
                    .write_record(columns.iter().map(|c| c.name.as_str()));
            }
            RunEvent::Rows { rows, .. } => {
                for r in rows {
                    let _ = self
                        .block()
                        .write_record(r.iter().map(|v| v.as_deref().unwrap_or("")));
                }
            }
            RunEvent::Documents { documents, .. } => {
                for d in documents {
                    let _ = self.block().write_record([d]);
                }
            }
            RunEvent::Done {
                index,
                rows_affected: Some(n),
                command,
                ..
            } => self.messages.push(format!(
                "statement {}: {} {n} row(s)",
                index + 1,
                command.as_deref().unwrap_or("affected")
            )),
            RunEvent::Done {
                index,
                truncated: true,
                row_limit,
                ..
            } => self.messages.push(format!(
                "statement {}: result cut at the connection's row limit{}",
                index + 1,
                row_limit.map(|n| format!(" ({n})")).unwrap_or_default()
            )),
            RunEvent::Failed { index, error, .. } => {
                self.failed = true;
                self.messages
                    .push(format!("statement {} failed: {error}", index + 1));
            }
            RunEvent::Lost { index, error, .. } => {
                self.failed = true;
                self.messages
                    .push(format!("statement {}: connection lost: {error}", index + 1));
            }
            _ => {}
        }
    }
}
