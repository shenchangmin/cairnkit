//! Reference-tracking closed loop (M6) — the engine of maturity.

use crate::knowledge::index::iter_entries;
use crate::knowledge::model::save_entry;
use chrono::Local;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Yield each balanced top-level {...} substring (robust to nested objects).
fn top_level_json_objects(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    for (i, ch) in text.char_indices() {
        if ch == '{' {
            if depth == 0 {
                start = Some(i);
            }
            depth += 1;
        } else if ch == '}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                if let Some(s) = start.take() {
                    out.push(text[s..i + ch.len_utf8()].to_string());
                }
            }
        }
    }
    out
}

fn ids_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if k == "knowledgeReferences" {
                    if let Some(arr) = val.as_array() {
                        for r in arr {
                            if let Some(id) = r.get("id").and_then(|x| x.as_str()) {
                                out.push(id.to_string());
                            }
                        }
                    }
                } else {
                    ids_in(val, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| ids_in(x, out)),
        _ => {}
    }
}

pub fn collect_references(run_dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    if !run_dir.exists() {
        return ids;
    }
    let mut files: Vec<_> = std::fs::read_dir(run_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    files.sort();
    for path in files {
        if let Ok(text) = std::fs::read_to_string(&path) {
            for blob in top_level_json_objects(&text) {
                if let Ok(v) = serde_json::from_str::<Value>(&blob) {
                    ids_in(&v, &mut ids);
                }
            }
        }
    }
    ids
}

pub fn touch(kb_root: &Path, run_dir: &Path, project: &str, today: Option<&str>) -> Value {
    let today = today.map(String::from).unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let referenced = collect_references(run_dir);
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for id in &referenced {
        *counts.entry(id.clone()).or_insert(0) += 1;
    }

    let mut updated = Vec::new();
    let entries = iter_entries(kb_root);
    for (id, n) in &counts {
        if let Some(entry) = entries.iter().find(|e| &e.id == id) {
            if let Some(path) = &entry.path {
                let mut new = entry.clone();
                if !new.evidence.projects.iter().any(|p| p == project) {
                    new.evidence.projects.push(project.to_string());
                }
                new.evidence.last_referenced = Some(today.clone());
                new.evidence.ref_count += n;
                if save_entry(path, &new).is_ok() {
                    updated.push(id.clone());
                }
            }
        }
    }
    let unknown: Vec<&String> = counts.keys().filter(|k| !updated.contains(k)).collect();
    json!({"referenced": referenced, "updated": updated, "unknown": unknown})
}
