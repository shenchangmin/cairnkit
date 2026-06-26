//! Notification dispatch (M10). Webhook via `curl` (no HTTP dependency); degrade to a local file.

use crate::config::Config;
use serde_json::{json, Value};
use std::process::Command;

const LOCAL_FILE: &str = ".cairnkit/notifications.log";

pub fn build_message(event: &str, project: &str, detail: &str) -> String {
    if detail.is_empty() {
        format!("[cairnkit:{project}] {event}")
    } else {
        format!("[cairnkit:{project}] {event} — {detail}")
    }
}

pub fn notify(event: &str, config: &Config, detail: &str, channel: &str) -> Value {
    let text = build_message(event, &config.project, detail);
    let url = config
        .notify_webhook_env
        .as_ref()
        .and_then(|name| std::env::var(name).ok());
    match url {
        None => local(config, &text, "no webhook configured"),
        Some(u) => match send_feishu(&u, &text) {
            Ok(()) => json!({"sent": true, "channel": channel}),
            Err(e) => local(config, &text, &format!("send failed: {e}")),
        },
    }
}

fn send_feishu(url: &str, text: &str) -> Result<(), String> {
    let payload = json!({"msg_type": "text", "content": {"text": text}}).to_string();
    let out = Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload,
            url,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn local(config: &Config, text: &str, reason: &str) -> Value {
    use std::io::Write;
    let path = config.root.join(LOCAL_FILE);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{text}");
    }
    let rel = path
        .strip_prefix(&config.root)
        .unwrap_or(&path)
        .to_string_lossy()
        .to_string();
    json!({"sent": false, "local": rel, "reason": reason})
}
