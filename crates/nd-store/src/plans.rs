use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::{Result, Store};

pub const PLANS_KEPT: u32 = 50;

pub struct NewPlan<'a> {
    pub connection_id: &'a str,
    pub database: &'a str,
    pub fingerprint: &'a str,
    pub sql: &'a str,
    pub at: i64,
    pub analyzed: bool,
    pub total: Option<f64>,
    pub shape: &'a str,
    pub report: &'a str,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
pub struct SavedPlan {
    #[specta(type = f64)]
    pub id: i64,
    #[specta(type = f64)]
    pub at: i64,
    pub analyzed: bool,
    pub total: Option<f64>,
    pub shape_changed: bool,
}

impl Store {
    pub fn add_plan(&self, p: &NewPlan<'_>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO plans (connection_id, database, fingerprint, sql, at, analyzed, total, shape, report)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![p.connection_id, p.database, p.fingerprint, p.sql, p.at, p.analyzed, p.total, p.shape, p.report],
        )?;
        let id = self.conn.last_insert_rowid();
        self.conn.execute(
            "DELETE FROM plans WHERE connection_id = ?1 AND database = ?2 AND fingerprint = ?3 AND id NOT IN
               (SELECT id FROM plans WHERE connection_id = ?1 AND database = ?2 AND fingerprint = ?3 ORDER BY at DESC, id DESC LIMIT ?4)",
            params![p.connection_id, p.database, p.fingerprint, PLANS_KEPT],
        )?;
        Ok(id)
    }

    pub fn plans(
        &self,
        connection_id: &str,
        database: &str,
        fingerprint: &str,
    ) -> Result<Vec<SavedPlan>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, at, analyzed, total, shape FROM plans
             WHERE connection_id = ?1 AND database = ?2 AND fingerprint = ?3 ORDER BY at DESC, id DESC",
        )?;
        let rows: Vec<(SavedPlan, String)> = stmt
            .query_map(params![connection_id, database, fingerprint], |r| {
                Ok((
                    SavedPlan {
                        id: r.get(0)?,
                        at: r.get(1)?,
                        analyzed: r.get(2)?,
                        total: r.get(3)?,
                        shape_changed: false,
                    },
                    r.get(4)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let shapes: Vec<&str> = rows.iter().map(|(_, s)| s.as_str()).collect();
        Ok(rows
            .iter()
            .enumerate()
            .map(|(i, (p, shape))| SavedPlan {
                shape_changed: shapes.get(i + 1).is_some_and(|older| older != shape),
                ..p.clone()
            })
            .collect())
    }

    pub fn plan_report(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT report FROM plans WHERE id = ?1", [id], |r| r.get(0))
            .optional()?)
    }
}
