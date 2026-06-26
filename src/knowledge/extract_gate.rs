//! Strict extraction gate (M6) — keep the knowledge base small and precise.

use crate::knowledge::model::{save_entry, Entry, Evidence};
use crate::knowledge::schema::iter_errors;
use crate::knowledge::{KNOWLEDGE_CLASSES, TYPES};
use chrono::Local;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MIN_BODY_CHARS: usize = 80;
pub const CANDIDATES_FILE: &str = "knowledge-candidates.json";

/// Decide whether a candidate dict passes the strict gate; return the rejection reasons.
pub fn evaluate(c: &Value) -> Vec<String> {
    let mut reasons = Vec::new();
    let s = |k: &str| c.get(k).and_then(|v| v.as_str()).unwrap_or("");
    if s("id").trim().is_empty() {
        reasons.push("missing id".to_string());
    }
    if s("body").trim().chars().count() < MIN_BODY_CHARS {
        reasons.push("insufficient depth (body too short)".to_string());
    }
    let phases_ok = c
        .get("applicable_phases")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if !phases_ok {
        reasons.push("not transferable (no applicable_phases)".to_string());
    }
    if !TYPES.contains(&s("type")) {
        reasons.push("missing/invalid type".to_string());
    }
    let kc = c.get("knowledge_class").and_then(|v| v.as_str()).unwrap_or("point");
    if !KNOWLEDGE_CLASSES.contains(&kc) {
        reasons.push("invalid knowledge_class".to_string());
    }
    if s("title").trim().is_empty() {
        reasons.push("missing title".to_string());
    }
    reasons
}

fn str_field(c: &Value, k: &str, default: &str) -> String {
    c.get(k).and_then(|v| v.as_str()).unwrap_or(default).to_string()
}
fn vec_field(c: &Value, k: &str) -> Vec<String> {
    c.get(k)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn candidate_to_entry(c: &Value, now: &str) -> Entry {
    let domain = c.get("domain").and_then(|v| v.as_str()).map(String::from);
    let polarity = c.get("guideline_polarity").and_then(|v| v.as_str()).map(String::from);
    let mut hist = serde_yaml::Mapping::new();
    hist.insert("date".into(), now.into());
    hist.insert("update_type".into(), "extract".into());
    hist.insert("by".into(), "archiver".into());
    let kc = {
        let v = str_field(c, "knowledge_class", "point");
        if v.is_empty() { "point".to_string() } else { v }
    };
    Entry {
        id: str_field(c, "id", ""),
        title: str_field(c, "title", ""),
        category: str_field(c, "category", ""),
        domain,
        kind: str_field(c, "type", ""),
        guideline_polarity: polarity,
        maturity: "draft".to_string(), // extraction always lands as draft
        knowledge_class: kc,
        layer: str_field(c, "layer", "L3"),
        tags: vec_field(c, "tags"),
        applicable_phases: vec_field(c, "applicable_phases"),
        evidence: Evidence { contributors: vec_field(c, "contributors"), ..Default::default() },
        history: vec![serde_yaml::Value::Mapping(hist)],
        body: str_field(c, "body", ""),
        path: None,
    }
}

fn entry_path(kb_root: &Path, e: &Entry) -> PathBuf {
    if e.category == "biz" {
        kb_root.join("biz-wiki").join(e.domain.clone().unwrap_or_else(|| "_".to_string())).join(format!("{}.md", e.id))
    } else {
        kb_root.join("tech-wiki").join(format!("{}.md", e.id))
    }
}

pub fn extract_from_run(run_dir: &Path, kb_root: &Path) -> Value {
    let now = Local::now().format("%Y-%m-%d").to_string();
    let cf = run_dir.join(CANDIDATES_FILE);
    if !cf.exists() {
        return json!({"written": [], "rejected": [], "note": format!("no {CANDIDATES_FILE} in {}", run_dir.display())});
    }
    let text = match std::fs::read_to_string(&cf) {
        Ok(t) => t,
        Err(e) => return json!({"written": [], "rejected": [], "error": e.to_string()}),
    };
    let candidates: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return json!({"written": [], "rejected": [], "error": format!("malformed {CANDIDATES_FILE}: {e}")}),
    };
    let arr = match candidates.as_array() {
        Some(a) => a,
        None => return json!({"written": [], "rejected": [], "error": format!("{CANDIDATES_FILE} must be a JSON list")}),
    };

    let mut written = Vec::new();
    let mut rejected = Vec::new();
    for c in arr {
        let reasons = evaluate(c);
        if !reasons.is_empty() {
            rejected.push(json!({"title": c.get("title").and_then(|v| v.as_str()).unwrap_or("<no title>"), "reasons": reasons}));
            continue;
        }
        let entry = candidate_to_entry(c, &now);
        let errs = iter_errors(&entry);
        if !errs.is_empty() {
            rejected.push(json!({"title": entry.title, "reasons": [format!("schema: {}", errs.join("; "))]}));
            continue;
        }
        if save_entry(&entry_path(kb_root, &entry), &entry).is_ok() {
            written.push(entry.id.clone());
        }
    }
    json!({"written": written, "rejected": rejected})
}
