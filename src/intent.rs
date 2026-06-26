//! IntentGate heuristic routing (mirrors Python `cairnkit.intent`).

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
const FRONTEND_HINTS: &[&str] = &[
    "ui",
    "page",
    "screen",
    "component",
    "css",
    "frontend",
    "front-end",
    "visual",
    "button",
    "layout",
    "style",
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
    if !FRONTEND_HINTS.iter().any(|h| low.contains(h)) {
        return IntentResult {
            path_mode: "lite",
            reason: "no frontend/UI surface detected — backend-only path",
        };
    }
    IntentResult {
        path_mode: "full",
        reason: "frontend surface present — full pipeline",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes() {
        assert_eq!(classify("fix a typo in the README").path_mode, "single");
        assert_eq!(
            classify("add a coupon endpoint to the order service with rate limiting").path_mode,
            "lite"
        );
        assert_eq!(
            classify("build a new checkout page UI component with a styled button").path_mode,
            "full"
        );
    }
}
