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
    assert_eq!(
        run(d.path(), &["state", "set-path-mode", "tooling"]).code,
        0
    );
    assert_eq!(run(d.path(), &["state", "set-path-mode", "turbo"]).code, 2);
    // typo close to the new mode is still rejected with code 2.
    assert_eq!(run(d.path(), &["state", "set-path-mode", "toling"]).code, 2);
}

/// A `tooling` run walks INIT -> DONE without E2E_VERIFY ever appearing or
/// `08-e2e.md` ever being demanded by the gate.
#[test]
fn tooling_walk_skips_e2e() {
    let d = project();
    let root = d.path();
    init(root);

    // tooling artifacts in advance-order; note 07-visual.md / 08-e2e.md are absent.
    let artifact_for = |stage: &str| -> Option<&'static str> {
        match stage {
            "ANALYSE_PRODUCT" => Some("01-product.md"),
            "ANALYSE_TECH" => Some("02-tech.md"),
            "ARCHITECT_BACKEND" => Some("03-arch.md"),
            "IMPLEMENT" => Some("05-implement.md"),
            "BUILD_VERIFY" => Some("06-build.md"),
            "TEST" => Some("09-test.md"),
            "ARCHIVE" => Some("10-archive.md"),
            _ => None,
        }
    };

    let stage_now = |root: &Path| -> String {
        json(&run(root, &["state", "show"]).stdout)["stage"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let mut guard = 0;
    loop {
        let s = stage_now(root);
        if s == "DONE" {
            break;
        }
        assert_ne!(s, "E2E_VERIFY", "tooling must never enter E2E_VERIFY");
        if s == "INTENT_GATE" {
            assert_eq!(run(root, &["state", "set-path-mode", "tooling"]).code, 0);
            assert_eq!(
                json(&run(root, &["state", "show"]).stdout)["path_mode"],
                "tooling"
            );
        }
        if let Some(f) = artifact_for(&s) {
            write_artifact(root, f);
        }
        // clear any CLARIFY pause
        if json(&run(root, &["state", "resume"]).stdout)["paused"] == true {
            run(root, &["state", "approve-clarify"]);
        }
        let adv = run(root, &["state", "advance"]);
        assert_eq!(
            adv.code, 0,
            "advance from {s} failed (code {}): {}",
            adv.code, adv.stderr
        );
        guard += 1;
        assert!(guard < 30, "tooling walk did not terminate");
    }

    let final_state = json(&run(root, &["state", "show"]).stdout);
    assert_eq!(final_state["stage"], "DONE");
    let history: Vec<String> = final_state["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    assert!(!history.contains(&"E2E_VERIFY".to_string()));
    assert!(!history.contains(&"VISUAL_REVIEW".to_string()));
    assert!(history.contains(&"BUILD_VERIFY".to_string()));
    assert!(history.contains(&"TEST".to_string()));
}

#[test]
fn state_new_archives_after_done() {
    let d = project();
    let root = d.path();
    init(root);
    run(root, &["state", "set-stage", "DONE"]);

    let r = run(root, &["state", "new", "--run-id", "r2"]);
    assert_eq!(r.code, 0);
    let v = json(&r.stdout);
    assert_eq!(v["stage"], "INIT");
    assert_eq!(v["run_id"], "r2");
    // archive notice on stderr; a timestamped sibling exists.
    assert!(r.stderr.contains("archived previous STATE"));
    let siblings: Vec<_> = std::fs::read_dir(root.join(".cairnkit"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("STATE.") && n != "STATE.yaml")
        .collect();
    assert_eq!(siblings.len(), 1, "exactly one archive sibling expected");
}

#[test]
fn state_new_refuses_in_progress_without_force() {
    let d = project();
    let root = d.path();
    init(root);
    run(root, &["state", "set-stage", "ANALYSE_PRODUCT"]);

    // in-progress without --force -> code 2.
    assert_eq!(run(root, &["state", "new", "--run-id", "r2"]).code, 2);
    // STATE still on the old run.
    assert_eq!(json(&run(root, &["state", "show"]).stdout)["run_id"], "r1");

    // with --force -> code 0 and a fresh run.
    let forced = run(root, &["state", "new", "--run-id", "r2", "--force"]);
    assert_eq!(forced.code, 0);
    let v = json(&forced.stdout);
    assert_eq!(v["stage"], "INIT");
    assert_eq!(v["run_id"], "r2");
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

// ---- knowledge loop: end-to-end proof ----
//
// These three tests prove the knowledge loop closes using ONLY the real `cairn`
// binary (`run()` -> subprocess) + `tempfile::TempDir` + `std::fs`. No Claude Code,
// no network, no Git remote, no agent invocation (acceptance checkbox 10, by construction).

/// Write a valid tech seed entry on `applicable_phases: [ARCHITECT_BACKEND]`.
/// `class` ∈ {"point","causal","spatiotemporal"} drives query sort rank
/// (causal > point), so callers control which id surfaces first. Parameterized
/// variant of `kb_entry` (which hardcodes TK-001); `kb_entry` stays untouched.
fn seed_entry(root: &Path, id: &str, class: &str) -> PathBuf {
    let p = root.join(format!("kb/tech-wiki/{id}.md"));
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        format!(
            "---\nid: {id}\ntitle: Pagination\ncategory: tech\ndomain: null\ntype: decision\nguideline_polarity: null\nmaturity: draft\nknowledge_class: {class}\nlayer: L1\ntags: [mysql]\napplicable_phases: [ARCHITECT_BACKEND]\nevidence: {{contributors: [you], sources: [], projects: [], last_referenced: null, ref_count: 0}}\nhistory: []\n---\nKeyset beats OFFSET under deep paging because it avoids scanning and discarding rows here.\n"
        ),
    )
    .unwrap();
    p
}

/// Write `knowledge-candidates.json` (exact filename from
/// `extract_gate.rs::CANDIDATES_FILE`) into `run_dir` as a JSON array.
/// Creates `run_dir` if absent.
fn write_candidates(run_dir: &Path, candidates: serde_json::Value) {
    std::fs::create_dir_all(run_dir).unwrap();
    std::fs::write(
        run_dir.join("knowledge-candidates.json"),
        serde_json::to_string(&candidates).unwrap(),
    )
    .unwrap();
}

/// One gate-passing candidate: id TK-100, type pitfall, knowledge_class causal,
/// applicable_phases [IMPLEMENT] (a phase the first query returns empty for),
/// body >= 80 chars after trim.
fn candidate_pass() -> serde_json::Value {
    serde_json::json!({
        "id": "TK-100",
        "title": "Cache stampede guard",
        "category": "tech",
        "type": "pitfall",
        "knowledge_class": "causal",
        "layer": "L1",
        "applicable_phases": ["IMPLEMENT"],
        "tags": ["cache"],
        "body": "Use a single-flight lock or request coalescing so a cache miss does not let every concurrent caller hit the database at once."
    })
}

/// Two gate-failing candidates covering >= 2 distinct reasons:
///  - title "Too shallow": body too short -> "insufficient depth (body too short)".
///  - title "Bad type": invalid type + empty applicable_phases ->
///    "missing/invalid type" AND "not transferable (no applicable_phases)".
fn candidates_fail() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "TK-101",
            "title": "Too shallow",
            "category": "tech",
            "type": "pitfall",
            "knowledge_class": "causal",
            "layer": "L1",
            "applicable_phases": ["IMPLEMENT"],
            "tags": [],
            "body": "too short"
        }),
        serde_json::json!({
            "id": "TK-102",
            "title": "Bad type",
            "category": "tech",
            "type": "not-a-real-type",
            "knowledge_class": "causal",
            "layer": "L1",
            "applicable_phases": [],
            "tags": [],
            "body": "This body is comfortably longer than the eighty character minimum so only the type and phases fail."
        }),
    ]
}

#[test]
fn knowledge_loop_closes() {
    let d = project();
    let root = d.path();

    // [R1] seed indexed: validate (exit 0) + build-index (total == 1).
    let seed = seed_entry(root, "TK-001", "causal");
    assert_eq!(
        run(root, &["kb", "validate", seed.to_str().unwrap()]).code,
        0
    );
    assert_eq!(json(&run(root, &["kb", "build-index"]).stdout)["total"], 1);

    // [R2 baseline] IMPLEMENT phase returns nothing yet — capture for the R6 delta.
    let first = json(
        &run(
            root,
            &["kb", "query", "--stage", "IMPLEMENT", "--budget", "300"],
        )
        .stdout,
    );
    let first_ids: Vec<String> = first["injected_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    assert!(
        !first_ids.iter().any(|x| x == "TK-100"),
        "TK-100 must not pre-exist for the loop-closure delta"
    );

    // [R2] seed retrievable on its own phase.
    let q_arch = json(
        &run(
            root,
            &[
                "kb",
                "query",
                "--stage",
                "ARCHITECT_BACKEND",
                "--budget",
                "300",
            ],
        )
        .stdout,
    );
    assert!(q_arch["injected_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "TK-001"));

    // [R4] extract: good candidate written, bad candidates rejected with reasons.
    let run_dir = root.join("docs/workflows/r1");
    let mut candidates = vec![candidate_pass()];
    candidates.extend(candidates_fail());
    write_candidates(&run_dir, serde_json::Value::Array(candidates));
    let ex = json(
        &run(
            root,
            &["kb", "extract", "--from", run_dir.to_str().unwrap()],
        )
        .stdout,
    );
    assert!(ex["written"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "TK-100"));

    // [R4 draft] extracted entry lands on disk as a draft.
    let extracted = std::fs::read_to_string(root.join("kb/tech-wiki/TK-100.md")).unwrap();
    assert!(extracted.contains("maturity: draft"));

    // [D4] extract does NOT auto-index — re-build before the loop-closing query.
    assert_eq!(json(&run(root, &["kb", "build-index"]).stdout)["total"], 2);

    // [R6] loop closes: TK-100 now visible on IMPLEMENT, and was NOT in first_ids.
    let second = json(
        &run(
            root,
            &["kb", "query", "--stage", "IMPLEMENT", "--budget", "300"],
        )
        .stdout,
    );
    let second_ids = second["injected_ids"].as_array().unwrap();
    assert!(
        second_ids.iter().any(|x| x == "TK-100"),
        "extracted id must surface on its phase after re-index"
    );
    assert!(
        !first_ids.iter().any(|x| x == "TK-100"),
        "loop closes only if TK-100 was newly visible (delta), not pre-existing"
    );

    // [R5] refs drive promotion: touch advances the seed, then promote.
    std::fs::write(
        run_dir.join("03-arch.md"),
        r#"{"knowledgeReferences":[{"id":"TK-001"}]}"#,
    )
    .unwrap();
    let touch = json(&run(root, &["kb", "touch", "--from", run_dir.to_str().unwrap()]).stdout);
    assert!(touch["updated"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "TK-001"));
    let promote = json(&run(root, &["lifecycle", "promote"]).stdout);
    assert!(promote["promoted"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "TK-001"));
}

#[test]
fn budget_never_truncates_silently() {
    let d = project();
    let root = d.path();

    // Two same-phase entries; causal sorts before point, so TK-002 is the one cut.
    seed_entry(root, "TK-001", "causal");
    seed_entry(root, "TK-002", "point");
    assert_eq!(json(&run(root, &["kb", "build-index"]).stdout)["total"], 2);

    // Under-budget: something was cut -> `dropped` non-empty and names what was dropped.
    let tight = json(
        &run(
            root,
            &[
                "kb",
                "query",
                "--stage",
                "ARCHITECT_BACKEND",
                "--budget",
                "2",
            ],
        )
        .stdout,
    );
    let dropped = tight["dropped"].as_array().unwrap();
    assert!(!dropped.is_empty(), "truncation must never be silent");
    assert!(
        dropped.iter().any(|x| x["id"] == "TK-002"),
        "dropped must name the cut entry"
    );
    assert_eq!(tight["injected_ids"].as_array().unwrap().len(), 1);

    // Sufficient budget: nothing dropped, not over budget, both surface (negative control).
    let wide = json(
        &run(
            root,
            &[
                "kb",
                "query",
                "--stage",
                "ARCHITECT_BACKEND",
                "--budget",
                "100",
            ],
        )
        .stdout,
    );
    assert_eq!(wide["over_budget"], false);
    assert!(wide["dropped"].as_array().unwrap().is_empty());
    assert_eq!(wide["injected_ids"].as_array().unwrap().len(), 2);
}

#[test]
fn over_budget_flags_oversized_top_entry() {
    let d = project();
    let root = d.path();

    // Single entry whose serialized block is many lines, queried with a budget
    // (1 line) guaranteed smaller than that one block.
    seed_entry(root, "TK-001", "causal");
    assert_eq!(json(&run(root, &["kb", "build-index"]).stdout)["total"], 1);

    let res = json(
        &run(
            root,
            &[
                "kb",
                "query",
                "--stage",
                "ARCHITECT_BACKEND",
                "--budget",
                "1",
            ],
        )
        .stdout,
    );

    // The single top-ranked entry alone exceeds budget: flagged, never silently omitted.
    assert_eq!(
        res["over_budget"], true,
        "an oversized sole entry must be flagged over_budget"
    );
    assert!(
        res["injected_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "TK-001"),
        "the oversized entry is still included, not silently dropped"
    );
    assert!(
        res["dropped"].as_array().unwrap().is_empty(),
        "no second entry exists, so nothing is dropped"
    );
}

#[test]
fn extract_gate_rejects_with_reasons() {
    let d = project();
    let root = d.path();
    let run_dir = root.join("docs/workflows/r1");

    let mut candidates = vec![candidate_pass()];
    candidates.extend(candidates_fail());
    write_candidates(&run_dir, serde_json::Value::Array(candidates));

    let ex = json(
        &run(
            root,
            &["kb", "extract", "--from", run_dir.to_str().unwrap()],
        )
        .stdout,
    );

    // Good candidate written.
    assert!(ex["written"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "TK-100"));

    // Bad candidates rejected, keyed by title, with non-empty reasons (substring-matched).
    let rejected = ex["rejected"].as_array().unwrap();
    let shallow = rejected
        .iter()
        .find(|x| x["title"] == "Too shallow")
        .expect("'Too shallow' must be rejected");
    let shallow_reasons = shallow["reasons"].as_array().unwrap();
    assert!(!shallow_reasons.is_empty());
    assert!(shallow_reasons
        .iter()
        .any(|r| r.as_str().unwrap().contains("insufficient depth")));

    let bad_type = rejected
        .iter()
        .find(|x| x["title"] == "Bad type")
        .expect("'Bad type' must be rejected");
    let bad_type_reasons: Vec<&str> = bad_type["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert!(bad_type_reasons
        .iter()
        .any(|r| r.contains("missing/invalid type")));
    assert!(bad_type_reasons
        .iter()
        .any(|r| r.contains("no applicable_phases")));

    // Bad candidates were never written to disk.
    assert!(!root.join("kb/tech-wiki/TK-101.md").exists());
    assert!(!root.join("kb/tech-wiki/TK-102.md").exists());
}

/// AC#2 / Bug 1: `kb touch` against the real moat-hardening artifacts — WITH the JSON
/// workaround file removed — collects TK-DOG-003..006 from the YAML/bullet reference
/// blocks, and never scrapes TK-DOG-007..010 (which live only in prose / `created:` /
/// `wroteBack:` / tables, not in a reference `- id:` line or TK- bullet).
#[test]
fn kb_touch_parses_yaml_reference_blocks() {
    let d = project();
    let root = d.path();

    // Seed only the entries this run actually applied.
    for id in ["TK-DOG-003", "TK-DOG-004", "TK-DOG-005", "TK-DOG-006"] {
        seed_entry(root, id, "causal");
    }

    // Hermetic fixtures: write artifacts carrying the real knowledgeReferences shapes
    // directly. Do NOT read docs/workflows/ — it is git-ignored, so it exists locally but
    // is absent on a fresh CI clone (this is what broke CI). Shape A: a fenced-yaml `- id:`
    // list, with `created:`/`wroteBack:` flow lists as over-match traps that must be ignored.
    // Shape B: bare `- TK-...` bullets under a heading.
    let run_dir = root.join("docs/workflows/moat");
    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(
        run_dir.join("01-product.md"),
        "## knowledgeReferences\n\n```yaml\nknowledgeReferences:\n  - id: TK-DOG-003\n  - id: TK-DOG-005\n  created: [TK-DOG-007, TK-DOG-008]\n  wroteBack: [TK-DOG-009, TK-DOG-010]\n```\n",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("08-e2e.md"),
        "## knowledgeReferences\n\n- TK-DOG-004 applied this run\n- TK-DOG-006 applied this run\n",
    )
    .unwrap();

    let res = json(&run(root, &["kb", "touch", "--from", run_dir.to_str().unwrap()]).stdout);
    let updated: Vec<&str> = res["updated"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for id in ["TK-DOG-003", "TK-DOG-004", "TK-DOG-005", "TK-DOG-006"] {
        assert!(
            updated.contains(&id),
            "{id} must be collected from YAML blocks"
        );
    }

    let referenced: Vec<&str> = res["referenced"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for id in ["TK-DOG-007", "TK-DOG-008", "TK-DOG-009", "TK-DOG-010"] {
        assert!(
            !referenced.contains(&id),
            "{id} must NOT be scraped (it is prose/created/wroteBack, not a reference)"
        );
    }
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
