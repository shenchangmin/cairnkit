//! Knowledge entry model + Markdown-with-frontmatter (de)serialization (M4).

use crate::errors::{usage, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(default)]
    pub contributors: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub last_referenced: Option<String>,
    #[serde(default)]
    pub ref_count: u64,
}

fn default_point() -> String {
    "point".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub guideline_polarity: Option<String>,
    #[serde(default)]
    pub maturity: String,
    #[serde(default = "default_point")]
    pub knowledge_class: String,
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub applicable_phases: Vec<String>,
    #[serde(default)]
    pub evidence: Evidence,
    #[serde(default)]
    pub history: Vec<serde_yaml::Value>,
    #[serde(skip)]
    pub body: String,
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

/// Split "---\n<frontmatter>\n---\n<body>" into (frontmatter, body).
fn split_frontmatter(text: &str) -> Result<(String, String)> {
    if !text.starts_with("---") {
        return Err(usage("entry missing leading '---' frontmatter block"));
    }
    let rest = &text[3..];
    let end = match rest.find("\n---") {
        Some(i) => i,
        None => return Err(usage("entry frontmatter is not terminated by '---'")),
    };
    let fm = &rest[..end];
    let after = &rest[end + 4..]; // skip "\n---"
    let body = match after.find('\n') {
        Some(nl) => &after[nl + 1..],
        None => "",
    };
    Ok((fm.to_string(), body.trim_matches('\n').to_string()))
}

pub fn parse_entry(text: &str, path: Option<PathBuf>) -> Result<Entry> {
    let (fm, body) = split_frontmatter(text)?;
    let mut entry: Entry = serde_yaml::from_str(&fm)
        .map_err(|e| usage(format!("entry frontmatter is not valid YAML: {e}")))?;
    // each history item must be a mapping (matches Python)
    if entry.history.iter().any(|h| !h.is_mapping()) {
        return Err(usage("each history item must be a mapping"));
    }
    if entry.knowledge_class.is_empty() {
        entry.knowledge_class = "point".to_string();
    }
    entry.body = body;
    entry.path = path;
    Ok(entry)
}

pub fn load_entry(path: &Path) -> Result<Entry> {
    let text = std::fs::read_to_string(path)?;
    parse_entry(&text, Some(path.to_path_buf()))
}

pub fn serialize_entry(entry: &Entry) -> String {
    // serde_yaml writes the struct fields in declaration order (skip body/path)
    let fm = serde_yaml::to_string(entry).unwrap_or_default();
    format!("---\n{fm}---\n\n{}\n", entry.body.trim_matches('\n'))
}

pub fn save_entry(path: &Path, entry: &Entry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serialize_entry(entry))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        "---\nid: TK-001\ntitle: Pagination\ncategory: tech\ndomain: null\ntype: decision\n\
         guideline_polarity: null\nmaturity: draft\nknowledge_class: causal\nlayer: L1\n\
         tags: [mysql]\napplicable_phases: [ARCHITECT_BACKEND]\n\
         evidence:\n  contributors: [you]\n  sources: []\n  projects: []\n  last_referenced: null\n  ref_count: 0\n\
         history: []\n---\nBody line.\n- bullet"
    }

    #[test]
    fn roundtrip_preserves_body_with_dash() {
        let e = parse_entry(sample(), None).unwrap();
        assert_eq!(e.id, "TK-001");
        assert_eq!(e.kind, "decision");
        assert!(e.body.contains("- bullet"));
        let again = parse_entry(&serialize_entry(&e), None).unwrap();
        assert_eq!(again.body, e.body);
        assert_eq!(again.id, "TK-001");
    }

    #[test]
    fn missing_frontmatter_errs() {
        assert!(parse_entry("no frontmatter", None).is_err());
    }

    #[test]
    fn knowledge_class_defaults_point() {
        let t = "---\nid: TK-9\ntitle: t\ncategory: tech\ntype: decision\nmaturity: draft\nlayer: L1\n---\nbody";
        assert_eq!(parse_entry(t, None).unwrap().knowledge_class, "point");
    }
}
