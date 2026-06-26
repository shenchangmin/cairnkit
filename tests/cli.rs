//! Integration tests — drive the real `cairn` binary (mirrors the Python subprocess contract tests).

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_cairn")
}

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(root: &Path, args: &[&str]) -> Out {
    let o = Command::new(bin())
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    Out {
        code: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).to_string(),
        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
    }
}

fn project() -> TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(
        d.path().join("cairnkit.yaml"),
        "project: demo\ndomain: ads\nknowledge_root: kb\nrepos:\n  - name: demo\n    path: .\n",
    )
    .unwrap();
    d
}

fn init(root: &Path) {
    assert_eq!(run(root, &["state", "init", "--run-id", "r1"]).code, 0);
}

fn write_artifact(root: &Path, name: &str) {
    let dir = root.join("docs/workflows/r1");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), "x").unwrap();
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s.trim()).unwrap()
}

// ---- state machine ----

#[test]
fn config_show_then_init() {
    let d = project();
    let r = run(d.path(), &["config", "show"]);
    assert_eq!(r.code, 0);
    assert_eq!(json(&r.stdout)["has_run"], false);
    init(d.path());
    assert_eq!(
        json(&run(d.path(), &["config", "show"]).stdout)["has_run"],
        true
    );
}

#[test]
fn first_advance_is_intent_gate() {
    let d = project();
    init(d.path());
    let r = run(d.path(), &["state", "advance"]);
    assert_eq!(json(&r.stdout)["stage"], "INTENT_GATE");
}

#[test]
fn gate_refuses_missing_artifact_code_3() {
    let d = project();
    init(d.path());
    run(d.path(), &["state", "set-stage", "ANALYSE_PRODUCT"]);
    let r = run(d.path(), &["state", "advance"]);
    assert_eq!(r.code, 3);
    assert!(r.stderr.contains("missing"));
}

#[test]
fn clarify_pause_and_approval() {
    let d = project();
    init(d.path());
    run(d.path(), &["state", "set-stage", "ANALYSE_PRODUCT"]);
    write_artifact(d.path(), "01-product.md");
    run(d.path(), &["state", "advance"]); // -> CLARIFY_PRODUCT
    assert_eq!(
        json(&run(d.path(), &["state", "resume"]).stdout)["paused"],
        true
    );
    assert_eq!(run(d.path(), &["state", "advance"]).code, 3); // not approved
    run(d.path(), &["state", "approve-clarify"]);
    assert_eq!(
        json(&run(d.path(), &["state", "advance"]).stdout)["stage"],
        "ANALYSE_TECH"
    );
}

#[test]
fn advance_past_terminal_code_2() {
    let d = project();
    init(d.path());
    run(d.path(), &["state", "set-stage", "DONE"]);
    assert_eq!(run(d.path(), &["state", "advance"]).code, 2);
}

#[test]
fn corrupt_state_code_4() {
    let d = project();
    init(d.path());
    std::fs::write(d.path().join(".cairnkit/STATE.yaml"), "stage: INIT\n").unwrap();
    assert_eq!(run(d.path(), &["state", "show"]).code, 4);
}

#[test]
fn retry_then_block_then_unblock() {
    let d = project();
    init(d.path());
    for _ in 0..5 {
        run(d.path(), &["state", "fail", "--stage", "BUILD_VERIFY"]);
    }
    let s = run(d.path(), &["state", "show"]);
    assert!(json(&s.stdout)["blocked_reason"].is_string());
    assert_eq!(
        run(d.path(), &["state", "fail", "--stage", "IMPLEMENT"]).code,
        2
    );
    assert_eq!(run(d.path(), &["state", "unblock"]).code, 0);
}

#[test]
fn set_path_mode_validation() {
    let d = project();
    init(d.path());
    assert_eq!(run(d.path(), &["state", "set-path-mode", "lite"]).code, 0);
    assert_eq!(run(d.path(), &["state", "set-path-mode", "turbo"]).code, 2);
}

#[test]
fn intent_classify() {
    let d = project();
    assert_eq!(
        json(&run(d.path(), &["intent", "classify", "--text", "fix a typo"]).stdout)["path_mode"],
        "single"
    );
}

// ---- knowledge layer ----

fn kb_entry(root: &Path) -> PathBuf {
    let p = root.join("kb/tech-wiki/TK-001.md");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "---\nid: TK-001\ntitle: Pagination\ncategory: tech\ndomain: null\ntype: decision\nguideline_polarity: null\nmaturity: draft\nknowledge_class: causal\nlayer: L1\ntags: [mysql]\napplicable_phases: [ARCHITECT_BACKEND]\nevidence: {contributors: [you], sources: [], projects: [], last_referenced: null, ref_count: 0}\nhistory: []\n---\nKeyset beats OFFSET under deep paging because it avoids scanning and discarding rows here.\n").unwrap();
    p
}

#[test]
fn kb_validate_build_query() {
    let d = project();
    let p = kb_entry(d.path());
    assert_eq!(
        run(d.path(), &["kb", "validate", p.to_str().unwrap()]).code,
        0
    );
    assert_eq!(
        json(&run(d.path(), &["kb", "build-index"]).stdout)["total"],
        1
    );
    let q = run(
        d.path(),
        &[
            "kb",
            "query",
            "--stage",
            "ARCHITECT_BACKEND",
            "--budget",
            "2",
        ],
    );
    let v = json(&q.stdout);
    assert_eq!(v["injected_ids"][0], "TK-001");
    assert_eq!(v["over_budget"], true); // never-silent budget
}

#[test]
fn kb_validate_rejects_bad_entry() {
    let d = project();
    let p = d.path().join("kb/tech-wiki/TK-9.md");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    // tech entry with a domain -> schema violation
    std::fs::write(&p, "---\nid: TK-9\ntitle: t\ncategory: tech\ndomain: oops\ntype: decision\nmaturity: draft\nlayer: L1\napplicable_phases: [IMPLEMENT]\n---\nbody here long enough.\n").unwrap();
    assert_eq!(
        run(d.path(), &["kb", "validate", p.to_str().unwrap()]).code,
        2
    );
}

#[test]
fn reference_loop_drives_promotion() {
    let d = project();
    kb_entry(d.path());
    let run_dir = d.path().join("docs/workflows/r1");
    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(
        run_dir.join("03-arch.md"),
        r#"{"knowledgeReferences":[{"id":"TK-001"}]}"#,
    )
    .unwrap();
    assert_eq!(
        run(
            d.path(),
            &["kb", "touch", "--from", run_dir.to_str().unwrap()]
        )
        .code,
        0
    );
    assert!(
        json(&run(d.path(), &["lifecycle", "promote"]).stdout)["promoted"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("TK-001"))
    );
}

// ---- evolve safety ----

#[test]
fn evolve_lifecycle_and_never_touches_harness() {
    let d = project();
    std::fs::create_dir_all(d.path().join("agents")).unwrap();
    std::fs::write(d.path().join("agents/dev.md"), "ORIGINAL").unwrap();
    assert_eq!(
        run(
            d.path(),
            &["evolve", "propose", "--id", "fix-1", "--content", "x"]
        )
        .code,
        0
    );
    assert!(
        json(&run(d.path(), &["evolve", "list", "--state", "pending"]).stdout)["proposals"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("fix-1"))
    );
    assert_eq!(run(d.path(), &["evolve", "apply", "--id", "fix-1"]).code, 0);
    // the harness file is untouched
    assert_eq!(
        std::fs::read_to_string(d.path().join("agents/dev.md")).unwrap(),
        "ORIGINAL"
    );
    assert_eq!(run(d.path(), &["evolve", "apply", "--id", "nope"]).code, 2);
}

#[test]
fn evolve_rejects_traversal_id() {
    let d = project();
    assert_eq!(
        run(
            d.path(),
            &["evolve", "propose", "--id", "../../evil", "--content", "x"]
        )
        .code,
        2
    );
}
