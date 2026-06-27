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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::{save_entry, Evidence};
    use tempfile::tempdir;

    const NOW_YMD: (i32, u32, u32) = (2026, 6, 1);

    fn now() -> NaiveDate {
        NaiveDate::from_ymd_opt(NOW_YMD.0, NOW_YMD.1, NOW_YMD.2).unwrap()
    }

    /// Schema-valid entry with the given knobs; defaults keep it lint-clean.
    #[allow(clippy::too_many_arguments)]
    fn write_entry(
        root: &Path,
        id: &str,
        title: &str,
        kind: &str,
        polarity: Option<&str>,
        maturity: &str,
        tags: &[&str],
        ev: Evidence,
    ) {
        let entry = Entry {
            id: id.to_string(),
            title: title.to_string(),
            category: "tech".to_string(),
            domain: None,
            kind: kind.to_string(),
            guideline_polarity: polarity.map(String::from),
            maturity: maturity.to_string(),
            knowledge_class: "point".to_string(),
            layer: "L1".to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            applicable_phases: vec!["IMPLEMENT".to_string()],
            evidence: ev,
            history: vec![],
            body: "body".to_string(),
            path: None,
        };
        let path = root.join("tech-wiki").join(format!("{id}.md"));
        save_entry(&path, &entry).unwrap();
    }

    // R7a: a non-draft entry with zero refs is an orphan.
    #[test]
    fn detects_orphan_non_draft_zero_refs() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-ORPH",
            "orphan title",
            "decision",
            None,
            "verified",
            &[],
            Evidence::default(),
        );
        let report = lint(dir.path(), false, Some(now())).unwrap();
        assert!(report.orphans.contains(&"TK-ORPH".to_string()));
        assert!(!report.clean());
    }

    // R7b: two entries with the same lowercased trimmed title form a duplicate group.
    #[test]
    fn detects_duplicate_same_lowercased_title() {
        let dir = tempdir().unwrap();
        // keep them draft so the orphan rule does not also fire.
        write_entry(
            dir.path(),
            "TK-D1",
            "Same Title",
            "decision",
            None,
            "draft",
            &[],
            Evidence::default(),
        );
        write_entry(
            dir.path(),
            "TK-D2",
            "same title",
            "decision",
            None,
            "draft",
            &[],
            Evidence::default(),
        );
        let report = lint(dir.path(), false, Some(now())).unwrap();
        let group = report
            .duplicates
            .iter()
            .find(|g| g.len() == 2)
            .expect("a duplicate group of two");
        assert!(group.contains(&"TK-D1".to_string()));
        assert!(group.contains(&"TK-D2".to_string()));
    }

    // R7c: two guidelines, equal non-empty tags, opposite polarity -> a contradiction.
    #[test]
    fn detects_contradiction_recommend_vs_avoid_same_tags() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-REC",
            "rec title",
            "guideline",
            Some("recommend"),
            "draft",
            &["x", "y"],
            Evidence::default(),
        );
        write_entry(
            dir.path(),
            "TK-AVO",
            "avo title",
            "guideline",
            Some("avoid"),
            "draft",
            &["x", "y"],
            Evidence::default(),
        );
        let report = lint(dir.path(), false, Some(now())).unwrap();
        assert!(report
            .conflicts
            .contains(&vec!["TK-REC".to_string(), "TK-AVO".to_string()]));
    }

    // R7d: last_referenced more than 12 months before `now` is stale.
    #[test]
    fn detects_stale_last_referenced_over_12_months() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-STALE",
            "stale title",
            "decision",
            None,
            "draft",
            &[],
            Evidence {
                last_referenced: Some("2025-01-01".to_string()),
                ref_count: 1,
                ..Default::default()
            },
        );
        let report = lint(dir.path(), false, Some(now())).unwrap();
        assert!(report.stale.contains(&"TK-STALE".to_string()));
    }

    // R7e: one well-formed draft (orphan-exempt), schema-valid, recent ref -> clean.
    #[test]
    fn clean_kb_reports_clean() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-OK",
            "ok title",
            "decision",
            None,
            "draft",
            &[],
            Evidence {
                last_referenced: Some("2026-05-01".to_string()),
                ref_count: 1,
                ..Default::default()
            },
        );
        let report = lint(dir.path(), false, Some(now())).unwrap();
        assert!(
            report.clean(),
            "expected clean, got: orphans={:?} stale={:?} dup={:?} invalid={:?} conflicts={:?}",
            report.orphans,
            report.stale,
            report.duplicates,
            report.invalid,
            report.conflicts
        );
    }
}
