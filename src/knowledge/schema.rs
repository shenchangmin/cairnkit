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
        e.push(format!(
            "category must be one of {CATEGORIES:?}, got {:?}",
            entry.category
        ));
    }
    if !TYPES.contains(&entry.kind.as_str()) {
        e.push(format!(
            "type must be one of {TYPES:?}, got {:?}",
            entry.kind
        ));
    }
    if !MATURITIES.contains(&entry.maturity.as_str()) {
        e.push(format!(
            "maturity must be one of {MATURITIES:?}, got {:?}",
            entry.maturity
        ));
    }
    if !KNOWLEDGE_CLASSES.contains(&entry.knowledge_class.as_str()) {
        e.push(format!(
            "knowledge_class must be one of {KNOWLEDGE_CLASSES:?}, got {:?}",
            entry.knowledge_class
        ));
    }
    if !LAYERS.contains(&entry.layer.as_str()) {
        e.push(format!(
            "layer must be one of {LAYERS:?}, got {:?}",
            entry.layer
        ));
    }
    // cross-field rules
    if entry.category == "biz" && entry.domain.as_deref().unwrap_or("").is_empty() {
        e.push("biz knowledge requires a domain".to_string());
    }
    if entry.category == "tech" && !entry.domain.as_deref().unwrap_or("").is_empty() {
        e.push("tech knowledge must not set a domain".to_string());
    }
    if entry.kind == "guideline" {
        if !POLARITIES.contains(&entry.guideline_polarity.as_deref().unwrap_or("")) {
            e.push(format!(
                "guideline requires guideline_polarity in {POLARITIES:?}"
            ));
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
        let id = if entry.id.is_empty() {
            "<no id>"
        } else {
            &entry.id
        };
        Err(CairnError::Usage(format!("{id}: {}", errs.join("; "))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::Evidence;

    fn valid_tech() -> Entry {
        Entry {
            id: "TK-1".to_string(),
            title: "tech title".to_string(),
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
            body: "b".to_string(),
            path: None,
        }
    }

    fn valid_biz() -> Entry {
        Entry {
            id: "BK-1".to_string(),
            category: "biz".to_string(),
            domain: Some("ecommerce".to_string()),
            ..valid_tech()
        }
    }

    // R9a: both a valid tech and a valid biz entry pass.
    #[test]
    fn accepts_valid_tech_and_biz_entries() {
        for e in [valid_tech(), valid_biz()] {
            assert!(
                iter_errors(&e).is_empty(),
                "unexpected errors: {:?}",
                iter_errors(&e)
            );
            assert!(validate(&e).is_ok());
        }
    }

    // R9b: each cross-field / enum / required-field defect produces its error fragment.
    #[test]
    fn rejects_cross_field_and_enum_violations() {
        let has = |e: &Entry, frag: &str| iter_errors(e).iter().any(|m| m.contains(frag));

        // tech with a domain
        let mut e = valid_tech();
        e.domain = Some("ecommerce".to_string());
        assert!(has(&e, "tech knowledge must not set a domain"));

        // biz without a domain
        let mut e = valid_biz();
        e.domain = None;
        assert!(has(&e, "biz knowledge requires a domain"));

        // bad category (layer L3 so the id-prefix rule does not also fire)
        let mut e = valid_tech();
        e.category = "bogus".to_string();
        e.layer = "L3".to_string();
        assert!(has(&e, "must be one of"));

        // bad kind/type
        let mut e = valid_tech();
        e.kind = "bogus".to_string();
        assert!(has(&e, "must be one of"));

        // bad maturity
        let mut e = valid_tech();
        e.maturity = "bogus".to_string();
        assert!(has(&e, "must be one of"));

        // bad knowledge_class
        let mut e = valid_tech();
        e.knowledge_class = "bogus".to_string();
        assert!(has(&e, "must be one of"));

        // bad layer
        let mut e = valid_tech();
        e.layer = "bogus".to_string();
        assert!(has(&e, "must be one of"));

        // empty id
        let mut e = valid_tech();
        e.id = String::new();
        assert!(has(&e, "missing id"));

        // empty title
        let mut e = valid_tech();
        e.title = String::new();
        assert!(has(&e, "missing title"));

        // polarity set on a non-guideline
        let mut e = valid_tech();
        e.guideline_polarity = Some("recommend".to_string());
        assert!(has(
            &e,
            "guideline_polarity is only valid for type=guideline"
        ));
    }
}
