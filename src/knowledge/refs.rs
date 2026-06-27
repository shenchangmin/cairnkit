//! Reference-tracking closed loop (M6) — the engine of maturity.

use crate::knowledge::index::iter_entries;
use crate::knowledge::model::save_entry;
use chrono::Local;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Yield each balanced top-level {...} substring (robust to nested objects).
fn top_level_json_objects(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    for (i, ch) in text.char_indices() {
        if ch == '{' {
            if depth == 0 {
                start = Some(i);
            }
            depth += 1;
        } else if ch == '}' && depth > 0 {
            depth -= 1;
            if depth == 0 {
                if let Some(s) = start.take() {
                    out.push(text[s..i + ch.len_utf8()].to_string());
                }
            }
        }
    }
    out
}

fn ids_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if k == "knowledgeReferences" {
                    if let Some(arr) = val.as_array() {
                        for r in arr {
                            if let Some(id) = r.get("id").and_then(|x| x.as_str()) {
                                out.push(id.to_string());
                            }
                        }
                    }
                } else {
                    ids_in(val, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| ids_in(x, out)),
        _ => {}
    }
}

/// Try to pull a TK-/BK- id out of one in-block line. Returns None for any line
/// that is not an `id:` line or a TK-/BK--leading bullet.
fn block_id_from_line(trimmed: &str) -> Option<String> {
    let (rest, is_bullet) = if let Some(r) = trimmed.strip_prefix("- ") {
        (r.trim_start(), true)
    } else if let Some(r) = trimmed.strip_prefix("* ") {
        (r.trim_start(), true)
    } else {
        (trimmed, false)
    };

    if let Some(v) = rest.strip_prefix("id:") {
        let v = v.trim();
        let v = v
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
            .unwrap_or(v);
        return (v.starts_with("TK-") || v.starts_with("BK-")).then(|| v.to_string());
    }
    if is_bullet {
        let tok = rest.split_whitespace().next().unwrap_or("");
        let tok = tok.trim_matches(|c| c == '"' || c == '\'');
        let tok = tok.trim_end_matches([',', '.']);
        return (tok.starts_with("TK-") || tok.starts_with("BK-")).then(|| tok.to_string());
    }
    None
}

/// Extract knowledgeReferences ids from the agent-emitted YAML/markdown shapes.
///
/// Scoped & structural: ids are collected ONLY from inside a recognized
/// `knowledgeReferences` block, and ONLY from `- id:`/`id:` lines and bare
/// `- TK-…`/`- BK-…` leading-token bullets. `created:`/`wroteBack:`/`injected_ids:`
/// inline-flow seqs and `note:`/`notes:`/`title:`/`usedIn:` prose are deliberately
/// NOT scraped (avoids self-reference + double-count). Hand-rolled line scan,
/// no `regex` crate, panic-free. Companion to the legacy JSON path (`ids_in`).
fn ids_from_reference_blocks(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut ids = Vec::new();
    let mut in_block = false;
    let mut block_saw_id_marker = false;
    let mut block_yielded = 0usize;
    for i in 0..lines.len() {
        let t = lines[i].trim();
        if !in_block {
            let is_key = t == "knowledgeReferences:";
            let is_heading = t.starts_with('#') && t.contains("knowledgeReferences");
            if is_key || is_heading {
                // Shape A "double-trigger" is intentional and harmless: a
                // `## knowledgeReferences` heading opens a block, the blank line
                // before the fenced opener closes it, then the inner
                // `knowledgeReferences:` YAML key re-opens block scope here. The
                // forward scan visits each physical line exactly once, so
                // reopening can never re-collect an id already yielded; genuine
                // cross-block/cross-file duplicates are summed into ref_count by
                // `touch` on purpose. Do not "fix" the re-trigger.
                in_block = true;
                block_saw_id_marker = false;
                block_yielded = 0;
            }
            continue; // never collect on the start line itself
        }
        // BLOCK END conditions:
        if t == "```" {
            warn_if_dropped(block_saw_id_marker, block_yielded);
            in_block = false;
            continue;
        }
        if t.starts_with('#') {
            warn_if_dropped(block_saw_id_marker, block_yielded);
            in_block = false;
            if t.contains("knowledgeReferences") {
                in_block = true; // a back-to-back block
                block_saw_id_marker = false;
                block_yielded = 0;
            }
            continue;
        }
        if t.is_empty() {
            // unfenced terminator: blank line followed by a line that is neither
            // blank, nor a bullet (`- `/`* `), nor an indented continuation.
            let next = lines.get(i + 1).copied().unwrap_or("");
            let nt = next.trim();
            let is_list_or_cont = nt.is_empty()
                || nt.starts_with("- ")
                || nt.starts_with("* ")
                || next.starts_with(' ')
                || next.starts_with('\t');
            if !is_list_or_cont {
                warn_if_dropped(block_saw_id_marker, block_yielded);
                in_block = false;
            }
            continue;
        }
        // IN-BLOCK COLLECT
        let key = t
            .strip_prefix("- ")
            .or_else(|| t.strip_prefix("* "))
            .unwrap_or(t)
            .trim_start();
        if key.starts_with("id:") {
            block_saw_id_marker = true; // visibly id:-bearing content
        }
        if let Some(id) = block_id_from_line(t) {
            ids.push(id);
            block_yielded += 1;
        }
    }
    ids
}

/// Log (never swallow) when a recognized block had id-bearing content but yielded nothing.
///
/// Scope note: the drop diagnostic only fires for `id:`-style blocks. `saw_id_marker`
/// is set exclusively by `id:`-bearing lines (Shape A), so a malformed Shape B
/// bare-bullet block (`- TK-DOG-NNN`) that yields nothing produces no warning — by
/// design. Bare bullets carry no unambiguous "this was meant to be a reference"
/// signal, so warning on them would emit false positives on ordinary prose bullets.
fn warn_if_dropped(saw_id_marker: bool, yielded: usize) {
    if saw_id_marker && yielded == 0 {
        eprintln!(
            "warning: a knowledgeReferences block contained id-like lines but yielded no references"
        );
    }
}

pub fn collect_references(run_dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    if !run_dir.exists() {
        return ids;
    }
    let mut files: Vec<_> = std::fs::read_dir(run_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    files.sort();
    for path in files {
        if let Ok(text) = std::fs::read_to_string(&path) {
            // legacy JSON path (Shape C) — unchanged.
            for blob in top_level_json_objects(&text) {
                if let Ok(v) = serde_json::from_str::<Value>(&blob) {
                    ids_in(&v, &mut ids);
                }
            }
            // scoped structural path for YAML/markdown shapes (A/B/E).
            ids.extend(ids_from_reference_blocks(&text));
        }
    }
    ids
}

pub fn touch(kb_root: &Path, run_dir: &Path, project: &str, today: Option<&str>) -> Value {
    let today = today
        .map(String::from)
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());
    let referenced = collect_references(run_dir);
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for id in &referenced {
        *counts.entry(id.clone()).or_insert(0) += 1;
    }

    let mut updated = Vec::new();
    let entries = iter_entries(kb_root);
    for (id, n) in &counts {
        if let Some(entry) = entries.iter().find(|e| &e.id == id) {
            if let Some(path) = &entry.path {
                let mut new = entry.clone();
                if !new.evidence.projects.iter().any(|p| p == project) {
                    new.evidence.projects.push(project.to_string());
                }
                new.evidence.last_referenced = Some(today.clone());
                new.evidence.ref_count += n;
                if save_entry(path, &new).is_ok() {
                    updated.push(id.clone());
                }
            }
        }
    }
    let unknown: Vec<&String> = counts.keys().filter(|k| !updated.contains(k)).collect();
    json!({"referenced": referenced, "updated": updated, "unknown": unknown})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::{load_entry, Entry, Evidence};
    use tempfile::tempdir;

    /// Seed a draft entry on disk so it has a `path` (touch only updates path-bearing entries).
    fn seed_entry(kb_root: &Path, id: &str) {
        let entry = Entry {
            id: id.to_string(),
            title: format!("title for {id}"),
            category: "tech".to_string(),
            domain: None,
            kind: "decision".to_string(),
            guideline_polarity: None,
            maturity: "draft".to_string(),
            knowledge_class: "point".to_string(),
            layer: "L1".to_string(),
            tags: vec![],
            applicable_phases: vec![],
            evidence: Evidence::default(),
            history: vec![],
            body: "body".to_string(),
            path: None,
        };
        save_entry(&kb_root.join("tech-wiki").join(format!("{id}.md")), &entry).unwrap();
    }

    /// Write a run-dir `.md` whose content embeds a knowledgeReferences block for each id.
    fn write_run_file(run_dir: &Path, name: &str, ids: &[&str]) {
        std::fs::create_dir_all(run_dir).unwrap();
        let refs: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
        let blob = json!({"knowledgeReferences": refs}).to_string();
        std::fs::write(run_dir.join(name), format!("# run\n\n{blob}\n")).unwrap();
    }

    /// Write a run-dir `.md` verbatim (for the YAML/markdown reference shapes).
    fn write_raw(run_dir: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(run_dir).unwrap();
        std::fs::write(run_dir.join(name), body).unwrap();
    }

    // Shape A: fenced yaml `- id:` list with colon-bearing title/usedIn/notes siblings.
    #[test]
    fn collects_ids_from_fenced_yaml_list() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-DOG-1");
        write_raw(
            run.path(),
            "05-implement.md",
            "## knowledgeReferences\n\n```yaml\nknowledgeReferences:\n  - id: TK-DOG-1\n    title: \"a: b\"\n    usedIn: \"x: y\"\nnotes: trailing note with a colon: here\n```\n",
        );

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));
        let updated = res["updated"].as_array().unwrap();
        assert!(updated.iter().any(|v| v == "TK-DOG-1"));
        // no spurious id scraped from the colon-bearing title/usedIn/notes.
        assert_eq!(res["referenced"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn collects_unquoted_and_quoted_ids() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-DOG-1");
        seed_entry(kb.path(), "BK-DOG-2");
        write_raw(
            run.path(),
            "05-implement.md",
            "```yaml\nknowledgeReferences:\n  - id: TK-DOG-1\n  - id: \"BK-DOG-2\"\n```\n",
        );

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));
        let updated = res["updated"].as_array().unwrap();
        assert!(updated.iter().any(|v| v == "TK-DOG-1"));
        assert!(updated.iter().any(|v| v == "BK-DOG-2"));
    }

    // Shape B: `## knowledgeReferences` heading then bare markdown bullets.
    #[test]
    fn collects_bullet_form() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-DOG-1");
        write_raw(
            run.path(),
            "08-e2e.md",
            "## knowledgeReferences\n\n- TK-DOG-1 (note text with a colon: here)\n",
        );

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));
        assert!(res["updated"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "TK-DOG-1"));
    }

    // Shapes D/E teeth: inline-flow seqs and prose must NOT be scraped.
    #[test]
    fn ignores_inline_flow_and_prose() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-DOG-1");
        write_raw(
            run.path(),
            "10-archive.md",
            "## knowledgeReferences\n\
             ```yaml\n\
             knowledgeReferences:\n\
             \x20 stage: ARCHIVE\n\
             \x20 injected_ids: [TK-DOG-9]\n\
             \x20 applied:\n\
             \x20   - id: TK-DOG-1\n\
             \x20     title: \"t\"\n\
             \x20 wroteBack: [TK-DOG-1]\n\
             \x20 created: [TK-DOG-9]\n\
             \x20 note: >\n\
             \x20   prose mentioning TK-DOG-9 inline\n\
             ```\n\
             \n\
             ## notes\n\
             \n\
             - **TK-DOG-9** — a sibling prose bullet under a separate heading\n",
        );

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));
        let referenced: Vec<&str> = res["referenced"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        // applied's `- id:` collected exactly once; TK-DOG-9 never collected.
        assert_eq!(referenced, vec!["TK-DOG-1"]);
        assert!(!referenced.contains(&"TK-DOG-9"));
    }

    // R8a: a known id is marked updated, and its evidence is bumped on disk.
    #[test]
    fn touch_marks_known_id_updated_and_bumps_evidence() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-X");
        write_run_file(run.path(), "05-implement.md", &["TK-X"]);

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));

        let updated = res["updated"].as_array().unwrap();
        assert!(updated.iter().any(|v| v == "TK-X"));
        assert!(res["unknown"].as_array().unwrap().is_empty());

        let reloaded = load_entry(&kb.path().join("tech-wiki").join("TK-X.md")).unwrap();
        assert_eq!(reloaded.evidence.ref_count, 1);
        assert_eq!(
            reloaded.evidence.last_referenced.as_deref(),
            Some("2026-06-28")
        );
        assert!(reloaded.evidence.projects.contains(&"proj".to_string()));
    }

    // R8b: an id referenced but not in the KB is reported unknown, never updated.
    #[test]
    fn touch_reports_unknown_id_not_in_kb() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        write_run_file(run.path(), "05-implement.md", &["TK-NOPE"]);

        let res = touch(kb.path(), run.path(), "proj", Some("2026-06-28"));

        assert!(res["unknown"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "TK-NOPE"));
        assert!(!res["updated"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "TK-NOPE"));
    }

    // R8c: duplicate references across files sum into ref_count.
    #[test]
    fn touch_sums_duplicate_references_into_ref_count() {
        let kb = tempdir().unwrap();
        let run = tempdir().unwrap();
        seed_entry(kb.path(), "TK-X");
        write_run_file(run.path(), "a.md", &["TK-X"]);
        write_run_file(run.path(), "b.md", &["TK-X"]);

        touch(kb.path(), run.path(), "proj", Some("2026-06-28"));

        let reloaded = load_entry(&kb.path().join("tech-wiki").join("TK-X.md")).unwrap();
        assert_eq!(reloaded.evidence.ref_count, 2);
    }
}
