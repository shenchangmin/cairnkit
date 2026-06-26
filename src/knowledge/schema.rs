//! Frontmatter schema validation (M4) — invalid entries are rejected, not stored.

use crate::errors::{CairnError, Result};
use crate::knowledge::model::Entry;
use crate::knowledge::{CATEGORIES, KNOWLEDGE_CLASSES, LAYERS, MATURITIES, POLARITIES, TYPES};

pub fn iter_errors(entry: &Entry) -> Vec<String> {
    let mut e = Vec::new();
    if entry.id.is_empty() {
        e.push("missing id".to_string());
    }
    if entry.title.is_empty() {
        e.push("missing title".to_string());
    }
    if !CATEGORIES.contains(&entry.category.as_str()) {
        e.push(format!("category must be one of {CATEGORIES:?}, got {:?}", entry.category));
    }
    if !TYPES.contains(&entry.kind.as_str()) {
        e.push(format!("type must be one of {TYPES:?}, got {:?}", entry.kind));
    }
    if !MATURITIES.contains(&entry.maturity.as_str()) {
        e.push(format!("maturity must be one of {MATURITIES:?}, got {:?}", entry.maturity));
    }
    if !KNOWLEDGE_CLASSES.contains(&entry.knowledge_class.as_str()) {
        e.push(format!(
            "knowledge_class must be one of {KNOWLEDGE_CLASSES:?}, got {:?}",
            entry.knowledge_class
        ));
    }
    if !LAYERS.contains(&entry.layer.as_str()) {
        e.push(format!("layer must be one of {LAYERS:?}, got {:?}", entry.layer));
    }
    // cross-field rules
    if entry.category == "biz" && entry.domain.as_deref().unwrap_or("").is_empty() {
        e.push("biz knowledge requires a domain".to_string());
    }
    if entry.category == "tech" && entry.domain.as_deref().unwrap_or("").len() > 0 {
        e.push("tech knowledge must not set a domain".to_string());
    }
    if entry.kind == "guideline" {
        if !POLARITIES.contains(&entry.guideline_polarity.as_deref().unwrap_or("")) {
            e.push(format!("guideline requires guideline_polarity in {POLARITIES:?}"));
        }
    } else if entry.guideline_polarity.is_some() {
        e.push("guideline_polarity is only valid for type=guideline".to_string());
    }
    // id prefix convention (L3 project-local entries exempt)
    if !entry.id.is_empty() && entry.layer != "L3" {
        if entry.category == "tech" && !entry.id.starts_with("TK-") {
            e.push("tech entry id should start with 'TK-'".to_string());
        }
        if entry.category == "biz" && !entry.id.starts_with("BK-") {
            e.push("biz entry id should start with 'BK-'".to_string());
        }
    }
    e
}

pub fn validate(entry: &Entry) -> Result<()> {
    let errs = iter_errors(entry);
    if errs.is_empty() {
        Ok(())
    } else {
        let id = if entry.id.is_empty() { "<no id>" } else { &entry.id };
        Err(CairnError::Usage(format!("{id}: {}", errs.join("; "))))
    }
}
