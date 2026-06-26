//! The knowledge layer — cairnkit's moat (mirrors Python `cairnkit.knowledge`).

pub mod extract_gate;
pub mod index;
pub mod kbrepo;
pub mod lifecycle;
pub mod lint;
pub mod model;
pub mod query;
pub mod refs;
pub mod schema;

pub const CATEGORIES: &[&str] = &["tech", "biz"];
pub const TYPES: &[&str] = &["model", "decision", "guideline", "pitfall", "process"];
pub const POLARITIES: &[&str] = &["recommend", "avoid"];
pub const MATURITIES: &[&str] = &["draft", "verified", "proven"];
pub const KNOWLEDGE_CLASSES: &[&str] = &["point", "causal", "spatiotemporal"];
pub const LAYERS: &[&str] = &["L0-P", "L0-T", "L1", "L2", "L3"];

pub fn maturity_rank(m: &str) -> i32 {
    match m {
        "proven" => 2,
        "verified" => 1,
        _ => 0,
    }
}
pub fn class_rank(c: &str) -> i32 {
    match c {
        "spatiotemporal" => 2,
        "causal" => 1,
        _ => 0,
    }
}
