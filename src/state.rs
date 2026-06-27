//! The state machine (mirrors Python `cairnkit.state`).

use crate::config::{init_state, load_state, save_state, Config, State};
use crate::errors::{usage, CairnError, Result};
use crate::{gate, stages};
use chrono::Local;
use std::path::{Path, PathBuf};

pub fn advance(state_path: &Path, config: &Config) -> Result<State> {
    let state = load_state(state_path)?;
    let current = state.stage.clone();
    let nxt = match stages::next_stage(&current, &state.path_mode) {
        Some(s) => s,
        None => {
            return Err(usage(format!(
                "{current} is the terminal stage; nothing to advance to."
            )))
        }
    };

    let result = gate::check(nxt, &state, config);
    if !result.ok {
        return Err(CairnError::Gate {
            message: result.message,
            missing: result.missing,
        });
    }

    let mut new = state.clone();
    new.history.push(current.clone());
    if let Some(produced) = stages::stage_artifact(&current) {
        let rel = config
            .run_dir(&new.run_id)
            .join(produced)
            .strip_prefix(&config.root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        new.artifacts.insert(current.clone(), rel);
    }
    // entering a CLARIFY stage pauses for async approval
    new.pending_clarify = if stages::is_clarify(nxt) {
        let after = stages::next_stage(nxt, &new.path_mode).unwrap_or("DONE");
        Some(format!("Awaiting approval before {after}"))
    } else {
        None
    };
    new.stage = nxt.to_string();

    save_state(state_path, &new)?;
    Ok(new)
}

pub fn record_failure(state_path: &Path, stage: &str) -> Result<State> {
    if !stages::RETRY_STAGES.contains(&stage) {
        return Err(usage(format!(
            "{stage} is not a retryable verify stage {:?}.",
            stages::RETRY_STAGES
        )));
    }
    let state = load_state(state_path)?;
    let mut new = state.clone();
    let n = new.retries.entry(stage.to_string()).or_insert(0);
    *n += 1;
    let count = *n;
    new.blocked_reason = if count >= stages::RETRY_CAP {
        Some(format!(
            "{stage} failed {count} times (cap {})",
            stages::RETRY_CAP
        ))
    } else {
        None
    };
    save_state(state_path, &new)?;
    Ok(new)
}

pub fn set_path_mode(state_path: &Path, path_mode: &str) -> Result<State> {
    if !stages::is_valid_path_mode(path_mode) {
        return Err(usage(format!(
            "Unknown path mode {path_mode:?}. Valid: {:?}.",
            stages::PATH_MODES
        )));
    }
    let state = load_state(state_path)?;
    let new = state.with(|s| s.path_mode = path_mode.to_string());
    save_state(state_path, &new)?;
    Ok(new)
}

pub fn approve_clarify(state_path: &Path) -> Result<State> {
    let state = load_state(state_path)?;
    let new = state.with(|s| s.pending_clarify = None);
    save_state(state_path, &new)?;
    Ok(new)
}

pub fn set_stage(state_path: &Path, stage: &str) -> Result<State> {
    if !stages::is_valid_stage(stage) {
        return Err(usage(format!(
            "Unknown stage {stage:?}. Valid stages: {}.",
            stages::FULL_SEQUENCE.join(", ")
        )));
    }
    let state = load_state(state_path)?;
    let prev = state.stage.clone();
    let new = state.with(|s| {
        s.history.push(prev);
        s.stage = stage.to_string();
    });
    save_state(state_path, &new)?;
    Ok(new)
}

pub fn unblock(state_path: &Path) -> Result<State> {
    let state = load_state(state_path)?;
    let new = state.with(|s| {
        s.blocked_reason = None;
        s.retries.clear();
    });
    save_state(state_path, &new)?;
    Ok(new)
}

pub fn resume(state_path: &Path) -> Result<State> {
    load_state(state_path)
}

pub fn show(state_path: &Path) -> Result<State> {
    load_state(state_path)
}

pub fn is_paused(state: &State) -> bool {
    state.pending_clarify.is_some()
}

/// Archive an existing STATE non-destructively, then start a fresh run.
///
/// Returns `(fresh_state, Some(archive_path))` when an old STATE was archived,
/// or `(fresh_state, None)` when there was none. Refuses an in-progress run
/// (stage != "DONE") unless `force`. Never clobbers an archive target.
/// A corrupt STATE is archived (as `STATE.unknown.<TS>.yaml`) and reset without
/// `--force`, since its stage can't be read to apply the in-progress guard.
pub fn new_run(
    state_path: &Path,
    config: &Config,
    run_id: &str,
    force: bool,
) -> Result<(State, Option<PathBuf>)> {
    if !state_path.exists() {
        return Ok((init_state(config, run_id)?, None));
    }
    match load_state(state_path) {
        Ok(old) if old.stage == "DONE" => {
            // Ordering is deliberate: archive (rename STATE.yaml away) *before*
            // init writes a fresh one. If init then fails, STATE.yaml is briefly
            // absent, but the archive is an intact, recoverable copy — we never
            // destroy the only copy. The archive is the recovery point. This same
            // archive-then-init ordering applies to the force and corrupt branches.
            let archive = archive_state(state_path, &old.run_id)?;
            Ok((init_state(config, run_id)?, Some(archive)))
        }
        Ok(old) => {
            if !force {
                return Err(usage(format!(
                    "run {:?} is in progress at {}; pass --force to reset.",
                    old.run_id, old.stage
                )));
            }
            let archive = archive_state(state_path, &old.run_id)?;
            Ok((init_state(config, run_id)?, Some(archive)))
        }
        Err(CairnError::StateCorrupt(_)) => {
            // Do not block a fresh start on a corrupt file (product instruction).
            let archive = archive_state(state_path, "unknown")?;
            Ok((init_state(config, run_id)?, Some(archive)))
        }
        Err(e) => Err(e),
    }
}

/// Rename an existing STATE.yaml to a colon-free, timestamped sibling. Never overwrites.
fn archive_state(state_path: &Path, old_run_id: &str) -> Result<PathBuf> {
    // Note: `old_run_id` comes from the run's own STATE; a `/` in it would steer
    // the archive into a sibling dir. Run ids are engine-minted and slash-free in
    // practice, so no sanitization is added here.
    let ts = Local::now().format("%Y%m%dT%H%M%S").to_string();
    let target = state_path.with_file_name(format!("STATE.{old_run_id}.{ts}.yaml"));
    if target.exists() {
        return Err(usage(format!(
            "archive target {} already exists; not clobbering.",
            target.display()
        )));
    }
    std::fs::rename(state_path, &target)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Repo;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn test_config(root: &Path) -> Config {
        Config {
            project: "demo".to_string(),
            domain: None,
            repos: vec![Repo {
                name: "demo".to_string(),
                path: ".".to_string(),
            }],
            root: root.to_path_buf(),
            knowledge_root: root.join("kb"),
            knowledge_repo_url: None,
            knowledge_repo_local: None,
            notify_webhook_env: None,
        }
    }

    /// Seed a STATE.yaml at a given stage so new_run has something to archive.
    fn seed_state(config: &Config, run_id: &str, stage: &str) {
        let s = init_state_at(config, run_id, stage);
        save_state(&config.state_path(), &s).unwrap();
    }

    fn init_state_at(config: &Config, run_id: &str, stage: &str) -> State {
        crate::config::init_state(config, run_id)
            .unwrap()
            .with(|s| s.stage = stage.to_string())
    }

    fn archive_siblings(config: &Config) -> Vec<PathBuf> {
        let dir = config.state_path().parent().unwrap().to_path_buf();
        std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("STATE.") && n != "STATE.yaml")
                    .unwrap_or(false)
            })
            .collect()
    }

    #[test]
    fn new_run_fresh_when_no_state() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        let (s, archived) = new_run(&c.state_path(), &c, "r1", false).unwrap();
        assert!(archived.is_none());
        assert_eq!(s.stage, "INIT");
        assert_eq!(s.run_id, "r1");
        assert!(c.state_path().exists());
    }

    #[test]
    fn new_run_archives_on_done() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        seed_state(&c, "old", "DONE");
        let old_text = std::fs::read_to_string(c.state_path()).unwrap();

        let (s, archived) = new_run(&c.state_path(), &c, "r2", false).unwrap();

        let archive = archived.expect("an existing STATE must be archived");
        assert!(archive.exists());
        assert_eq!(std::fs::read_to_string(&archive).unwrap(), old_text);
        assert_eq!(s.stage, "INIT");
        assert_eq!(s.run_id, "r2");
        // fresh STATE.yaml exists and reads back as the new run.
        let reloaded = load_state(&c.state_path()).unwrap();
        assert_eq!(reloaded.run_id, "r2");
    }

    #[test]
    fn new_run_refuses_in_progress_without_force() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        seed_state(&c, "old", "IMPLEMENT");
        let before = std::fs::read_to_string(c.state_path()).unwrap();

        let err = new_run(&c.state_path(), &c, "r2", false).unwrap_err();
        assert_eq!(err.code(), 2);
        // STATE untouched, no archive written.
        assert_eq!(std::fs::read_to_string(c.state_path()).unwrap(), before);
        assert!(archive_siblings(&c).is_empty());
    }

    #[test]
    fn new_run_force_resets_in_progress() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        seed_state(&c, "old", "IMPLEMENT");

        let (s, archived) = new_run(&c.state_path(), &c, "r2", true).unwrap();
        assert!(archived.unwrap().exists());
        assert_eq!(s.stage, "INIT");
        assert_eq!(s.run_id, "r2");
    }

    #[test]
    fn new_run_archive_collision_errors() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        seed_state(&c, "old", "DONE");
        let before = std::fs::read_to_string(c.state_path()).unwrap();

        // Pre-create every possible archive target for this second so rename collides.
        let ts = Local::now().format("%Y%m%dT%H%M%S").to_string();
        let target = c
            .state_path()
            .with_file_name(format!("STATE.old.{ts}.yaml"));
        std::fs::write(&target, "occupied").unwrap();

        let err = new_run(&c.state_path(), &c, "r2", false).unwrap_err();
        assert_eq!(err.code(), 2);
        // Original STATE not renamed/lost; pre-existing archive not clobbered.
        assert_eq!(std::fs::read_to_string(c.state_path()).unwrap(), before);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "occupied");
    }

    #[test]
    fn new_run_corrupt_state_archives_and_proceeds() {
        let d = tempdir().unwrap();
        let c = test_config(d.path());
        std::fs::create_dir_all(c.state_path().parent().unwrap()).unwrap();
        std::fs::write(c.state_path(), "this is not valid state yaml: [").unwrap();

        let (s, archived) = new_run(&c.state_path(), &c, "r2", false).unwrap();
        let archive = archived.expect("corrupt STATE must be archived, not blocked");
        assert!(archive.exists());
        assert!(archive
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("STATE.unknown."));
        assert_eq!(s.stage, "INIT");
        assert_eq!(s.run_id, "r2");
    }
}
