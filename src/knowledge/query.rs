//! Budget-bounded progressive knowledge injection (M5).

use crate::knowledge::index::iter_entries;
use crate::knowledge::model::{serialize_entry, Entry};
use crate::knowledge::{class_rank, maturity_rank};
use serde_json::{json, Value};
use std::path::Path;

pub struct QueryResult {
    pub stage: String,
    pub budget_lines: i64,
    pub injected_ids: Vec<String>,
    pub dropped: Vec<Value>,
    pub text: String,
    pub lines: i64,
    pub over_budget: bool,
}

fn applies(e: &Entry, stage: &str, domain: Option<&str>) -> bool {
    if !e.applicable_phases.iter().any(|p| p == stage) {
        return false;
    }
    if e.category == "tech" {
        return true; // L1 tech knowledge is globally visible
    }
    domain.is_some() && e.domain.as_deref() == domain
}

pub fn query(kb_root: &Path, stage: &str, budget_lines: i64, domain: Option<&str>) -> QueryResult {
    let mut candidates: Vec<Entry> = iter_entries(kb_root)
        .into_iter()
        .filter(|e| applies(e, stage, domain))
        .collect();
    candidates.sort_by(|a, b| {
        (-maturity_rank(&a.maturity), -class_rank(&a.knowledge_class), &a.id)
            .cmp(&(-maturity_rank(&b.maturity), -class_rank(&b.knowledge_class), &b.id))
    });

    let mut injected: Vec<String> = Vec::new();
    let mut injected_ids: Vec<String> = Vec::new();
    let mut dropped: Vec<Value> = Vec::new();
    let mut used: i64 = 0;
    let mut over_budget = false;

    for e in &candidates {
        let block = serialize_entry(e);
        let block_lines = block.matches('\n').count() as i64;
        if injected.is_empty() {
            // always include the single top-ranked entry; flag (never silently) if it alone exceeds budget
            injected.push(block);
            injected_ids.push(e.id.clone());
            used += block_lines;
            over_budget = block_lines > budget_lines;
        } else if used + block_lines > budget_lines {
            dropped.push(json!({"id": e.id, "title": e.title, "reason": "budget"}));
        } else {
            injected.push(block);
            injected_ids.push(e.id.clone());
            used += block_lines;
        }
    }

    QueryResult {
        stage: stage.to_string(),
        budget_lines,
        injected_ids,
        dropped,
        text: injected.join("\n"),
        lines: used,
        over_budget,
    }
}
