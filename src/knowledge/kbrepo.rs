//! Cross-project knowledge Git repository (M7) — the moat itself. Git via subprocess.

use crate::errors::{usage, Result};
use crate::knowledge::index::iter_entries;
use crate::knowledge::model::save_entry;
use chrono::Local;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOG_FILE: &str = "log.md";
pub const CONFLICTS_DIR: &str = "contributions/conflicts";

const AUTO_MERGE: &[&str] = &["add", "evidence", "promote_verified"];
const NEEDS_REVIEW: &[&str] = &["conflict", "promote_proven", "team_convention"];

pub fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| usage(format!("failed to run git: {e}")))?;
    if !out.status.success() {
        return Err(usage(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}
pub fn has_remote(repo: &Path) -> bool {
    git(repo, &["remote"]).map(|s| !s.is_empty()).unwrap_or(false)
}

pub fn init_repo(repo: &Path) -> Result<()> {
    std::fs::create_dir_all(repo)?;
    if !is_git_repo(repo) {
        git(repo, &["init", "-q"])?;
    }
    for sub in ["tech-wiki", "biz-wiki", "team-conventions", CONFLICTS_DIR] {
        std::fs::create_dir_all(repo.join(sub))?;
    }
    let log = repo.join(LOG_FILE);
    if !log.exists() {
        std::fs::write(&log, "# Knowledge contribution log (append-only)\n")?;
    }
    Ok(())
}

pub fn pull(repo: &Path) -> Value {
    if !has_remote(repo) {
        return json!({"pulled": false, "reason": "no remote configured (local-only mode)"});
    }
    match git(repo, &["pull", "--ff-only"]) {
        Ok(out) => json!({"pulled": true, "detail": out}),
        Err(e) => json!({"pulled": false, "reason": e.message()}),
    }
}

pub fn push(repo: &Path, message: &str) -> Result<Value> {
    git(repo, &["add", "-A"])?;
    let status = git(repo, &["status", "--porcelain"])?;
    if status.is_empty() {
        return Ok(json!({"committed": false, "reason": "nothing to commit"}));
    }
    git(repo, &["commit", "-q", "-m", message])?;
    if !has_remote(repo) {
        return Ok(json!({"committed": true, "pushed": false, "reason": "no remote (committed locally)"}));
    }
    match git(repo, &["push"]) {
        Ok(_) => Ok(json!({"committed": true, "pushed": true})),
        Err(e) => Ok(json!({"committed": true, "pushed": false, "reason": e.message()})),
    }
}

pub fn append_log(repo: &Path, line: &str) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(repo.join(LOG_FILE))?;
    writeln!(f, "{}", line.replace('\n', " ").trim_end())?;
    Ok(())
}

pub fn classify_contribution(kind: &str) -> Result<&'static str> {
    if AUTO_MERGE.contains(&kind) {
        Ok("auto")
    } else if NEEDS_REVIEW.contains(&kind) {
        Ok("review")
    } else {
        Err(usage(format!("unknown contribution kind {kind:?}")))
    }
}

fn safe_id(id: &str) -> Result<&str> {
    let ok = !id.is_empty()
        && !id.contains("..")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(id)
    } else {
        Err(usage(format!("invalid id {id:?} (allowed: letters, digits, . _ -)")))
    }
}

pub fn stage_conflict(repo: &Path, entry_id: &str, body: &str, today: Option<&str>) -> Result<PathBuf> {
    safe_id(entry_id)?;
    let today = today.map(String::from).unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let target = repo.join(CONFLICTS_DIR);
    std::fs::create_dir_all(&target)?;
    let mut path = target.join(format!("{entry_id}-{today}.md"));
    let mut n = 1;
    while path.exists() {
        path = target.join(format!("{entry_id}-{today}-{n}.md"));
        n += 1;
    }
    std::fs::write(&path, body)?;
    append_log(repo, &format!("- {today} CONFLICT staged for {entry_id} -> {}", path.file_name().unwrap().to_string_lossy()))?;
    Ok(path)
}

pub fn promote_entry(repo: &Path, entry_id: &str, target_layer: &str) -> Result<PathBuf> {
    safe_id(entry_id)?;
    if target_layer != "L1" && target_layer != "L2" {
        return Err(usage("promotion target must be L1 or L2"));
    }
    let entries = iter_entries(repo);
    let m = entries.iter().find(|e| e.id == entry_id && e.layer == "L3");
    let entry = match m {
        Some(e) if e.path.is_some() => e,
        _ => return Err(usage(format!("no L3 entry with id {entry_id} found in {}", repo.display()))),
    };
    let src = entry.path.clone().unwrap();
    let fname = src.file_name().unwrap();
    let dest = if target_layer == "L1" {
        repo.join("tech-wiki").join(fname)
    } else {
        repo.join("biz-wiki").join(entry.domain.clone().unwrap_or_else(|| "_".to_string())).join(fname)
    };
    if dest.exists() && dest != src {
        return Err(usage(format!("destination {} already exists — refusing to overwrite", fname.to_string_lossy())));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut promoted = entry.clone();
    promoted.layer = target_layer.to_string();
    save_entry(&dest, &promoted)?;
    if src != dest {
        let _ = std::fs::remove_file(&src);
    }
    append_log(repo, &format!("- {} PROMOTE {entry_id} -> {target_layer}", Local::now().format("%Y-%m-%d")))?;
    Ok(dest)
}

pub fn stats(repo: &Path) -> Value {
    let entries = iter_entries(repo);
    let mut by_mat: BTreeMap<String, i64> = BTreeMap::new();
    let mut by_cat: BTreeMap<String, i64> = BTreeMap::new();
    for e in &entries {
        *by_mat.entry(e.maturity.clone()).or_insert(0) += 1;
        *by_cat.entry(e.category.clone()).or_insert(0) += 1;
    }
    let referenced = entries.iter().filter(|e| e.evidence.ref_count > 0).count();
    let orphans: Vec<String> = entries
        .iter()
        .filter(|e| e.evidence.ref_count == 0 && e.maturity != "draft")
        .map(|e| e.id.clone())
        .collect();
    let rate = if entries.is_empty() { 0.0 } else { (referenced as f64 / entries.len() as f64 * 100.0).round() / 100.0 };
    json!({
        "total": entries.len(),
        "by_maturity": by_mat,
        "by_category": by_cat,
        "referenced": referenced,
        "reference_rate": rate,
        "orphans": orphans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify() {
        assert_eq!(classify_contribution("add").unwrap(), "auto");
        assert_eq!(classify_contribution("promote_verified").unwrap(), "auto");
        assert_eq!(classify_contribution("conflict").unwrap(), "review");
        assert_eq!(classify_contribution("promote_proven").unwrap(), "review");
        assert!(classify_contribution("bogus").is_err());
    }

    #[test]
    fn init_creates_skeleton() {
        let d = tempfile::tempdir().unwrap();
        let repo = d.path().join("kb");
        init_repo(&repo).unwrap();
        assert!(repo.join("tech-wiki").is_dir());
        assert!(repo.join("biz-wiki").is_dir());
        assert!(repo.join(LOG_FILE).exists());
        assert!(is_git_repo(&repo));
    }

    #[test]
    fn safe_id_blocks_traversal() {
        assert!(safe_id("../evil").is_err());
        assert!(safe_id("TK-1").is_ok());
    }
}
