use nd_plan::Plan;
use nd_store::{NewPlan, SavedPlan};

use crate::{Core, PlanReport, Result};

pub fn fingerprint(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut it = sql.chars().peekable();
    let space = |out: &mut String| {
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    };
    while let Some(c) = it.next() {
        match c {
            '-' if it.peek() == Some(&'-') => {
                for n in it.by_ref() {
                    if n == '\n' {
                        break;
                    }
                }
                space(&mut out);
            }
            '/' if it.peek() == Some(&'*') => {
                it.next();
                let mut last = ' ';
                for n in it.by_ref() {
                    if last == '*' && n == '/' {
                        break;
                    }
                    last = n;
                }
                space(&mut out);
            }
            '\'' => {
                while let Some(n) = it.next() {
                    if n == '\'' {
                        if it.peek() == Some(&'\'') {
                            it.next();
                        } else {
                            break;
                        }
                    }
                }
                out.push('?');
            }
            '"' | '`' => {
                out.push(c);
                for n in it.by_ref() {
                    out.extend(n.to_lowercase());
                    if n == c {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit()
                && !out.ends_with(|p: char| {
                    p.is_alphanumeric() || p == '_' || p == '$' || p == '?'
                }) =>
            {
                while it
                    .peek()
                    .is_some_and(|n| n.is_ascii_alphanumeric() || *n == '.')
                {
                    it.next();
                }
                out.push('?');
            }
            c if c.is_whitespace() => space(&mut out),
            c => out.extend(c.to_lowercase()),
        }
    }
    let out = out.trim_end().trim_end_matches(';').trim_end();
    let word = |c: char| c.is_alphanumeric() || matches!(c, '_' | '?' | '$' | '"' | '`');
    let chars: Vec<char> = out.chars().collect();
    chars
        .iter()
        .enumerate()
        .filter(|&(i, &c)| {
            c != ' ' || (i > 0 && word(chars[i - 1]) && chars.get(i + 1).is_some_and(|&n| word(n)))
        })
        .map(|(_, &c)| c)
        .collect()
}

fn shape(plan: &Plan) -> String {
    plan.nodes
        .iter()
        .map(|n| {
            format!(
                "{} {} {} {}",
                n.depth,
                n.op,
                n.relation.as_deref().unwrap_or(""),
                n.index.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn total(plan: &Plan) -> Option<f64> {
    if plan.analyzed {
        plan.total_ms
            .or_else(|| plan.nodes.first().and_then(|n| n.time_ms))
    } else {
        plan.nodes.first().and_then(|n| n.total_cost)
    }
}

impl Core {
    pub(crate) fn keep_plan(&self, connection_id: &str, database: &str, report: &PlanReport) {
        if report.unreadable.is_some() || report.plan.nodes.is_empty() {
            return;
        }
        let Ok(json) = serde_json::to_string(report) else {
            return;
        };
        let fp = fingerprint(&report.sql);
        let shape = shape(&report.plan);
        let kept = self.store(|s| {
            s.add_plan(&NewPlan {
                connection_id,
                database,
                fingerprint: &fp,
                sql: &report.sql,
                at: report.at,
                analyzed: report.plan.analyzed,
                total: total(&report.plan),
                shape: &shape,
                report: &json,
            })
        });
        if let Err(e) = kept {
            tracing::warn!(error = %e, "plan not kept");
        }
    }

    pub fn plan_history(
        &self,
        connection_id: &str,
        database: &str,
        sql: &str,
    ) -> Result<Vec<SavedPlan>> {
        self.store(|s| s.plans(connection_id, database, &fingerprint(sql)))
    }

    pub fn plan_saved(&self, id: i64) -> Result<Option<String>> {
        self.store(|s| s.plan_report(id))
    }
}
