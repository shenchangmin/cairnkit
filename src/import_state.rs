//! Cold-start import progress (M8) — resumable `/flow-import` pipeline state.

use crate::errors::{usage, Result};
use chrono::Local;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const IMPORT_STEPS: &[&str] = &["doc-collect", "codebase-profile", "knowledge-build", "done"];

fn import_path(root: &Path) -> PathBuf {
    root.join("docs")
        .join("knowledge-import")
        .join("import-state.json")
}
fn now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn save(path: &Path, step: &str, done: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let v = json!({"step": step, "done": done, "updated_at": now()});
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap())?;
    Ok(())
}

pub fn init_import(root: &Path) -> Result<Value> {
    let path = import_path(root);
    if path.exists() {
        return Err(usage(
            "an import is already in progress; use `import show`/`import advance`.",
        ));
    }
    save(&path, IMPORT_STEPS[0], &[])?;
    load_import(root)
}

pub fn load_import(root: &Path) -> Result<Value> {
    let path = import_path(root);
    if !path.exists() {
        return Err(usage(
            "no import in progress; start one with `import init`.",
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text).map_err(|e| usage(format!("import-state.json corrupt: {e}")))
}

pub fn advance_import(root: &Path) -> Result<Value> {
    let cur = load_import(root)?;
    let step = cur.get("step").and_then(|v| v.as_str()).unwrap_or("done");
    if step == "done" {
        return Err(usage("import already complete."));
    }
    let idx = IMPORT_STEPS
        .iter()
        .position(|s| *s == step)
        .unwrap_or(IMPORT_STEPS.len() - 1);
    let mut done: Vec<String> = cur
        .get("done")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    done.push(step.to_string());
    save(&import_path(root), IMPORT_STEPS[idx + 1], &done)?;
    load_import(root)
}
