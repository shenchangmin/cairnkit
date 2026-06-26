//! Knowledge maturity lifecycle (M6): promotion, decay, layer judging.

use crate::knowledge::index::iter_entries;
use crate::knowledge::model::{save_entry, Entry};
use chrono::{Local, NaiveDate};

const PROVEN_DECAY_MONTHS: f64 = 12.0;
const VERIFIED_DECAY_MONTHS: f64 = 6.0;

pub fn months_between(earlier_iso: &str, now: NaiveDate) -> f64 {
    let parts: Vec<i32> = earlier_iso
        .split('-')
        .take(3)
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() < 3 {
        return 0.0;
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    use chrono::Datelike;
    (now.year() - y) as f64 * 12.0
        + (now.month() as i32 - m) as f64
        + (now.day() as i32 - d) as f64 / 30.0
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn with_maturity(entry: &Entry, maturity: &str, now: NaiveDate, why: &str) -> Entry {
    let mut e = entry.clone();
    e.maturity = maturity.to_string();
    let mut map = serde_yaml::Mapping::new();
    map.insert("date".into(), now.to_string().into());
    map.insert("update_type".into(), why.into());
    map.insert("by".into(), "system".into());
    e.history.push(serde_yaml::Value::Mapping(map));
    e
}

pub fn promote(entry: &Entry, now: Option<NaiveDate>) -> Entry {
    let now = now.unwrap_or_else(today);
    if entry.maturity == "draft" && entry.evidence.ref_count >= 1 {
        return with_maturity(entry, "verified", now, "promote");
    }
    if entry.maturity == "verified" && entry.evidence.projects.len() >= 2 {
        return with_maturity(entry, "proven", now, "promote");
    }
    entry.clone()
}

pub fn decay(entry: &Entry, now: Option<NaiveDate>) -> Entry {
    let now = now.unwrap_or_else(today);
    let last = match &entry.evidence.last_referenced {
        Some(s) if !s.is_empty() => s,
        _ => return entry.clone(),
    };
    let age = months_between(last, now);
    if entry.maturity == "proven" && age >= PROVEN_DECAY_MONTHS {
        return with_maturity(entry, "verified", now, "decay");
    }
    if entry.maturity == "verified" && age >= VERIFIED_DECAY_MONTHS {
        return with_maturity(entry, "draft", now, "decay");
    }
    entry.clone()
}

#[allow(dead_code)] // promotion-layer suggestion API (tested; advisory)
pub fn judge_layer(entry: &Entry) -> &'static str {
    if entry.evidence.projects.len() <= 1 {
        "L3"
    } else if entry.category == "tech" {
        "L1"
    } else {
        "L2"
    }
}

fn apply_repo(kb_root: &std::path::Path, f: impl Fn(&Entry) -> Entry) -> Vec<String> {
    let mut changed = Vec::new();
    for entry in iter_entries(kb_root) {
        let new = f(&entry);
        if new.maturity != entry.maturity {
            if let Some(p) = &entry.path {
                if save_entry(p, &new).is_ok() {
                    changed.push(entry.id.clone());
                }
            }
        }
    }
    changed
}

pub fn promote_repo(kb_root: &std::path::Path) -> Vec<String> {
    apply_repo(kb_root, |e| promote(e, None))
}
pub fn decay_repo(kb_root: &std::path::Path) -> Vec<String> {
    apply_repo(kb_root, |e| decay(e, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::{Entry, Evidence};

    fn entry(maturity: &str, ev: Evidence) -> Entry {
        Entry {
            id: "TK-1".into(),
            title: "t".into(),
            category: "tech".into(),
            domain: None,
            kind: "decision".into(),
            guideline_polarity: None,
            maturity: maturity.into(),
            knowledge_class: "point".into(),
            layer: "L3".into(),
            tags: vec![],
            applicable_phases: vec![],
            evidence: ev,
            history: vec![],
            body: "b".into(),
            path: None,
        }
    }

    #[test]
    fn promote_rules() {
        let e = entry(
            "draft",
            Evidence {
                ref_count: 1,
                ..Default::default()
            },
        );
        assert_eq!(promote(&e, None).maturity, "verified");
        let e = entry(
            "verified",
            Evidence {
                projects: vec!["a".into(), "b".into()],
                ..Default::default()
            },
        );
        assert_eq!(promote(&e, None).maturity, "proven");
        let e = entry("draft", Evidence::default());
        assert_eq!(promote(&e, None).maturity, "draft");
    }

    #[test]
    fn decay_rules() {
        let now = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let e = entry(
            "proven",
            Evidence {
                last_referenced: Some("2025-01-01".into()),
                ref_count: 5,
                ..Default::default()
            },
        );
        assert_eq!(decay(&e, Some(now)).maturity, "verified");
        let e = entry(
            "proven",
            Evidence {
                last_referenced: None,
                ..Default::default()
            },
        );
        assert_eq!(decay(&e, Some(now)).maturity, "proven");
    }

    #[test]
    fn judge() {
        assert_eq!(
            judge_layer(&entry(
                "draft",
                Evidence {
                    projects: vec!["only".into()],
                    ..Default::default()
                }
            )),
            "L3"
        );
        let mut e = entry(
            "verified",
            Evidence {
                projects: vec!["a".into(), "b".into()],
                ..Default::default()
            },
        );
        assert_eq!(judge_layer(&e), "L1");
        e.category = "biz".into();
        e.domain = Some("ads".into());
        assert_eq!(judge_layer(&e), "L2");
    }
}
