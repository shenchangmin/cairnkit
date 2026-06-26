//! cairn — the cairnkit deterministic core CLI (Rust, single binary).
//!
//! Query subcommands print JSON to stdout; mutations change files and return an exit code.
//! Codes: 0 ok · 2 usage/precondition · 3 admission-gate refusal · 4 STATE corrupt.

mod config;
mod errors;
mod gate;
mod intent;
mod stages;
mod state;

use clap::{Parser, Subcommand};
use errors::{usage, CairnError, Result};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "cairn", version, about = "cairnkit deterministic core")]
struct Cli {
    /// Host project root (default: cwd)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    cmd: Top,
}

#[derive(Subcommand)]
enum Top {
    #[command(subcommand)]
    Config(ConfigCmd),
    #[command(subcommand)]
    State(StateCmd),
    #[command(subcommand)]
    Gate(GateCmd),
    #[command(subcommand)]
    Intent(IntentCmd),
}

#[derive(Subcommand)]
enum ConfigCmd {
    Show,
}

#[derive(Subcommand)]
enum StateCmd {
    Init {
        #[arg(long = "run-id")]
        run_id: String,
    },
    Show,
    Resume,
    Advance,
    SetStage {
        stage: String,
    },
    SetPathMode {
        mode: String,
    },
    ApproveClarify,
    Fail {
        #[arg(long)]
        stage: String,
    },
    Unblock,
}

#[derive(Subcommand)]
enum GateCmd {
    Check {
        #[arg(long)]
        stage: String,
    },
}

#[derive(Subcommand)]
enum IntentCmd {
    Classify {
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        input: Option<PathBuf>,
    },
}

fn emit(v: serde_json::Value) {
    println!("{v}");
}
fn emit_state(s: &config::State) {
    println!("{}", serde_json::to_string(s).unwrap());
}

fn run(cli: Cli) -> Result<i32> {
    let root = cli.root.as_path();
    match cli.cmd {
        Top::Config(ConfigCmd::Show) => {
            let c = config::load_config(root)?;
            emit(json!({
                "project": c.project,
                "domain": c.domain,
                "repos": c.repos.iter().map(|r| json!({"name": r.name, "path": r.path})).collect::<Vec<_>>(),
                "has_run": c.state_path().exists(),
            }));
        }
        Top::State(sc) => {
            let c = config::load_config(root)?;
            let sp = c.state_path();
            match sc {
                StateCmd::Init { run_id } => {
                    if sp.exists() {
                        return Err(usage(
                            "A run already exists (.cairnkit/STATE.yaml). Resume with /flow-run \
                             or remove the file to start over.",
                        ));
                    }
                    emit_state(&config::init_state(&c, &run_id)?);
                }
                StateCmd::Show => emit_state(&state::show(&sp)?),
                StateCmd::Resume => {
                    let s = state::resume(&sp)?;
                    emit(json!({"stage": s.stage, "paused": state::is_paused(&s)}));
                }
                StateCmd::Advance => emit_state(&state::advance(&sp, &c)?),
                StateCmd::SetStage { stage } => emit_state(&state::set_stage(&sp, &stage)?),
                StateCmd::SetPathMode { mode } => emit_state(&state::set_path_mode(&sp, &mode)?),
                StateCmd::ApproveClarify => emit_state(&state::approve_clarify(&sp)?),
                StateCmd::Fail { stage } => emit_state(&state::record_failure(&sp, &stage)?),
                StateCmd::Unblock => emit_state(&state::unblock(&sp)?),
            }
        }
        Top::Gate(GateCmd::Check { stage }) => {
            let c = config::load_config(root)?;
            let s = config::load_state(&c.state_path())?;
            let r = gate::check(&stage, &s, &c);
            emit(json!({"ok": r.ok, "stage": r.stage, "missing": r.missing, "message": r.message}));
            return Ok(if r.ok { 0 } else { 3 });
        }
        Top::Intent(IntentCmd::Classify { text, input }) => {
            let body = match input {
                Some(p) => std::fs::read_to_string(p)?,
                None => text.unwrap_or_default(),
            };
            let r = intent::classify(&body);
            emit(json!({"path_mode": r.path_mode, "reason": r.reason}));
        }
    }
    Ok(0)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("{}", json!({"error": e.message(), "code": e.code()}));
            ExitCode::from(e.code() as u8)
        }
    }
}

#[allow(dead_code)]
fn _silence(_: CairnError) {}
