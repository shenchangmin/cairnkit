//! Data models + file persistence (mirrors Python `cairnkit.config`).
//!
//! Owns the Config / State models and YAML read/write for cairnkit.yaml and STATE.yaml.
//! Files are the single source of truth; writes are atomic (temp + rename) and the STATE
//! field order is the struct declaration order (stable diffs).

use crate::errors::{corrupt, usage, Result};
use crate::stages;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    #[serde(default = "dot")]
    pub path: String,
}
fn dot() -> String {
    ".".to_string()
}

#[derive(Clone, Debug)]
pub struct Config {
    pub project: String,
    pub domain: Option<String>,
    pub repos: Vec<Repo>,
    pub root: PathBuf,
    pub knowledge_root: PathBuf,
    #[allow(dead_code)] // parsed for completeness; cloning uses .local
    pub knowledge_repo_url: Option<String>,
    pub knowledge_repo_local: Option<PathBuf>,
    pub notify_webhook_env: Option<String>,
}

impl Config {
    pub fn state_path(&self) -> PathBuf {
        self.root.join(".cairnkit").join("STATE.yaml")
    }
    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.root.join("docs").join("workflows").join(run_id)
    }
}

// Raw deserialization shapes for cairnkit.yaml.
#[derive(Deserialize)]
struct RawRepoCfg {
    url: Option<String>,
    local: Option<String>,
}
#[derive(Deserialize)]
struct RawNotify {
    feishu_webhook_env: Option<String>,
}
#[derive(Deserialize)]
struct RawConfig {
    project: String,
    domain: Option<String>,
    #[serde(default)]
    repos: Vec<Repo>,
    knowledge_root: Option<String>,
    knowledge_repo: Option<RawRepoCfg>,
    notify: Option<RawNotify>,
}

pub fn load_config(root: &Path) -> Result<Config> {
    let path = root.join("cairnkit.yaml");
    if !path.exists() {
        return Err(usage(format!(
            "cairnkit.yaml not found in {}. Run /team-init to initialise the project.",
            root.display()
        )));
    }
    let text = std::fs::read_to_string(&path)?;
    let raw: RawConfig =
        serde_yaml::from_str(&text).map_err(|e| usage(format!("cairnkit.yaml is invalid: {e}")))?;

    let mut repos = raw.repos;
    if repos.is_empty() {
        repos.push(Repo {
            name: raw.project.clone(),
            path: ".".to_string(),
        });
    }
    let knowledge_root = match raw.knowledge_root {
        Some(kr) => root.join(kr),
        None => root.join("docs").join("knowledge"),
    };
    let (url, local) = match raw.knowledge_repo {
        Some(r) => (r.url, r.local.map(expand_user)),
        None => (None, None),
    };
    Ok(Config {
        project: raw.project,
        domain: raw.domain,
        repos,
        root: root.to_path_buf(),
        knowledge_root,
        knowledge_repo_url: url,
        knowledge_repo_local: local,
        notify_webhook_env: raw.notify.and_then(|n| n.feishu_webhook_env),
    })
}

fn expand_user(p: String) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(p)
}

/// STATE.yaml — field declaration order IS the on-disk order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub run_id: String,
    pub stage: String,
    pub path_mode: String,
    pub history: Vec<String>,
    pub artifacts: BTreeMap<String, String>,
    pub retries: BTreeMap<String, u32>,
    pub pending_clarify: Option<String>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub updated_at: String,
}

fn now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn init_state(config: &Config, run_id: &str) -> Result<State> {
    let state = State {
        run_id: run_id.to_string(),
        stage: "INIT".to_string(),
        path_mode: "full".to_string(),
        history: Vec::new(),
        artifacts: BTreeMap::new(),
        retries: BTreeMap::new(),
        pending_clarify: None,
        blocked_reason: None,
        updated_at: now(),
    };
    save_state(&config.state_path(), &state)?;
    Ok(state)
}

pub fn load_state(state_path: &Path) -> Result<State> {
    if !state_path.exists() {
        return Err(corrupt(format!(
            "STATE.yaml not found at {}. Start a run with /flow-run.",
            state_path.display()
        )));
    }
    let text = std::fs::read_to_string(state_path)
        .map_err(|e| corrupt(format!("STATE.yaml unreadable: {e}")))?;
    let state: State = serde_yaml::from_str(&text).map_err(|e| {
        corrupt(format!(
            "STATE.yaml is corrupt: {e}. Expected fields: run_id, stage, path_mode, history, \
             artifacts, retries, pending_clarify, blocked_reason, updated_at."
        ))
    })?;
    if !stages::is_valid_stage(&state.stage) {
        return Err(corrupt(format!(
            "STATE.yaml has an unknown stage {:?}.",
            state.stage
        )));
    }
    if !stages::is_valid_path_mode(&state.path_mode) {
        return Err(corrupt(format!(
            "STATE.yaml has an unknown path_mode {:?}.",
            state.path_mode
        )));
    }
    Ok(state)
}

pub fn save_state(state_path: &Path, state: &State) -> Result<()> {
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut to_write = state.clone();
    to_write.updated_at = now(); // Python/Rust owns the timestamp
    let yaml = serde_yaml::to_string(&to_write)
        .map_err(|e| usage(format!("failed to serialize STATE: {e}")))?;
    let tmp = state_path.with_extension("yaml.tmp");
    std::fs::write(&tmp, yaml)?;
    std::fs::rename(&tmp, state_path)?;
    Ok(())
}

impl State {
    /// Functional update helper (immutability-friendly).
    pub fn with(mut self, f: impl FnOnce(&mut State)) -> State {
        f(&mut self);
        self
    }
}
