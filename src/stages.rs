//! The full 16-stage delivery pipeline metadata (mirrors Python `cairnkit.stages`).

/// Canonical stage order. advance() walks this, skipping stages not in the active path mode.
pub const FULL_SEQUENCE: &[&str] = &[
    "INIT",
    "INTENT_GATE",
    "ANALYSE_PRODUCT",
    "CLARIFY_PRODUCT",
    "ANALYSE_TECH",
    "CLARIFY_TECH",
    "ARCHITECT_BACKEND",
    "CLARIFY_ARCH_BACKEND",
    "ARCHITECT_FRONTEND",
    "CLARIFY_ARCH_FRONTEND",
    "IMPLEMENT",
    "BUILD_VERIFY",
    "VISUAL_REVIEW",
    "E2E_VERIFY",
    "TEST",
    "ARCHIVE",
    "DONE",
];

pub const PATH_MODES: &[&str] = &["full", "lite", "single"];
pub const RETRY_STAGES: &[&str] = &["BUILD_VERIFY", "E2E_VERIFY"];
pub const RETRY_CAP: u32 = 5;

const LITE_EXCLUDE: &[&str] = &["ARCHITECT_FRONTEND", "CLARIFY_ARCH_FRONTEND", "VISUAL_REVIEW"];
const SINGLE_INCLUDE: &[&str] = &[
    "INIT", "INTENT_GATE", "IMPLEMENT", "BUILD_VERIFY", "TEST", "ARCHIVE", "DONE",
];

/// Artifact a stage produces (verified on advance out of that stage). None for others.
pub fn stage_artifact(stage: &str) -> Option<&'static str> {
    match stage {
        "ANALYSE_PRODUCT" => Some("01-product.md"),
        "ANALYSE_TECH" => Some("02-tech.md"),
        "ARCHITECT_BACKEND" => Some("03-arch.md"),
        "ARCHITECT_FRONTEND" => Some("04-arch-fe.md"),
        "IMPLEMENT" => Some("05-implement.md"),
        "BUILD_VERIFY" => Some("06-build.md"),
        "VISUAL_REVIEW" => Some("07-visual.md"),
        "E2E_VERIFY" => Some("08-e2e.md"),
        "TEST" => Some("09-test.md"),
        "ARCHIVE" => Some("10-archive.md"),
        _ => None,
    }
}

pub fn is_clarify(stage: &str) -> bool {
    stage.starts_with("CLARIFY")
}

pub fn is_valid_stage(stage: &str) -> bool {
    FULL_SEQUENCE.contains(&stage)
}

pub fn is_valid_path_mode(mode: &str) -> bool {
    PATH_MODES.contains(&mode)
}

/// The ordered stages active for a path mode. Returns None for an unknown mode (no silent fallback).
pub fn stages_for(path_mode: &str) -> Option<Vec<&'static str>> {
    match path_mode {
        "single" => Some(
            FULL_SEQUENCE
                .iter()
                .copied()
                .filter(|s| SINGLE_INCLUDE.contains(s))
                .collect(),
        ),
        "lite" => Some(
            FULL_SEQUENCE
                .iter()
                .copied()
                .filter(|s| !LITE_EXCLUDE.contains(s))
                .collect(),
        ),
        "full" => Some(FULL_SEQUENCE.to_vec()),
        _ => None,
    }
}

/// The next active stage after `current` for the path mode, or None at the end.
pub fn next_stage(current: &str, path_mode: &str) -> Option<&'static str> {
    let active = stages_for(path_mode)?;
    if let Some(i) = active.iter().position(|s| *s == current) {
        return active.get(i + 1).copied();
    }
    // current was skipped/foreign — fall back to its position in the full sequence
    let idx = FULL_SEQUENCE.iter().position(|s| *s == current)?;
    FULL_SEQUENCE[idx + 1..]
        .iter()
        .find(|s| active.contains(s))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_chain() {
        assert_eq!(next_stage("INIT", "full"), Some("INTENT_GATE"));
        assert_eq!(next_stage("ARCHIVE", "full"), Some("DONE"));
        assert_eq!(next_stage("DONE", "full"), None);
    }

    #[test]
    fn lite_skips_frontend() {
        assert_eq!(next_stage("CLARIFY_ARCH_BACKEND", "lite"), Some("IMPLEMENT"));
    }

    #[test]
    fn foreign_current_falls_back() {
        assert_eq!(next_stage("ARCHITECT_FRONTEND", "lite"), Some("IMPLEMENT"));
        assert_eq!(next_stage("CLARIFY_TECH", "single"), Some("IMPLEMENT"));
    }

    #[test]
    fn single_is_minimal() {
        let s = stages_for("single").unwrap();
        assert!(!s.contains(&"ANALYSE_PRODUCT"));
        assert!(s.contains(&"IMPLEMENT") && s.contains(&"DONE"));
    }

    #[test]
    fn unknown_mode_none() {
        assert!(stages_for("turbo").is_none());
    }
}
