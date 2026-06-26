//! Self-evolution proposal lifecycle (M9).
//!
//! This module manages ONLY the proposal lifecycle (pending -> applied/rejected/deferred) and the
//! audit log. It has NO code path that writes to agents/ or rules/ — the "never auto-apply"
//! guarantee is structural, not a matter of discipline.

use crate::errors::{usage, Result};
use chrono::Local;
use std::path::{Path, PathBuf};

pub const STATES: &[&str] = &["pending", "applied", "rejected", "deferred"];
const LOG: &str = "log.md";

fn root_dir(root: &Path) -> PathBuf {
    root.join("docs").join("workflows").join("evolve-log")
}
pub fn evolve_dir(root: &Path, state: &str) -> PathBuf {
    root_dir(root).join(state)
}

fn log_line(root: &Path, line: &str) -> Result<()> {
    use std::io::Write;
    let log = root_dir(root).join(LOG);
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&log)?;
    writeln!(f, "{} {}", Local::now().format("%Y-%m-%dT%H:%M:%S"), line)?;
    Ok(())
}

fn safe_id(id: &str) -> Result<&str> {
    let ok = !id.is_empty()
        && !id.contains("..")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(id)
    } else {
        Err(usage(format!("invalid proposal id {id:?} (allowed: letters, digits, . _ -)")))
    }
}

pub fn propose(root: &Path, proposal_id: &str, content: &str) -> Result<PathBuf> {
    safe_id(proposal_id)?;
    let pending = evolve_dir(root, "pending");
    std::fs::create_dir_all(&pending)?;
    let path = pending.join(format!("{proposal_id}.md"));
    if path.exists() {
        return Err(usage(format!("proposal {proposal_id} already exists")));
    }
    std::fs::write(&path, content)?;
    log_line(root, &format!("PROPOSE {proposal_id}"))?;
    Ok(path)
}

pub fn list_proposals(root: &Path, state: &str) -> Result<Vec<String>> {
    if !STATES.contains(&state) {
        return Err(usage(format!("unknown state {state:?}; valid: {STATES:?}")));
    }
    let dir = evolve_dir(root, state);
    let mut ids: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension().map(|x| x == "md").unwrap_or(false) {
                p.file_stem().map(|s| s.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    ids.sort();
    Ok(ids)
}

pub fn transition(root: &Path, proposal_id: &str, to_state: &str) -> Result<PathBuf> {
    safe_id(proposal_id)?;
    if !matches!(to_state, "applied" | "rejected" | "deferred") {
        return Err(usage(format!("cannot transition to {to_state:?}")));
    }
    let src = evolve_dir(root, "pending").join(format!("{proposal_id}.md"));
    if !src.exists() {
        return Err(usage(format!("no pending proposal {proposal_id}")));
    }
    let dest_dir = evolve_dir(root, to_state);
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(format!("{proposal_id}.md"));
    std::fs::rename(&src, &dest)?;
    log_line(root, &format!("{} {proposal_id}", to_state.to_uppercase()))?;
    Ok(dest)
}
