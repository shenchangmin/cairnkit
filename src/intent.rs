//! IntentGate fallback default for headless/scripted use.
//!
//! Classification is fuzzy work, so the **orchestrator (the model) is the primary
//! classifier** — it reads the request and picks `full`/`lite`/`single` by judgment in
//! any language. This Rust heuristic is only a deterministic fallback default for headless
//! or scripted runs where no model is in the loop. When unsure it defaults to `full`, so it
//! never under-processes a request.

const SINGLE_HINTS: &[&str] = &[
    "typo",
    "rename",
    "bump",
    "comment",
    "log message",
    "one line",
    "one-line",
    "tweak",
    "constant",
    "config value",
    "version",
];

pub struct IntentResult {
    pub path_mode: &'static str,
    pub reason: &'static str,
}

pub fn classify(text: &str) -> IntentResult {
    let low = text.to_lowercase();
    let words = low.split_whitespace().count();

    if words <= 12 && SINGLE_HINTS.iter().any(|h| low.contains(h)) {
        return IntentResult {
            path_mode: "single",
            reason: "short single-point change (keyword + brevity)",
        };
    }
    IntentResult {
        path_mode: "full",
        reason: "default to full pipeline; the orchestrator refines to lite/single by judgment",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes() {
        // Short single-point change → single.
        assert_eq!(classify("fix a typo in the README").path_mode, "single");
        // The heuristic no longer guesses lite — backend and frontend both default to full.
        assert_eq!(
            classify("add a coupon endpoint to the order service with rate limiting").path_mode,
            "full"
        );
        assert_eq!(
            classify("build a new checkout page UI component with a styled button").path_mode,
            "full"
        );
    }
}
