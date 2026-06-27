//! Budget-bounded progressive knowledge injection (M5).

use crate::knowledge::index::iter_entries;
use crate::knowledge::model::{serialize_entry, Entry};
use crate::knowledge::{class_rank, maturity_rank};
use serde_json::{json, Value};
use std::path::Path;

pub struct QueryResult {
    pub stage: String,
    pub budget_lines: i64,
    pub injected_ids: Vec<String>,
    pub dropped: Vec<Value>,
    pub text: String,
    pub lines: i64,
    pub over_budget: bool,
}

fn applies(e: &Entry, stage: &str, domain: Option<&str>) -> bool {
    if !e.applicable_phases.iter().any(|p| p == stage) {
        return false;
    }
    if e.category == "tech" {
        return true; // L1 tech knowledge is globally visible
    }
    domain.is_some() && e.domain.as_deref() == domain
}

pub fn query(kb_root: &Path, stage: &str, budget_lines: i64, domain: Option<&str>) -> QueryResult {
    let mut candidates: Vec<Entry> = iter_entries(kb_root)
        .into_iter()
        .filter(|e| applies(e, stage, domain))
        .collect();
    candidates.sort_by(|a, b| {
        (
            -maturity_rank(&a.maturity),
            -class_rank(&a.knowledge_class),
            &a.id,
        )
            .cmp(&(
                -maturity_rank(&b.maturity),
                -class_rank(&b.knowledge_class),
                &b.id,
            ))
    });

    let mut injected: Vec<String> = Vec::new();
    let mut injected_ids: Vec<String> = Vec::new();
    let mut dropped: Vec<Value> = Vec::new();
    let mut used: i64 = 0;
    let mut over_budget = false;

    for e in &candidates {
        let block = serialize_entry(e);
        let block_lines = block.matches('\n').count() as i64;
        if injected.is_empty() {
            // always include the single top-ranked entry; flag (never silently) if it alone exceeds budget
            injected.push(block);
            injected_ids.push(e.id.clone());
            used += block_lines;
            over_budget = block_lines > budget_lines;
        } else if used + block_lines > budget_lines {
            dropped.push(json!({"id": e.id, "title": e.title, "reason": "budget"}));
        } else {
            injected.push(block);
            injected_ids.push(e.id.clone());
            used += block_lines;
        }
    }

    QueryResult {
        stage: stage.to_string(),
        budget_lines,
        injected_ids,
        dropped,
        text: injected.join("\n"),
        lines: used,
        over_budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::{save_entry, Entry, Evidence};
    use tempfile::tempdir;

    /// Build a schema-valid entry literal and write it to the correct wiki subdir.
    #[allow(clippy::too_many_arguments)]
    fn write_entry(
        root: &Path,
        id: &str,
        category: &str,
        domain: Option<&str>,
        maturity: &str,
        phases: &[&str],
        body: &str,
    ) {
        let entry = Entry {
            id: id.to_string(),
            title: format!("title for {id}"),
            category: category.to_string(),
            domain: domain.map(String::from),
            kind: "decision".to_string(),
            guideline_polarity: None,
            maturity: maturity.to_string(),
            knowledge_class: "point".to_string(),
            layer: "L1".to_string(),
            tags: vec!["t".to_string()],
            applicable_phases: phases.iter().map(|p| p.to_string()).collect(),
            evidence: Evidence::default(),
            history: vec![],
            body: body.to_string(),
            path: None,
        };
        let path = if category == "biz" {
            root.join("biz-wiki")
                .join(domain.unwrap_or("_"))
                .join(format!("{id}.md"))
        } else {
            root.join("tech-wiki").join(format!("{id}.md"))
        };
        save_entry(&path, &entry).unwrap();
    }

    const BIG_BODY: &str = "This body is comfortably over eighty characters long so the \
        serialized entry block is a realistic number of lines for budget sizing in tests.";

    // R1: two tech entries same stage, top fits but second does not -> the second is
    // pushed to `dropped` (the never-silent follow-on cut). Asserts `dropped` ONLY,
    // never `over_budget` — they are distinct signals (TK-DOG-003).
    #[test]
    fn dropped_names_followon_entry_cut_for_budget() {
        let dir = tempdir().unwrap();
        // TK-A is "proven" so it ranks first and is guaranteed the injected top entry.
        write_entry(
            dir.path(),
            "TK-A",
            "tech",
            None,
            "proven",
            &["IMPLEMENT"],
            BIG_BODY,
        );
        write_entry(
            dir.path(),
            "TK-B",
            "tech",
            None,
            "draft",
            &["IMPLEMENT"],
            BIG_BODY,
        );

        let r = query(dir.path(), "IMPLEMENT", 35, None);

        assert!(
            !r.dropped.is_empty(),
            "second entry should be cut to dropped"
        );
        assert_eq!(r.dropped[0]["id"], "TK-B");
        assert_eq!(r.injected_ids, vec!["TK-A".to_string()]);
        // Intentionally NO over_budget assertion here (TK-DOG-003).
    }

    // R2: one tech entry, budget so tight its own block exceeds it -> the entry is still
    // injected but `over_budget` is flagged, with NOTHING in `dropped`. This is the teeth
    // target for the over_budget signal (TK-DOG-006).
    #[test]
    fn over_budget_flags_single_oversized_top_entry() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-A",
            "tech",
            None,
            "draft",
            &["IMPLEMENT"],
            BIG_BODY,
        );

        let r = query(dir.path(), "IMPLEMENT", 1, None);

        assert!(
            r.over_budget,
            "single oversized top entry must flag over_budget"
        );
        assert_eq!(r.injected_ids, vec!["TK-A".to_string()]);
        assert!(
            r.dropped.is_empty(),
            "nothing follows the top entry, so dropped is empty"
        );
    }

    // R3a: tech knowledge is globally visible; biz knowledge is gated by domain.
    #[test]
    fn tech_visible_globally_biz_gated_by_domain() {
        let dir = tempdir().unwrap();
        write_entry(
            dir.path(),
            "TK-A",
            "tech",
            None,
            "draft",
            &["IMPLEMENT"],
            BIG_BODY,
        );
        write_entry(
            dir.path(),
            "BK-A",
            "biz",
            Some("ecommerce"),
            "draft",
            &["IMPLEMENT"],
            BIG_BODY,
        );

        let no_domain = query(dir.path(), "IMPLEMENT", 1000, None);
        assert!(no_domain.injected_ids.contains(&"TK-A".to_string()));
        assert!(
            !no_domain.injected_ids.contains(&"BK-A".to_string()),
            "biz entry must not surface without a matching domain"
        );

        let with_domain = query(dir.path(), "IMPLEMENT", 1000, Some("ecommerce"));
        assert!(with_domain.injected_ids.contains(&"TK-A".to_string()));
        assert!(with_domain.injected_ids.contains(&"BK-A".to_string()));
    }

    // R3b: empty KB -> fully empty result, no panic.
    #[test]
    fn empty_kb_returns_empty_result() {
        let dir = tempdir().unwrap();
        let r = query(dir.path(), "IMPLEMENT", 300, None);
        assert!(r.injected_ids.is_empty());
        assert!(r.dropped.is_empty());
        assert!(r.text.is_empty());
    }
}
