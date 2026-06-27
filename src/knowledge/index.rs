//! Three-level progressive index generation (M5).

use crate::errors::Result;
use crate::knowledge::model::{load_entry, Entry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CATALOG_A: &str = "knowledge-catalog.md";
pub const CATALOG_B: &str = "catalog.md";

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut paths: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            if p.is_dir() {
                collect_md(&p, out);
            } else if p.extension().map(|e| e == "md").unwrap_or(false)
                && p.file_name().map(|n| n != CATALOG_B).unwrap_or(true)
            {
                out.push(p);
            }
        }
    }
}

/// Load every entry under tech-wiki/ and biz-wiki/, skipping catalogs and unparseable files.
pub fn iter_entries(kb_root: &Path) -> Vec<Entry> {
    let mut paths = Vec::new();
    for wiki in ["tech-wiki", "biz-wiki"] {
        collect_md(&kb_root.join(wiki), &mut paths);
    }
    paths
        .iter()
        .filter_map(|p| load_entry(p).ok()) // a stray non-entry .md must not crash the scan
        .collect()
}

fn line(e: &Entry) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        e.id,
        e.title,
        e.maturity,
        e.knowledge_class,
        e.tags.join(",")
    )
}

fn write_catalog(path: &Path, title: &str, entries: &mut Vec<&Entry>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let mut lines = vec![
        format!("# {title}"),
        String::new(),
        "ID | title | maturity | class | tags".to_string(),
        "--- | --- | --- | --- | ---".to_string(),
    ];
    lines.extend(entries.iter().map(|e| line(e)));
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

pub fn build_index(kb_root: &Path) -> Result<BTreeMap<String, i64>> {
    std::fs::create_dir_all(kb_root)?;
    let entries = iter_entries(kb_root);

    let mut tech: Vec<&Entry> = entries.iter().filter(|e| e.category == "tech").collect();
    if !tech.is_empty() {
        write_catalog(
            &kb_root.join("tech-wiki").join(CATALOG_B),
            "Tech knowledge (L1)",
            &mut tech,
        )?;
    }
    let mut biz_by_domain: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for e in entries.iter().filter(|e| e.category == "biz") {
        biz_by_domain
            .entry(e.domain.clone().unwrap_or_else(|| "_".to_string()))
            .or_default()
            .push(e);
    }
    for (domain, mut items) in biz_by_domain.clone() {
        write_catalog(
            &kb_root.join("biz-wiki").join(&domain).join(CATALOG_B),
            &format!("Business knowledge — {domain} (L2)"),
            &mut items,
        )?;
    }

    write_panorama(&kb_root.join(CATALOG_A), &entries)?;

    let mut stats = BTreeMap::new();
    stats.insert("total".to_string(), entries.len() as i64);
    stats.insert(
        "tech".to_string(),
        entries.iter().filter(|e| e.category == "tech").count() as i64,
    );
    stats.insert(
        "biz".to_string(),
        entries.iter().filter(|e| e.category == "biz").count() as i64,
    );
    stats.insert("domains".to_string(), biz_by_domain.len() as i64);
    Ok(stats)
}

fn write_panorama(path: &Path, entries: &[Entry]) -> Result<()> {
    let count = |pred: &dyn Fn(&Entry) -> bool| entries.iter().filter(|e| pred(e)).count();
    let mat = |m: &str| count(&|e| e.maturity == m);
    let cls = |c: &str| count(&|e| e.knowledge_class == c);

    let mut stage_counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in entries {
        for ph in &e.applicable_phases {
            *stage_counts.entry(ph.clone()).or_insert(0) += 1;
        }
    }

    let mut lines = vec![
        "# Knowledge catalog (panorama)".to_string(),
        String::new(),
        format!(
            "- total: {}  ·  tech: {}  ·  biz: {}",
            entries.len(),
            count(&|e| e.category == "tech"),
            count(&|e| e.category == "biz")
        ),
        format!(
            "- maturity: draft={}, verified={}, proven={}",
            mat("draft"),
            mat("verified"),
            mat("proven")
        ),
        format!(
            "- class: point={}, causal={}, spatiotemporal={}",
            cls("point"),
            cls("causal"),
            cls("spatiotemporal")
        ),
        String::new(),
        "## Entries applicable per stage".to_string(),
        String::new(),
        "stage | count".to_string(),
        "--- | ---".to_string(),
    ];
    for (stage, n) in &stage_counts {
        lines.push(format!("{stage} | {n}"));
    }
    lines.push(String::new());
    lines.push("> Drill down: read a wiki `catalog.md` (B) for one-line entries, then the entry `.md` (C).".to_string());
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::{save_entry, Evidence};
    use tempfile::tempdir;

    fn write_entry(root: &Path, id: &str, category: &str, domain: Option<&str>) {
        let entry = Entry {
            id: id.to_string(),
            title: format!("title for {id}"),
            category: category.to_string(),
            domain: domain.map(String::from),
            kind: "decision".to_string(),
            guideline_polarity: None,
            maturity: "draft".to_string(),
            knowledge_class: "point".to_string(),
            layer: "L1".to_string(),
            tags: vec![],
            applicable_phases: vec![],
            evidence: Evidence::default(),
            history: vec![],
            body: "b".to_string(),
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

    // R4a: stats reflect total / tech / biz / distinct-domain counts.
    #[test]
    fn build_index_counts_total_tech_biz_domains() {
        let dir = tempdir().unwrap();
        write_entry(dir.path(), "TK-1", "tech", None);
        write_entry(dir.path(), "TK-2", "tech", None);
        write_entry(dir.path(), "BK-1", "biz", Some("ecommerce"));
        write_entry(dir.path(), "BK-2", "biz", Some("ads"));

        let stats = build_index(dir.path()).unwrap();

        assert_eq!(stats["total"], 4);
        assert_eq!(stats["tech"], 2);
        assert_eq!(stats["biz"], 2);
        assert_eq!(stats["domains"], 2);
    }

    // R4b: a null-domain biz entry is grouped under "_" (index lenient; schema strict).
    #[test]
    fn biz_null_domain_grouped_under_underscore() {
        let dir = tempdir().unwrap();
        write_entry(dir.path(), "BK-X", "biz", None);

        let stats = build_index(dir.path()).unwrap();

        assert!(stats["domains"] >= 1);
        assert!(dir
            .path()
            .join("biz-wiki")
            .join("_")
            .join(CATALOG_B)
            .exists());
    }

    // R4c: empty KB -> ok, all stats zero, no panic.
    #[test]
    fn empty_kb_does_not_panic_all_zero() {
        let dir = tempdir().unwrap();
        let stats = build_index(dir.path()).unwrap();
        assert_eq!(stats["total"], 0);
        assert_eq!(stats["tech"], 0);
        assert_eq!(stats["biz"], 0);
        assert_eq!(stats["domains"], 0);
    }
}
