use std::path::PathBuf;
use std::time::Instant;

use nd_transfer::{Doc, DocLayout, Unit, read_documents};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::{OnRowError, Progress, RowFault, ToolFault, ms};
use crate::guard::{Access, claim};
use crate::{Core, CoreError, Result};

const PREVIEW: usize = 20;
const MAX_FAULTS: usize = 500;
const BATCH: usize = 500;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DocumentSample {
    pub layout: DocLayout,
    #[specta(type = f64)]
    pub count: u64,
    pub preview: Vec<String>,
    pub faults: Vec<RowFault>,
    pub faults_cut: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct DocumentImport {
    pub job_id: String,
    pub session_id: String,
    pub path: String,
    pub database: String,
    pub collection: String,
    pub on_error: OnRowError,
    pub confirm: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DocumentsImported {
    #[specta(type = f64)]
    pub inserted: u64,
    #[specta(type = f64)]
    pub skipped: u64,
    pub faults: Vec<RowFault>,
    pub faults_cut: bool,
    pub stopped: bool,
    pub cancelled: bool,
    #[specta(type = f64)]
    pub elapsed_ms: u64,
}

impl DocumentsImported {
    fn fault(&mut self, line: u64, message: String) {
        self.skipped += 1;
        if self.faults.len() < MAX_FAULTS {
            self.faults.push(RowFault { line, message });
        } else {
            self.faults_cut = true;
        }
    }

    async fn write(
        &mut self,
        conn: &dyn nd_driver::Connection,
        req: &DocumentImport,
        docs: Vec<Doc>,
    ) -> std::result::Result<(), nd_driver::DriverError> {
        let stop = req.on_error == OnRowError::Stop;
        let mut texts = Vec::with_capacity(docs.len());
        let mut lines = Vec::with_capacity(docs.len());
        for d in docs {
            match d.text {
                Ok(t) => {
                    texts.push(t);
                    lines.push(d.n);
                }
                Err(message) => {
                    self.fault(d.n, message);
                    if stop {
                        self.stopped = true;
                        break;
                    }
                }
            }
        }
        if texts.is_empty() {
            return Ok(());
        }
        let results = conn
            .insert_documents(&req.database, &req.collection, &texts)
            .await?;
        for (line, r) in lines.into_iter().zip(results) {
            match r {
                None => self.inserted += 1,
                Some(message) => {
                    self.fault(line, message);
                    self.stopped |= stop;
                }
            }
        }
        Ok(())
    }
}

fn document_path(path: &str) -> Result<PathBuf> {
    let p = PathBuf::from(path);
    let ok = p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["json", "ndjson", "jsonl"]
            .iter()
            .any(|x| e.eq_ignore_ascii_case(x))
    });
    if ok {
        Ok(p)
    } else {
        Err(ToolFault::WrongExtension.into())
    }
}

fn file_fault(e: impl std::fmt::Display) -> CoreError {
    ToolFault::File {
        message: e.to_string(),
    }
    .into()
}

impl Core {
    pub async fn document_sample(&self, path: &str) -> Result<DocumentSample> {
        let path = document_path(path)?;
        tokio::task::spawn_blocking(move || {
            let mut out = DocumentSample {
                layout: DocLayout::Lines,
                count: 0,
                preview: Vec::new(),
                faults: Vec::new(),
                faults_cut: false,
            };
            out.layout = read_documents(&path, BATCH, &mut |docs, _| {
                for d in docs {
                    match d.text {
                        Ok(text) => {
                            out.count += 1;
                            if out.preview.len() < PREVIEW {
                                let pretty = serde_json::from_str::<serde_json::Value>(&text)
                                    .and_then(|v| serde_json::to_string_pretty(&v))
                                    .unwrap_or(text);
                                out.preview.push(pretty);
                            }
                        }
                        Err(message) if out.faults.len() < MAX_FAULTS => {
                            out.faults.push(RowFault { line: d.n, message });
                        }
                        Err(_) => out.faults_cut = true,
                    }
                }
                true
            })
            .map_err(file_fault)?;
            Ok(out)
        })
        .await
        .map_err(|e| CoreError::Store(e.to_string()))?
    }

    pub async fn import_documents(
        &self,
        req: DocumentImport,
        progress: impl Fn(Progress) + Send + Sync,
    ) -> Result<DocumentsImported> {
        let started = Instant::now();
        let path = document_path(&req.path)?;
        if req.collection.trim().is_empty() {
            return Err(ToolFault::Empty.into());
        }
        let session = self.live(&req.session_id).await?;
        if !session.caps.editable || session.caps.dialect().is_some() {
            return Err(ToolFault::Unsupported.into());
        }
        let _busy = claim(&session)?;
        self.admit(
            &session,
            Access::Write {
                confirm: req.confirm.as_deref(),
            },
        )
        .await?;
        let slot = self.job_start(&req.job_id)?;
        let (tx, mut rx) = mpsc::channel::<(Vec<Doc>, nd_transfer::Seen)>(2);
        let reader = tokio::task::spawn_blocking(move || {
            read_documents(&path, BATCH, &mut |docs, seen| {
                tx.blocking_send((docs, seen)).is_ok()
            })
        });
        let mut out = DocumentsImported {
            inserted: 0,
            skipped: 0,
            faults: Vec::new(),
            faults_cut: false,
            stopped: false,
            cancelled: false,
            elapsed_ms: 0,
        };
        let mut failure = None;
        while let Some((docs, (done, total))) = rx.recv().await {
            if slot.job.stopped() {
                out.cancelled = true;
                break;
            }
            if let Err(e) = out.write(session.conn.as_ref(), &req, docs).await {
                failure = Some(e);
                break;
            }
            progress(Progress {
                done,
                total: Some(total),
                unit: Unit::Bytes,
                rows: out.inserted,
                bytes: done,
                stage: None,
            });
            if out.stopped {
                break;
            }
        }
        drop(rx);
        let read = reader.await.map_err(|e| CoreError::Store(e.to_string()))?;
        out.elapsed_ms = ms(started);
        let shown = format!(
            "import {} documents into {}{} from {}",
            out.inserted,
            if req.database.is_empty() {
                String::new()
            } else {
                format!("{}.", req.database)
            },
            req.collection,
            req.path
        );
        let error = failure.as_ref().map(ToString::to_string);
        self.audit(&session, &shown, Some(out.inserted), error);
        if let Some(e) = failure {
            return Err(e.into());
        }
        if let Err(e) = read {
            if out.inserted == 0 && out.faults.is_empty() {
                return Err(file_fault(e));
            }
            if out.faults.len() < MAX_FAULTS {
                out.faults.push(RowFault {
                    line: 0,
                    message: e.to_string(),
                });
            }
        }
        Ok(out)
    }
}
