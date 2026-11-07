use nd_sql::RemovalKind;
use serde::{Deserialize, Serialize};

use crate::data::count_rows;
use crate::guard::{Access, claim};
use crate::run::{RunScope, select};
use crate::session::Sql as _;
use crate::{Core, Result};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct Removing {
    pub index: u32,
    pub kind: RemovalKind,
    pub namespace: String,
    pub name: String,
    #[specta(type = Option<f64>)]
    pub rows: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct Referencing {
    pub namespace: String,
    pub table: String,
    pub columns: Vec<String>,
    pub ref_columns: Vec<String>,
}

impl Core {
    pub async fn removals(
        &self,
        session_id: &str,
        text: &str,
        scope: &RunScope,
    ) -> Result<Vec<Removing>> {
        let session = self.live(session_id).await?;
        let _busy = claim(&session)?;
        let dialect = session.conn.sql()?;
        self.admit(&session, Access::Read).await?;
        let mut found = Vec::new();
        for (i, st) in select(text, dialect, scope).iter().enumerate() {
            if let Some(r) = nd_sql::removal(st.text(text), dialect) {
                found.push((u32::try_from(i).unwrap_or(u32::MAX), r));
            }
        }
        let queries: Vec<_> = found
            .iter()
            .map(|(_, r)| (r.count(dialect).unwrap_or_default(), Vec::new()))
            .collect();
        let counts = count_rows(&session, &queries).await;
        Ok(found
            .into_iter()
            .zip(counts)
            .map(|((index, r), rows)| Removing {
                index,
                kind: r.kind,
                namespace: r.relation.0,
                name: r.relation.1,
                rows,
            })
            .collect())
    }

    pub async fn linked_rows(
        &self,
        session_id: &str,
        text: &str,
        scope: &RunScope,
        index: u32,
        links: &[Referencing],
    ) -> Result<Vec<Option<u64>>> {
        let session = self.live(session_id).await?;
        let _busy = claim(&session)?;
        let dialect = session.conn.sql()?;
        self.admit(&session, Access::Read).await?;
        let statements = select(text, dialect, scope);
        let Some(r) = statements
            .get(index as usize)
            .and_then(|st| nd_sql::removal(st.text(text), dialect))
        else {
            return Ok(vec![None; links.len()]);
        };
        let queries: Vec<_> = links
            .iter()
            .map(|l| {
                let sql = r
                    .linked(
                        dialect,
                        (&l.namespace, &l.table),
                        &l.columns,
                        &l.ref_columns,
                    )
                    .unwrap_or_default();
                (sql, Vec::new())
            })
            .collect();
        Ok(count_rows(&session, &queries).await)
    }
}
