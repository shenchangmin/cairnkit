//! Knowledge base Lint (M6) — contradictions/orphans/stale/duplicates/schema/index.

use crate::errors::Result;
use crate::knowledge::index::{build_index, iter_entries};
use crate::knowledge::lifecycle::months_between;
use crate::knowledge::model::Entry;
use crate::knowledge::schema::iter_errors;
use chrono::{Local, NaiveDate};
use std::collections::BTreeMap;
use std::path::Path;

const STALE_MONTHS: f64 = 12.0;

#[derive(Default)]
pub struct LintReport {
    pub orphans: Vec<String>,
    pub stale: Vec<String>,
    pub duplicates: Vec<Vec<String>>,
    pub invalid: Vec<String>,
    pub conflicts: Vec<Vec<String>>,
    pub fixed: Vec<String>,
}

impl LintReport {
    pub fn clean(&self) -> bool {
        self.orphans.is_empty()
            && self.stale.is_empty()
            && self.duplicates.is_empty()
            && self.invalid.is_empty()
            && self.conflicts.is_empty()
    }
}

pub fn lint(kb_root: &Path, fix: bool, now: Option<NaiveDate>) -> Result<LintReport> {
    let now = now.unwrap_or_else(|| Local::now().date_naive());
    let entries = iter_entries(kb_root);

    let orphans: Vec<String> = entries
        .iter()
        .filter(|e| e.evidence.ref_count == 0 && e.maturity != "draft")
        .map(|e| e.id.clone())
        .collect();

    let stale: Vec<String> = entries
        .iter()
        .filter(|e| {
            e.evidence
                .last_referenced
                .as_deref()
                .map(|d| months_between(d, now) >= STALE_MONTHS)
                .unwrap_or(false)
        })
        .map(|e| e.id.clone())
        .collect();

    let mut invalid = Vec::new();
    for e in &entries {
        let errs = iter_errors(e);
        if !errs.is_empty() {
            invalid.push(format!("{}: {}", e.id, errs.join("; ")));
        }
    }

    let mut by_title: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in &entries {
        by_title
            .entry(e.title.trim().to_lowercase())
            .or_default()
            .push(e.id.clone());
    }
    let duplicates: Vec<Vec<String>> = by_title.into_values().filter(|ids| ids.len() > 1).collect();

    let conflicts = find_conflicts(&entries);

    let mut fixed = Vec::new();
    if fix {
        build_index(kb_root)?;
        fixed.push("rebuilt index".to_string());
    }

    Ok(LintReport {
        orphans,
        stale,
        duplicates,
        invalid,
        conflicts,
        fixed,
    })
}

fn find_conflicts(entries: &[Entry]) -> Vec<Vec<String>> {
    let recs: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.kind == "guideline" && e.guideline_polarity.as_deref() == Some("recommend"))
        .collect();
    let avos: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.kind == "guideline" && e.guideline_polarity.as_deref() == Some("avoid"))
        .collect();
    let mut out = Vec::new();
    for r in &recs {
        for a in &avos {
            let rt: std::collections::BTreeSet<&String> = r.tags.iter().collect();
            let at: std::collections::BTreeSet<&String> = a.tags.iter().collect();
            if !rt.is_empty() && rt == at {
                out.push(vec![r.id.clone(), a.id.clone()]);
            }
        }
    }
    out
}
