//! The state machine (mirrors Python `cairnkit.state`).

use crate::config::{load_state, save_state, Config, State};
use crate::errors::{usage, CairnError, Result};
use crate::{gate, stages};
use std::path::Path;

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
