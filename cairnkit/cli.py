"""The cairnkit command-line surface — the deterministic API the Markdown shell calls.

Two kinds of subcommand (CLAUDE.md §5):
  - queries (show / resume / gate check) print JSON to stdout, return 0
  - mutations (init / advance / set-stage / approve-clarify) change files and return a code

Return codes: 0 ok · 2 usage · 3 admission-gate refusal · 4 STATE corrupt.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Mapping

from cairnkit import state as sm
from cairnkit.config import State, init_state, load_config, load_state
from cairnkit import gate
from cairnkit.errors import CairnkitError, UsageError
from cairnkit.knowledge.index import build_index
from cairnkit.knowledge.model import load_entry
from cairnkit.knowledge.query import query as kb_query
from cairnkit.knowledge.schema import validate as kb_validate


def _state_dict(state: State) -> dict:
    return {
        "run_id": state.run_id,
        "stage": state.stage,
        "path_mode": state.path_mode,
        "history": list(state.history),
        "artifacts": dict(state.artifacts),
        "retries": dict(state.retries),
        "pending_clarify": state.pending_clarify,
        "updated_at": state.updated_at,
    }


def _emit(obj: Mapping) -> None:
    print(json.dumps(obj))


# --- command handlers ------------------------------------------------------

def _cmd_state_init(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    if config.state_path.exists():
        raise UsageError(
            "A run already exists (.cairnkit/STATE.yaml). Resume it with /flow-run "
            "instead of re-initialising, or remove the file to start over."
        )
    state = init_state(config, args.run_id)
    _emit(_state_dict(state))
    return 0


def _cmd_config_show(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit({
        "project": config.project,
        "domain": config.domain,
        "repos": [{"name": r.name, "path": r.path} for r in config.repos],
        "has_run": config.state_path.exists(),
    })
    return 0


def _cmd_state_show(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit(_state_dict(sm.show(config.state_path)))
    return 0


def _cmd_state_resume(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    state = sm.resume(config.state_path)
    _emit({"stage": state.stage, "paused": sm.is_paused(state)})
    return 0


def _cmd_state_advance(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit(_state_dict(sm.advance(config.state_path, config)))
    return 0


def _cmd_state_set_stage(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit(_state_dict(sm.set_stage(config.state_path, args.stage, config)))
    return 0


def _cmd_state_approve_clarify(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit(_state_dict(sm.approve_clarify(config.state_path)))
    return 0


def _cmd_kb_build_index(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    _emit(build_index(config.knowledge_root))
    return 0


def _cmd_kb_query(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    domain = args.domain if args.domain is not None else config.domain
    res = kb_query(config.knowledge_root, args.stage, args.budget, domain)
    _emit({
        "stage": res.stage,
        "budget_lines": res.budget_lines,
        "lines": res.lines,
        "injected_ids": list(res.injected_ids),
        "dropped": list(res.dropped),
        "text": res.text,
    })
    return 0


def _cmd_kb_validate(args: argparse.Namespace) -> int:
    kb_validate(load_entry(Path(args.file)))
    _emit({"ok": True, "file": args.file})
    return 0


def _cmd_gate_check(args: argparse.Namespace) -> int:
    config = load_config(args.root)
    state = load_state(config.state_path)
    result = gate.check(args.stage, state, config)
    _emit({
        "ok": result.ok,
        "stage": result.stage,
        "missing": list(result.missing),
        "message": result.message,
    })
    return 0 if result.ok else 3


# --- parser ----------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="cairnkit", description="cairnkit core CLI")
    parser.add_argument(
        "--root", type=Path, default=Path.cwd(),
        help="host project root (default: cwd)",
    )
    sub = parser.add_subparsers(dest="group", required=True)

    config_p = sub.add_parser("config", help="project config")
    config_sub = config_p.add_subparsers(dest="cmd", required=True)
    config_sub.add_parser("show", help="validate & print cairnkit.yaml").set_defaults(func=_cmd_config_show)

    state_p = sub.add_parser("state", help="state machine")
    state_sub = state_p.add_subparsers(dest="cmd", required=True)

    p = state_sub.add_parser("init", help="create a fresh run")
    p.add_argument("--run-id", required=True)
    p.set_defaults(func=_cmd_state_init)

    state_sub.add_parser("show", help="print current state").set_defaults(func=_cmd_state_show)
    state_sub.add_parser("resume", help="state to continue from").set_defaults(func=_cmd_state_resume)
    state_sub.add_parser("advance", help="advance to the next stage").set_defaults(func=_cmd_state_advance)
    state_sub.add_parser("approve-clarify", help="clear a CLARIFY pause").set_defaults(func=_cmd_state_approve_clarify)

    p = state_sub.add_parser("set-stage", help="force a stage (repair)")
    p.add_argument("stage")
    p.set_defaults(func=_cmd_state_set_stage)

    kb_p = sub.add_parser("kb", help="knowledge base")
    kb_sub = kb_p.add_subparsers(dest="cmd", required=True)
    kb_sub.add_parser("build-index", help="(re)generate the 3-level index").set_defaults(func=_cmd_kb_build_index)
    p = kb_sub.add_parser("query", help="budget-bounded knowledge injection for a stage")
    p.add_argument("--stage", required=True)
    p.add_argument("--budget", type=int, default=300)
    p.add_argument("--domain", default=None)
    p.set_defaults(func=_cmd_kb_query)
    p = kb_sub.add_parser("validate", help="schema-validate an entry file")
    p.add_argument("file")
    p.set_defaults(func=_cmd_kb_validate)

    gate_p = sub.add_parser("gate", help="admission gate")
    gate_sub = gate_p.add_subparsers(dest="cmd", required=True)
    p = gate_sub.add_parser("check", help="check entry preconditions for a stage")
    p.add_argument("--stage", required=True)
    p.set_defaults(func=_cmd_gate_check)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)  # argparse exits 2 on usage error
    try:
        return args.func(args)
    except CairnkitError as exc:
        print(json.dumps({"error": str(exc), "code": exc.code}), file=sys.stderr)
        return exc.code
    except OSError as exc:  # filesystem issues (e.g. missing knowledge_root) -> usage code
        print(json.dumps({"error": str(exc), "code": 2}), file=sys.stderr)
        return 2
