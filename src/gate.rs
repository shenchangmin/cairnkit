//! Stage admission gate (mirrors Python `cairnkit.gate`).
//!
//! `check(next_stage, state, config)` answers: may the run move current -> next_stage?
//! Path-mode agnostic: allowed only when not blocked, any CLARIFY pause is approved, and the
//! current stage's produced artifact exists and is non-empty.

use crate::config::{Config, State};
use crate::stages;

pub struct GateResult {
    pub ok: bool,
    pub stage: String,
    pub missing: Vec<String>,
    pub message: String,
}

pub fn check(next_stage: &str, state: &State, config: &Config) -> GateResult {
    let current = &state.stage;
    let ok = |msg: &str| GateResult {
        ok: true,
        stage: next_stage.to_string(),
        missing: vec![],
        message: msg.to_string(),
    };
    let fail = |msg: String, missing: Vec<String>| GateResult {
        ok: false,
        stage: next_stage.to_string(),
        missing,
        message: msg,
    };

    if let Some(reason) = &state.blocked_reason {
        return fail(format!("run is blocked: {reason}"), vec![]);
    }

    if current == "INIT" && next_stage == "INTENT_GATE" {
        if config.root.join("cairnkit.yaml").exists() {
            return ok("ok");
        }
        return fail(
            "cairnkit.yaml missing — run /team-init first.".to_string(),
            vec!["cairnkit.yaml".to_string()],
        );
    }

    if stages::is_clarify(current) && state.pending_clarify.is_some() {
        return fail(
            format!(
                "CLARIFY not yet approved (pending: {}). Approve with `cairn state approve-clarify`.",
                state.pending_clarify.as_deref().unwrap_or("")
            ),
            vec![],
        );
    }

    if let Some(produced) = stages::stage_artifact(current) {
        let path = config.run_dir(&state.run_id).join(produced);
        let empty = std::fs::metadata(&path)
            .map(|m| m.len() == 0)
            .unwrap_or(true);
        if empty {
            let rel = path
                .strip_prefix(&config.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            return fail(
                format!("Cannot leave {current}: its artifact is missing/empty: {rel}"),
                vec![rel],
            );
        }
    }

    ok("ok")
}
