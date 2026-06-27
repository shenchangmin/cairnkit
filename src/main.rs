//! cairn — the cairnkit deterministic core CLI (Rust, single binary, zero runtime deps).
//!
//! Query subcommands print JSON to stdout; mutations change files and return an exit code.
//! Codes: 0 ok · 2 usage/precondition · 3 admission-gate refusal · 4 STATE corrupt.

mod config;
mod errors;
mod evolve;
mod gate;
mod import_state;
mod intent;
mod knowledge;
mod notify;
mod stages;
mod state;

use clap::{Parser, Subcommand};
use errors::{usage, Result};
use knowledge::{extract_gate, index, kbrepo, lifecycle, lint, model, query, refs, schema};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "cairn", version, about = "cairnkit deterministic core")]
struct Cli {
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
    #[command(subcommand)]
    Kb(KbCmd),
    #[command(subcommand)]
    Lifecycle(LifecycleCmd),
    Lint {
        #[arg(long)]
        fix: bool,
    },
    #[command(subcommand)]
    Kbrepo(KbrepoCmd),
    #[command(subcommand)]
    Knowledge(KnowledgeCmd),
    Notify {
        #[arg(long)]
        event: String,
        #[arg(long)]
        detail: Option<String>,
        #[arg(long, default_value = "feishu")]
        channel: String,
    },
    #[command(subcommand)]
    Import(ImportCmd),
    #[command(subcommand)]
    Evolve(EvolveCmd),
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
    New {
        #[arg(long = "run-id")]
        run_id: String,
        #[arg(long)]
        force: bool,
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
#[derive(Subcommand)]
enum KbCmd {
    BuildIndex,
    Query {
        #[arg(long)]
        stage: String,
        #[arg(long, default_value_t = 300)]
        budget: i64,
        #[arg(long)]
        domain: Option<String>,
    },
    Validate {
        file: PathBuf,
    },
    Extract {
        #[arg(long = "from")]
        from: PathBuf,
    },
    Touch {
        #[arg(long = "from")]
        from: PathBuf,
    },
}
#[derive(Subcommand)]
enum LifecycleCmd {
    Promote,
    Decay,
}
#[derive(Subcommand)]
enum KbrepoCmd {
    Init,
    Pull,
    Push {
        #[arg(long)]
        message: String,
    },
    Promote {
        #[arg(long)]
        id: String,
        #[arg(long)]
        to: String,
    },
    StageConflict {
        #[arg(long)]
        id: String,
        #[arg(long)]
        file: PathBuf,
    },
}
#[derive(Subcommand)]
enum KnowledgeCmd {
    Stats,
}
#[derive(Subcommand)]
enum ImportCmd {
    Init,
    Show,
    Advance,
}
#[derive(Subcommand)]
enum EvolveCmd {
    Propose {
        #[arg(long)]
        id: String,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        content: Option<String>,
    },
    List {
        #[arg(long, default_value = "pending")]
        state: String,
    },
    Apply {
        #[arg(long)]
        id: String,
    },
    Reject {
        #[arg(long)]
        id: String,
    },
    Defer {
        #[arg(long)]
        id: String,
    },
}

fn emit(v: Value) {
    println!("{v}");
}
fn emit_state(s: &config::State) {
    println!("{}", serde_json::to_string(s).unwrap());
}

fn kbrepo_path(c: &config::Config) -> Result<PathBuf> {
    c.knowledge_repo_local.clone().ok_or_else(|| {
        usage("no knowledge_repo.local configured in cairnkit.yaml — set it to a local clone of the shared knowledge repo.")
    })
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
        Top::State(sc) => return state_cmd(root, sc),
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
        Top::Kb(kc) => return kb_cmd(root, kc),
        Top::Lifecycle(lc) => {
            let c = config::load_config(root)?;
            match lc {
                LifecycleCmd::Promote => {
                    emit(json!({"promoted": lifecycle::promote_repo(&c.knowledge_root)}))
                }
                LifecycleCmd::Decay => {
                    emit(json!({"decayed": lifecycle::decay_repo(&c.knowledge_root)}))
                }
            }
        }
        Top::Lint { fix } => {
            let c = config::load_config(root)?;
            let r = lint::lint(&c.knowledge_root, fix, None)?;
            emit(json!({
                "clean": r.clean(), "orphans": r.orphans, "stale": r.stale,
                "duplicates": r.duplicates, "invalid": r.invalid, "conflicts": r.conflicts, "fixed": r.fixed,
            }));
        }
        Top::Kbrepo(kc) => return kbrepo_cmd(root, kc),
        Top::Knowledge(KnowledgeCmd::Stats) => {
            let c = config::load_config(root)?;
            let repo = c
                .knowledge_repo_local
                .clone()
                .unwrap_or_else(|| c.knowledge_root.clone());
            let mut v = kbrepo::stats(&repo);
            if c.knowledge_repo_local.is_none() {
                v["warning"] =
                    json!("no shared knowledge_repo configured — scanning local knowledge_root");
            }
            emit(v);
        }
        Top::Notify {
            event,
            detail,
            channel,
        } => {
            let c = config::load_config(root)?;
            emit(notify::notify(
                &event,
                &c,
                detail.as_deref().unwrap_or(""),
                &channel,
            ));
        }
        Top::Import(ic) => {
            let c = config::load_config(root)?;
            match ic {
                ImportCmd::Init => emit(import_state::init_import(&c.root)?),
                ImportCmd::Show => emit(import_state::load_import(&c.root)?),
                ImportCmd::Advance => emit(import_state::advance_import(&c.root)?),
            }
        }
        Top::Evolve(ec) => return evolve_cmd(root, ec),
    }
    Ok(0)
}

fn state_cmd(root: &Path, sc: StateCmd) -> Result<i32> {
    let c = config::load_config(root)?;
    let sp = c.state_path();
    match sc {
        StateCmd::Init { run_id } => {
            if sp.exists() {
                return Err(usage(
                    "A run already exists (.cairnkit/STATE.yaml). Resume with /flow-run or remove the file to start over.",
                ));
            }
            emit_state(&config::init_state(&c, &run_id)?);
        }
        StateCmd::New { run_id, force } => {
            let (s, archived) = state::new_run(&sp, &c, &run_id, force)?;
            if let Some(p) = archived {
                eprintln!("archived previous STATE -> {}", p.display());
            }
            emit_state(&s);
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
    Ok(0)
}

fn kb_cmd(root: &Path, kc: KbCmd) -> Result<i32> {
    let c = config::load_config(root)?;
    match kc {
        KbCmd::BuildIndex => {
            let stats = index::build_index(&c.knowledge_root)?;
            emit(serde_json::to_value(stats).unwrap());
        }
        KbCmd::Query {
            stage,
            budget,
            domain,
        } => {
            let dom = domain.or_else(|| c.domain.clone());
            let r = query::query(&c.knowledge_root, &stage, budget, dom.as_deref());
            emit(json!({
                "stage": r.stage, "budget_lines": r.budget_lines, "lines": r.lines,
                "over_budget": r.over_budget, "injected_ids": r.injected_ids,
                "dropped": r.dropped, "text": r.text,
            }));
        }
        KbCmd::Validate { file } => {
            schema::validate(&model::load_entry(&file)?)?;
            emit(json!({"ok": true, "file": file.to_string_lossy()}));
        }
        KbCmd::Extract { from } => emit(extract_gate::extract_from_run(&from, &c.knowledge_root)),
        KbCmd::Touch { from } => emit(refs::touch(&c.knowledge_root, &from, &c.project, None)),
    }
    Ok(0)
}

fn kbrepo_cmd(root: &Path, kc: KbrepoCmd) -> Result<i32> {
    let c = config::load_config(root)?;
    let repo = kbrepo_path(&c)?;
    match kc {
        KbrepoCmd::Init => {
            kbrepo::init_repo(&repo)?;
            emit(
                json!({"initialized": repo.to_string_lossy(), "is_git_repo": kbrepo::is_git_repo(&repo)}),
            );
        }
        KbrepoCmd::Pull => emit(kbrepo::pull(&repo)),
        KbrepoCmd::Push { message } => emit(kbrepo::push(&repo, &message)?),
        KbrepoCmd::Promote { id, to } => {
            let dest = kbrepo::promote_entry(&repo, &id, &to)?;
            emit(
                json!({"promoted": id, "to": to, "path": dest.strip_prefix(&repo).unwrap_or(&dest).to_string_lossy()}),
            );
        }
        KbrepoCmd::StageConflict { id, file } => {
            let body = std::fs::read_to_string(&file)?;
            let path = kbrepo::stage_conflict(&repo, &id, &body, None)?;
            emit(json!({"staged": path.strip_prefix(&repo).unwrap_or(&path).to_string_lossy()}));
        }
    }
    Ok(0)
}

fn evolve_cmd(root: &Path, ec: EvolveCmd) -> Result<i32> {
    let c = config::load_config(root)?;
    match ec {
        EvolveCmd::Propose { id, file, content } => {
            let body = match file {
                Some(p) => std::fs::read_to_string(p)?,
                None => content.unwrap_or_default(),
            };
            let path = evolve::propose(&c.root, &id, &body)?;
            emit(
                json!({"proposed": id, "path": path.strip_prefix(&c.root).unwrap_or(&path).to_string_lossy()}),
            );
        }
        EvolveCmd::List { state } => {
            emit(json!({"state": state, "proposals": evolve::list_proposals(&c.root, &state)?}));
        }
        EvolveCmd::Apply { id } => return evolve_transition(&c.root, &id, "applied"),
        EvolveCmd::Reject { id } => return evolve_transition(&c.root, &id, "rejected"),
        EvolveCmd::Defer { id } => return evolve_transition(&c.root, &id, "deferred"),
    }
    Ok(0)
}

fn evolve_transition(root: &Path, id: &str, to: &str) -> Result<i32> {
    let path = evolve::transition(root, id, to)?;
    emit(json!({id: to, "path": path.strip_prefix(root).unwrap_or(&path).to_string_lossy()}));
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
